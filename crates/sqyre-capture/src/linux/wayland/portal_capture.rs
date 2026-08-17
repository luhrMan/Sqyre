//! Wayland screen capture via XDG portal ScreenCast + PipeWire.

use crate::cap_log;
use crate::error::CaptureError;
use crate::linux::session::{LinuxCaptureBackend, LinuxSessionInfo};
use image::RgbaImage;
use parking_lot::{Condvar, Mutex};
use pipewire as pw;
use pw::context::ContextRc;
use pw::main_loop::MainLoopRc;
use pw::properties::properties;
use pw::spa::param::video::{VideoFormat, VideoInfoRaw};
use pw::spa::pod::Pod;
use pw::stream::StreamRc;
use sqyre_ports::{DesktopRect, RgbCapture};
use std::os::fd::OwnedFd;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, Sender, SyncSender};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

/// Portal + PipeWire capturer for Wayland sessions.
pub struct PortalCapturer {
    frame: Arc<(Mutex<FrameSlot>, Condvar)>,
    shutdown: Arc<AtomicBool>,
    quit_tx: Mutex<Option<Sender<PwThreadMsg>>>,
    thread: Mutex<Option<JoinHandle<()>>>,
}

struct FrameSlot {
    cache: FrameCache,
    /// Incremented on every PipeWire frame copied into `cache`.
    generation: u64,
}

struct FrameCache {
    virtual_bounds: DesktopRect,
    monitor_rects: Vec<DesktopRect>,
    width: u32,
    height: u32,
    stride: usize,
    pixels: Vec<u8>,
    ready: bool,
}

struct PwSetup {
    node_id: u32,
    fd: OwnedFd,
    virtual_bounds: DesktopRect,
    monitor_rects: Vec<DesktopRect>,
}

enum PwThreadMsg {
    Quit,
}

impl PortalCapturer {
    pub fn open() -> Result<Self, CaptureError> {
        let info = LinuxSessionInfo::detect();
        if info.capture_backend() != LinuxCaptureBackend::WaylandPortal {
            return Err(CaptureError::Message(
                "portal capture not selected for this session".into(),
            ));
        }

        let frame = Arc::new((
            Mutex::new(FrameSlot {
                cache: FrameCache {
                    virtual_bounds: DesktopRect::default(),
                    monitor_rects: Vec::new(),
                    width: 0,
                    height: 0,
                    stride: 0,
                    pixels: Vec::new(),
                    ready: false,
                },
                generation: 0,
            }),
            Condvar::new(),
        ));
        let shutdown = Arc::new(AtomicBool::new(false));
        let (ready_tx, ready_rx) = mpsc::sync_channel::<Result<(), CaptureError>>(1);
        let (msg_tx, msg_rx) = mpsc::channel::<PwThreadMsg>();

        let frame_thread = Arc::clone(&frame);
        let shutdown_thread = Arc::clone(&shutdown);
        let handle = thread::Builder::new()
            .name("sqyre-portal-pw".into())
            .spawn(move || {
                if let Err(e) = portal_pw_thread(frame_thread, shutdown_thread, ready_tx, msg_rx) {
                    cap_log("PORTAL", "fail", &format!("thread={e}"));
                }
            })
            .map_err(|e| CaptureError::Message(format!("portal thread spawn: {e}")))?;

        match ready_rx.recv_timeout(Duration::from_secs(120)) {
            Ok(Ok(())) => {}
            Ok(Err(e)) => {
                shutdown.store(true, Ordering::SeqCst);
                let _ = msg_tx.send(PwThreadMsg::Quit);
                let _ = handle.join();
                return Err(e);
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {
                shutdown.store(true, Ordering::SeqCst);
                let _ = msg_tx.send(PwThreadMsg::Quit);
                let _ = handle.join();
                return Err(CaptureError::Message(
                    "portal ScreenCast timed out (grant permission in the picker)".into(),
                ));
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                let _ = handle.join();
                return Err(CaptureError::Message(
                    "portal PipeWire thread exited before first frame".into(),
                ));
            }
        }

        let slot = frame.0.lock();
        cap_log(
            "PORTAL",
            "ok",
            &format!(
                "backend=portal+pipewire size={}x{}",
                slot.cache.width, slot.cache.height
            ),
        );
        drop(slot);

        Ok(Self {
            frame,
            shutdown,
            quit_tx: Mutex::new(Some(msg_tx)),
            thread: Mutex::new(Some(handle)),
        })
    }

    /// Capture after waiting for a new PipeWire frame (for manual refresh).
    pub fn capture_rect_fresh_ref(&self, rect: DesktopRect) -> Result<RgbaImage, CaptureError> {
        self.wait_for_fresh_frame(Duration::from_secs(1))?;
        self.capture_rect_ref(rect)
    }

    pub fn capture_rect_ref(&self, rect: DesktopRect) -> Result<RgbaImage, CaptureError> {
        if rect.is_empty() {
            return Err(CaptureError::EmptyRect);
        }
        let slot = self.frame.0.lock();
        let cache = &slot.cache;
        if !cache.ready {
            return Err(CaptureError::Message(
                "portal capture: no frame yet from PipeWire".into(),
            ));
        }

        let vb = cache.virtual_bounds;
        let left = rect.x.max(vb.x);
        let top = rect.y.max(vb.y);
        let right = (rect.x + rect.w).min(vb.x + vb.w);
        let bottom = (rect.y + rect.h).min(vb.y + vb.h);
        if right <= left || bottom <= top {
            return Err(CaptureError::OutsideVirtualDesktop);
        }

        let out_w = (right - left) as u32;
        let out_h = (bottom - top) as u32;
        let src_x = (left - vb.x) as u32;
        let src_y = (top - vb.y) as u32;
        let mut out = vec![0u8; out_w as usize * out_h as usize * 4];
        let src_stride = cache.stride;

        for row in 0..out_h {
            let src_row = src_y + row;
            let dst_off = row as usize * out_w as usize * 4;
            let src_off = src_row as usize * src_stride + src_x as usize * 4;
            let end = src_off + out_w as usize * 4;
            if end > cache.pixels.len() {
                return Err(CaptureError::Message(
                    "portal capture: frame buffer shorter than expected".into(),
                ));
            }
            out[dst_off..dst_off + out_w as usize * 4].copy_from_slice(&cache.pixels[src_off..end]);
        }

        RgbaImage::from_raw(out_w, out_h, out)
            .ok_or_else(|| CaptureError::Message("portal capture: RGBA size mismatch".into()))
    }

    pub fn capture_rect_rgb_ref(&self, rect: DesktopRect) -> Result<RgbCapture, CaptureError> {
        Ok(RgbCapture::from_rgba(&self.capture_rect_ref(rect)?))
    }

    pub fn virtual_bounds_ref(&self) -> Result<DesktopRect, CaptureError> {
        let slot = self.frame.0.lock();
        if slot.cache.virtual_bounds.w <= 0 || slot.cache.virtual_bounds.h <= 0 {
            return Err(CaptureError::Message(
                "portal capture: virtual bounds unavailable before stream metadata".into(),
            ));
        }
        Ok(slot.cache.virtual_bounds)
    }

    pub fn monitor_rects_ref(&self) -> Result<Vec<DesktopRect>, CaptureError> {
        let slot = self.frame.0.lock();
        if slot.cache.monitor_rects.is_empty() {
            return Err(CaptureError::Message(
                "portal capture: monitor layout unavailable before stream metadata".into(),
            ));
        }
        Ok(slot.cache.monitor_rects.clone())
    }

    fn wait_for_fresh_frame(&self, timeout: Duration) -> Result<(), CaptureError> {
        let (lock, cvar) = &*self.frame;
        let start_gen = lock.lock().generation;
        let deadline = Instant::now() + timeout;
        let mut slot = lock.lock();
        while slot.generation <= start_gen {
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                cap_log(
                    "PORTAL",
                    "wait",
                    &format!("fresh frame timeout after gen={start_gen}"),
                );
                break;
            }
            cvar.wait_for(&mut slot, remaining);
        }
        Ok(())
    }

    pub fn monitor_sizes_ref(&self) -> Result<Vec<(i32, i32)>, CaptureError> {
        Ok(self
            .monitor_rects_ref()?
            .into_iter()
            .map(|r| (r.w, r.h))
            .collect())
    }
}

impl Drop for PortalCapturer {
    fn drop(&mut self) {
        self.shutdown.store(true, Ordering::SeqCst);
        if let Some(tx) = self.quit_tx.lock().take() {
            let _ = tx.send(PwThreadMsg::Quit);
        }
        if let Some(handle) = self.thread.lock().take() {
            let _ = handle.join();
        }
    }
}

fn portal_pw_thread(
    frame: Arc<(Mutex<FrameSlot>, Condvar)>,
    shutdown: Arc<AtomicBool>,
    ready: SyncSender<Result<(), CaptureError>>,
    msg_rx: Receiver<PwThreadMsg>,
) -> Result<(), CaptureError> {
    let ready = Arc::new(ready);
    let (portal_hold, setup) = propagate_ready(&ready, pollster::block_on(open_portal_session()))?;

    {
        let mut slot = frame.0.lock();
        slot.cache.virtual_bounds = setup.virtual_bounds;
        slot.cache.monitor_rects = setup.monitor_rects.clone();
    }

    ensure_spa_plugin_dir();
    pw::init();
    let mainloop = propagate_ready(
        &ready,
        MainLoopRc::new(None).map_err(pipewire_main_loop_err),
    )?;
    let context = propagate_ready(
        &ready,
        ContextRc::new(&mainloop, None)
            .map_err(|e| CaptureError::Message(format!("PipeWire context: {e}"))),
    )?;
    let core = propagate_ready(
        &ready,
        context
            .connect_fd_rc(setup.fd, None)
            .map_err(|e| CaptureError::Message(format!("PipeWire connect: {e}"))),
    )?;

    let frame_cb = Arc::clone(&frame);
    let shutdown_cb = Arc::clone(&shutdown);
    let first_frame = Arc::new(AtomicBool::new(false));
    let first_frame_cb = Arc::clone(&first_frame);
    let ready_cb = Arc::clone(&ready);

    let stream = propagate_ready(
        &ready,
        StreamRc::new(
            core,
            "sqyre-screencast",
            properties! {
                *pw::keys::MEDIA_TYPE => "Video",
                *pw::keys::MEDIA_CATEGORY => "Capture",
                *pw::keys::MEDIA_ROLE => "Screen",
            },
        )
        .map_err(|e| CaptureError::Message(format!("PipeWire stream: {e}"))),
    )?;

    let _listener = propagate_ready(
        &ready,
        stream
            .add_local_listener_with_user_data(UserData {
                format: VideoInfoRaw::default(),
            })
            .param_changed(|_, user_data, id, param| {
                let Some(param) = param else { return };
                if id != pw::spa::param::ParamType::Format.as_raw() {
                    return;
                }
                let Ok((media_type, media_subtype)) =
                    pw::spa::param::format_utils::parse_format(param)
                else {
                    return;
                };
                if media_type != pw::spa::param::format::MediaType::Video
                    || media_subtype != pw::spa::param::format::MediaSubtype::Raw
                {
                    return;
                }
                let _ = user_data.format.parse(param);
            })
            .process(move |stream, user_data| {
                if shutdown_cb.load(Ordering::SeqCst) {
                    return;
                }
                let Some(mut buffer) = stream.dequeue_buffer() else {
                    return;
                };
                let datas = buffer.datas_mut();
                if datas.is_empty() {
                    return;
                }
                let data = &mut datas[0];
                let chunk = data.chunk();
                let size = chunk.size() as usize;
                let row_stride = if chunk.stride() > 0 {
                    chunk.stride() as usize
                } else {
                    0
                };
                if size == 0 {
                    return;
                }
                let Some(bytes) = data.data() else {
                    return;
                };
                let stride = if row_stride > 0 {
                    row_stride
                } else {
                    bytes.len().max(1)
                };
                let width = user_data.format.size().width.max(1);
                let height = user_data.format.size().height.max(1);
                let format = user_data.format.format();
                let needed = height as usize * stride;
                let (lock, cvar) = &*frame_cb;
                let mut slot = lock.lock();
                let cache = &mut slot.cache;
                if cache.pixels.len() != needed {
                    cache.pixels.resize(needed, 0);
                }
                cache.width = width;
                cache.height = height;
                cache.stride = stride;
                if copy_pw_frame_to_rgba(
                    bytes,
                    size,
                    stride,
                    width,
                    height,
                    format,
                    &mut cache.pixels,
                )
                .is_ok()
                {
                    cache.ready = true;
                    slot.generation = slot.generation.saturating_add(1);
                    cvar.notify_all();
                    if !first_frame_cb.swap(true, Ordering::SeqCst) {
                        let _ = ready_cb.send(Ok(()));
                    }
                }
            })
            .register()
            .map_err(|e| CaptureError::Message(format!("PipeWire listener: {e}"))),
    )?;

    let obj = pw::spa::pod::object!(
        pw::spa::utils::SpaTypes::ObjectParamFormat,
        pw::spa::param::ParamType::EnumFormat,
        pw::spa::pod::property!(
            pw::spa::param::format::FormatProperties::MediaType,
            Id,
            pw::spa::param::format::MediaType::Video
        ),
        pw::spa::pod::property!(
            pw::spa::param::format::FormatProperties::MediaSubtype,
            Id,
            pw::spa::param::format::MediaSubtype::Raw
        ),
        pw::spa::pod::property!(
            pw::spa::param::format::FormatProperties::VideoFormat,
            Choice,
            Enum,
            Id,
            VideoFormat::RGBA,
            VideoFormat::RGBx,
            VideoFormat::BGRx,
            VideoFormat::BGRA,
        ),
    );
    let values: Vec<u8> = propagate_ready(
        &ready,
        pw::spa::pod::serialize::PodSerializer::serialize(
            std::io::Cursor::new(Vec::new()),
            &pw::spa::pod::Value::Object(obj),
        )
        .map_err(|e| CaptureError::Message(format!("PipeWire format pod: {e}")))
        .map(|v| v.0.into_inner()),
    )?;
    let pod = propagate_ready(
        &ready,
        Pod::from_bytes(&values)
            .ok_or_else(|| CaptureError::Message("PipeWire pod bytes invalid".into())),
    )?;
    let mut params = [pod];

    propagate_ready(
        &ready,
        stream
            .connect(
                pw::spa::utils::Direction::Input,
                Some(setup.node_id),
                pw::stream::StreamFlags::AUTOCONNECT | pw::stream::StreamFlags::MAP_BUFFERS,
                &mut params,
            )
            .map_err(|e| CaptureError::Message(format!("PipeWire stream connect: {e}"))),
    )?;

    let loop_ref = mainloop.loop_();
    while !shutdown.load(Ordering::SeqCst) {
        if msg_rx.try_recv().is_ok() {
            break;
        }
        loop_ref.iterate(pw::loop_::Timeout::Finite(Duration::from_millis(200)));
    }

    drop(portal_hold);

    if !first_frame.load(Ordering::SeqCst) {
        return propagate_ready(
            &ready,
            Err(CaptureError::Message(
                "portal PipeWire stream ended before first frame".into(),
            )),
        );
    }
    Ok(())
}

/// PipeWire loads SPA plugins from the host; bundled `libpipewire` in AppImage/bundle breaks this.
fn ensure_spa_plugin_dir() {
    if std::env::var_os("SPA_PLUGIN_DIR").is_some() {
        return;
    }
    const CANDIDATES: &[&str] = &[
        "/usr/lib/x86_64-linux-gnu/spa-0.2",
        "/usr/lib64/spa-0.2",
        "/usr/lib/spa-0.2",
    ];
    for dir in CANDIDATES {
        let support = std::path::Path::new(dir).join("support/libspa-support.so");
        if support.exists() {
            // SAFETY: called on the portal thread before PipeWire worker threads exist.
            unsafe { std::env::set_var("SPA_PLUGIN_DIR", dir) };
            cap_log("PORTAL", "spa", &format!("SPA_PLUGIN_DIR={dir}"));
            return;
        }
    }
}

fn pipewire_main_loop_err(e: pw::Error) -> CaptureError {
    CaptureError::Message(format!(
        "PipeWire main loop: {e} (host PipeWire with SPA plugins required; \
         do not bundle libpipewire in portable releases)"
    ))
}

fn propagate_ready<T>(
    ready: &Arc<SyncSender<Result<(), CaptureError>>>,
    result: Result<T, CaptureError>,
) -> Result<T, CaptureError> {
    if let Err(ref e) = result {
        let _ = ready.send(Err(e.clone()));
    }
    result
}

struct UserData {
    format: VideoInfoRaw,
}

/// Holds portal DBus session open for the lifetime of the PipeWire loop.
struct PortalHold {
    _proxy: ashpd::desktop::screencast::Screencast<'static>,
    _session: ashpd::desktop::Session<'static, ashpd::desktop::screencast::Screencast<'static>>,
}

async fn open_portal_session() -> Result<(PortalHold, PwSetup), CaptureError> {
    use ashpd::desktop::screencast::{CursorMode, Screencast, SourceType, Stream};
    use ashpd::desktop::PersistMode;
    use enumflags2::BitFlags;

    cap_log("PORTAL", "start", "interface=ScreenCast");
    let proxy = Screencast::new()
        .await
        .map_err(portal_err("Screencast proxy"))?;
    let session = proxy
        .create_session()
        .await
        .map_err(portal_err("create_session"))?;
    proxy
        .select_sources(
            &session,
            CursorMode::Hidden,
            BitFlags::from(SourceType::Monitor),
            true,
            None,
            PersistMode::DoNot,
        )
        .await
        .map_err(portal_err("select_sources"))?;
    let response = proxy
        .start(&session, None)
        .await
        .map_err(portal_err("start"))?
        .response()
        .map_err(portal_err("start response"))?;
    let streams = response.streams();
    let stream = streams
        .first()
        .ok_or_else(|| CaptureError::Message("portal ScreenCast returned no streams".into()))?;
    let node_id = stream.pipe_wire_node_id();
    let monitor_rects: Vec<DesktopRect> = streams
        .iter()
        .filter_map(|s: &Stream| {
            let (x, y) = s.position()?;
            let (w, h) = s.size()?;
            Some(DesktopRect { x, y, w, h })
        })
        .collect();
    let virtual_bounds = if monitor_rects.is_empty() {
        let (w, h) = stream.size().unwrap_or((1920, 1080));
        DesktopRect { x: 0, y: 0, w, h }
    } else {
        monitor_rects.iter().copied().reduce(union_rect).unwrap()
    };
    let fd = proxy
        .open_pipe_wire_remote(&session)
        .await
        .map_err(portal_err("open_pipe_wire_remote"))?;

    let hold = PortalHold {
        _proxy: proxy,
        _session: session,
    };
    Ok((
        hold,
        PwSetup {
            node_id,
            fd,
            virtual_bounds,
            monitor_rects,
        },
    ))
}

fn union_rect(a: DesktopRect, b: DesktopRect) -> DesktopRect {
    let left = a.x.min(b.x);
    let top = a.y.min(b.y);
    let right = (a.x + a.w).max(b.x + b.w);
    let bottom = (a.y + a.h).max(b.y + b.h);
    DesktopRect {
        x: left,
        y: top,
        w: right - left,
        h: bottom - top,
    }
}

fn portal_err(step: &'static str) -> impl Fn(ashpd::Error) -> CaptureError {
    move |e| {
        cap_log("PORTAL", "fail", &format!("step={step} error={e}"));
        CaptureError::Message(format!("portal {step}: {e}"))
    }
}

fn copy_pw_frame_to_rgba(
    src: &[u8],
    size: usize,
    stride: usize,
    width: u32,
    height: u32,
    format: VideoFormat,
    dst: &mut [u8],
) -> Result<(), CaptureError> {
    let w = width as usize;
    let h = height as usize;
    let needed = h * w * 4;
    if dst.len() < needed {
        return Err(CaptureError::Message(
            "portal capture: RGBA buffer too small".into(),
        ));
    }
    for y in 0..h {
        let src_off = y * stride;
        let dst_off = y * w * 4;
        if src_off >= size.min(src.len()) {
            break;
        }
        let row_len = stride.min(src.len().saturating_sub(src_off));
        let row = &src[src_off..src_off + row_len];
        let dst_row = &mut dst[dst_off..dst_off + w * 4];
        swizzle_row_to_rgba(row, w, format, dst_row)?;
    }
    Ok(())
}

fn swizzle_row_to_rgba(
    row: &[u8],
    width: usize,
    format: VideoFormat,
    dst: &mut [u8],
) -> Result<(), CaptureError> {
    let bpp = match format {
        VideoFormat::RGB | VideoFormat::BGR => 3,
        VideoFormat::RGBA | VideoFormat::BGRA | VideoFormat::RGBx | VideoFormat::BGRx => 4,
        other => {
            return Err(CaptureError::Message(format!(
                "portal capture: unsupported PipeWire format {other:?}"
            )));
        }
    };
    for x in 0..width {
        let src_off = x * bpp;
        let dst_off = x * 4;
        if src_off + bpp > row.len() || dst_off + 4 > dst.len() {
            break;
        }
        match format {
            VideoFormat::RGBA => {
                dst[dst_off..dst_off + 4].copy_from_slice(&row[src_off..src_off + 4])
            }
            VideoFormat::BGRA => {
                dst[dst_off] = row[src_off + 2];
                dst[dst_off + 1] = row[src_off + 1];
                dst[dst_off + 2] = row[src_off];
                dst[dst_off + 3] = row[src_off + 3];
            }
            VideoFormat::RGBx => {
                dst[dst_off..dst_off + 3].copy_from_slice(&row[src_off..src_off + 3]);
                dst[dst_off + 3] = 255;
            }
            VideoFormat::BGRx => {
                dst[dst_off] = row[src_off + 2];
                dst[dst_off + 1] = row[src_off + 1];
                dst[dst_off + 2] = row[src_off];
                dst[dst_off + 3] = 255;
            }
            VideoFormat::RGB => {
                dst[dst_off..dst_off + 3].copy_from_slice(&row[src_off..src_off + 3]);
                dst[dst_off + 3] = 255;
            }
            VideoFormat::BGR => {
                dst[dst_off] = row[src_off + 2];
                dst[dst_off + 1] = row[src_off + 1];
                dst[dst_off + 2] = row[src_off];
                dst[dst_off + 3] = 255;
            }
            _ => {}
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bgrx_row_to_rgba() {
        let row = [0u8, 1, 2, 0, 10, 11, 12, 0];
        let mut out = [0u8; 8];
        swizzle_row_to_rgba(&row, 2, VideoFormat::BGRx, &mut out).unwrap();
        assert_eq!(out, [2, 1, 0, 255, 12, 11, 10, 255]);
    }

    #[test]
    fn union_monitor_rects() {
        let a = DesktopRect {
            x: 0,
            y: 0,
            w: 1920,
            h: 1080,
        };
        let b = DesktopRect {
            x: 1920,
            y: 0,
            w: 1920,
            h: 1080,
        };
        let u = union_rect(a, b);
        assert_eq!(u.w, 3840);
        assert_eq!(u.h, 1080);
    }
}
