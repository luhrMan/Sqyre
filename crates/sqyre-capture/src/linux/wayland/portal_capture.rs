//! Wayland screen capture via XDG portal ScreenCast + PipeWire.

use crate::cap_log;
use crate::error::CaptureError;
use crate::linux::session::{LinuxCaptureBackend, LinuxSessionInfo};
use crate::linux::wayland::eis::EisInput;
use image::RgbaImage;
use parking_lot::{Condvar, Mutex};
use pipewire as pw;
use pw::context::ContextRc;
use pw::main_loop::MainLoopRc;
use pw::properties::properties;
use pw::spa::buffer::{
    meta::{MetaCursor, MetaHeader, Metadata},
    ChunkFlags, DataType,
};
use pw::spa::param::video::{VideoFormat, VideoInfoRaw};
use pw::spa::pod::Pod;
use pw::stream::StreamRc;
use sqyre_ports::{AutomationError, DesktopRect, RgbCapture};
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::os::fd::OwnedFd;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, Sender, SyncSender};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

static INPUT_TX: Mutex<Option<Sender<EisCmd>>> = Mutex::new(None);
static PENDING_EIS_FD: Mutex<Option<OwnedFd>> = Mutex::new(None);
static LAST_ABS: Mutex<Option<(i32, i32)>> = Mutex::new(None);
static PORTAL_CURSOR: Mutex<Option<(i32, i32)>> = Mutex::new(None);
static LOGGED_PORTAL_CURSOR: AtomicBool = AtomicBool::new(false);
static LOGGED_CURSOR_META: AtomicBool = AtomicBool::new(false);
static EIS_READY: AtomicBool = AtomicBool::new(false);
static EIS_SHUTDOWN: AtomicBool = AtomicBool::new(false);
static REMOTE_DESKTOP_GRANTED: AtomicBool = AtomicBool::new(false);

enum EisCmd {
    Move {
        x: i32,
        y: i32,
        reply: Sender<Result<(), AutomationError>>,
    },
    Click {
        button: u32,
        down: bool,
        reply: Sender<Result<(), AutomationError>>,
    },
    Scroll {
        up: bool,
        reply: Sender<Result<(), AutomationError>>,
    },
    Key {
        evdev: u32,
        down: bool,
        reply: Sender<Result<(), AutomationError>>,
    },
}

const BTN_LEFT: u32 = 0x110;
const BTN_RIGHT: u32 = 0x111;
const BTN_MIDDLE: u32 = 0x112;

const REGION_FRAME_WAIT: Duration = Duration::from_millis(50);
const KICK_FRAME_WAIT: Duration = Duration::from_millis(80);

/// Portal + PipeWire capturer for Wayland sessions.
pub struct PortalCapturer {
    frame: Arc<(Mutex<FrameSlot>, Condvar)>,
    shutdown: Arc<AtomicBool>,
    quit_tx: Mutex<Option<Sender<PwThreadMsg>>>,
    thread: Mutex<Option<JoinHandle<()>>>,
    kick: Mutex<Option<crate::x11_capture::CompositorKick>>,
}

struct FrameSlot {
    cache: FrameCache,
    /// Incremented on every PipeWire frame copied into `cache`.
    generation: u64,
    /// Last global generation at which each stream dest was copied. Search waits
    /// on the dests that overlap the crop so the other monitor cannot starve it.
    region_gen: Vec<(DesktopRect, u64)>,
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

struct PwStreamSetup {
    node_id: u32,
    rect: DesktopRect,
}

struct PwSetup {
    fd: OwnedFd,
    virtual_bounds: DesktopRect,
    monitor_rects: Vec<DesktopRect>,
    streams: Vec<PwStreamSetup>,
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
        if crate::linux::shared_capturer_open_superseded() {
            return Err(CaptureError::Message(
                "portal ScreenCast superseded by a newer picker request".into(),
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
                region_gen: Vec::new(),
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
            kick: Mutex::new(None),
        })
    }

    /// Wait until a PipeWire stream that overlaps `rect` copies a newer frame.
    /// A global generation bump is not enough: GNOME often delivers the other
    /// monitor first, which left nested wait-until-found on an unchanged crop.
    fn wait_for_overlapping_stream_frame(&self, rect: DesktopRect) {
        let min_gen = {
            let slot = self.frame.0.lock();
            region_generation(&slot, rect)
        };
        let region_after = wait_until_region_after(
            &self.frame.0,
            &self.frame.1,
            rect,
            min_gen,
            REGION_FRAME_WAIT,
        );
        if region_after <= min_gen {
            let dest = {
                let slot = self.frame.0.lock();
                overlapping_stream_dest(&slot, rect)
            };
            {
                let mut kick = self.kick.lock();
                if kick.is_none() {
                    *kick = crate::x11_capture::CompositorKick::open();
                }
                if let Some(k) = kick.as_mut() {
                    k.map_rect(dest);
                }
            }
            let _ = wait_until_region_after(
                &self.frame.0,
                &self.frame.1,
                rect,
                min_gen,
                KICK_FRAME_WAIT,
            );
            if let Some(k) = self.kick.lock().as_mut() {
                k.unmap();
            }
        }
    }

    /// Capture after waiting for a new PipeWire frame on the overlapping stream.
    pub fn capture_rect_fresh_ref(&self, rect: DesktopRect) -> Result<RgbaImage, CaptureError> {
        self.wait_for_overlapping_stream_frame(rect);
        self.capture_rect_ref(rect)
    }

    pub fn capture_rect_ref(&self, rect: DesktopRect) -> Result<RgbaImage, CaptureError> {
        self.crop_cached_rgba(rect)
    }

    pub fn capture_rect_rgb_ref(&self, rect: DesktopRect) -> Result<RgbCapture, CaptureError> {
        self.crop_cached_rgb(rect)
    }

    pub fn capture_rect_rgb_fresh_ref(
        &self,
        rect: DesktopRect,
    ) -> Result<RgbCapture, CaptureError> {
        self.wait_for_overlapping_stream_frame(rect);
        self.crop_cached_rgb(rect)
    }

    fn crop_cached_rgba(&self, rect: DesktopRect) -> Result<RgbaImage, CaptureError> {
        let slot = self.frame.0.lock();
        let crop = cache_crop_geom(&slot.cache, rect)?;
        copy_cache_rgba(&slot.cache, crop)
    }

    fn crop_cached_rgb(&self, rect: DesktopRect) -> Result<RgbCapture, CaptureError> {
        let slot = self.frame.0.lock();
        let crop = cache_crop_geom(&slot.cache, rect)?;
        copy_cache_rgb(&slot.cache, crop)
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
        drop(self.kick.lock().take());
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
    let eis_fd = PENDING_EIS_FD.lock().take();

    {
        let mut slot = frame.0.lock();
        slot.cache.virtual_bounds = setup.virtual_bounds;
        slot.cache.monitor_rects = setup.monitor_rects.clone();
        ensure_cache_contains(&mut slot.cache, setup.virtual_bounds);
    }

    let stream_count = setup.streams.len();
    if stream_count == 0 {
        return propagate_ready(
            &ready,
            Err(CaptureError::Message(
                "portal ScreenCast returned no PipeWire streams".into(),
            )),
        );
    }
    let node_id = setup.streams[0].node_id;
    cap_log(
        "PORTAL",
        "pw",
        &format!("connecting node={node_id} streams={stream_count}"),
    );

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
    let loop_ref = mainloop.loop_();
    let registry = propagate_ready(
        &ready,
        core.get_registry_rc()
            .map_err(|e| CaptureError::Message(format!("PipeWire registry: {e}"))),
    )?;
    let node_serials =
        propagate_ready(&ready, pw_collect_node_serials(&core, &registry, loop_ref))?;

    let frame_cb = Arc::clone(&frame);
    let shutdown_cb = Arc::clone(&shutdown);
    let first_ready = Arc::new(AtomicBool::new(false));
    let logged_unmap = Arc::new(AtomicBool::new(false));
    let logged_copy = Arc::new(AtomicBool::new(false));
    let logged_type = Arc::new(AtomicBool::new(false));
    let ready_cb = Arc::clone(&ready);

    let values: Vec<u8> = propagate_ready(&ready, pw_video_enum_format_bytes())?;
    let cursor_meta: Vec<u8> = propagate_ready(&ready, pw_cursor_meta_bytes())?;
    let mut stream_holds = Vec::with_capacity(setup.streams.len());
    let mut listeners = Vec::with_capacity(setup.streams.len());
    for (index, stream_setup) in setup.streams.iter().enumerate() {
        let node_id = stream_setup.node_id;
        let serial = node_serials.get(&node_id).cloned();
        cap_log(
            "PORTAL",
            "pw",
            &format!(
                "stream {index} node={node_id} serial={}",
                serial.as_deref().unwrap_or("-")
            ),
        );

        let mut props = properties! {
            *pw::keys::MEDIA_TYPE => "Video",
            *pw::keys::MEDIA_CATEGORY => "Capture",
            *pw::keys::MEDIA_ROLE => "Screen",
        };
        if let Some(serial) = serial.as_deref() {
            props.insert("target.object", serial);
        }

        let stream = propagate_ready(
            &ready,
            StreamRc::new(core.clone(), &format!("sqyre-screencast-{index}"), props)
                .map_err(|e| CaptureError::Message(format!("PipeWire stream: {e}"))),
        )?;

        let frame_stream = Arc::clone(&frame_cb);
        let shutdown_stream = Arc::clone(&shutdown_cb);
        let first_ready_stream = Arc::clone(&first_ready);
        let logged_unmap_stream = Arc::clone(&logged_unmap);
        let logged_copy_stream = Arc::clone(&logged_copy);
        let logged_type_stream = Arc::clone(&logged_type);
        let ready_stream = Arc::clone(&ready_cb);
        let listener = propagate_ready(
            &ready,
            stream
                .add_local_listener_with_user_data(UserData {
                    format: VideoInfoRaw::default(),
                    monitor_rect: stream_setup.rect,
                })
                .state_changed(move |_, _, old, new| {
                    cap_log(
                        "PORTAL",
                        "pw",
                        &format!("stream {index} {old:?} -> {new:?}"),
                    );
                })
                .param_changed(move |stream, user_data, id, param| {
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
                    let fps = user_data.format.framerate();
                    let max_fps = user_data.format.max_framerate();
                    let size = user_data.format.size();
                    cap_log(
                        "PORTAL",
                        "pw",
                        &format!(
                            "format={:?} {}x{} fps={}/{} max={}/{}",
                            user_data.format.format(),
                            size.width,
                            size.height,
                            fps.num,
                            fps.denom,
                            max_fps.num,
                            max_fps.denom
                        ),
                    );
                    let width = size.width.max(1) as i32;
                    let height = size.height.max(1) as i32;
                    let stride = width.saturating_mul(4);
                    let bytes = stride.saturating_mul(height);
                    match apply_stream_buffer_params(stream, bytes, stride) {
                        Ok(()) => cap_log(
                            "PORTAL",
                            "cursor",
                            &format!("stream {index} params=buffers+header+cursor"),
                        ),
                        Err(e) => cap_log(
                            "PORTAL",
                            "cursor",
                            &format!("stream {index} update_params: {e}"),
                        ),
                    }
                })
                .process(move |stream, user_data| {
                    if shutdown_stream.load(Ordering::SeqCst) {
                        return;
                    }
                    let dest = user_data.monitor_rect;
                    let stream_size = user_data.format.size();
                    let Some(mut buffer) = stream.dequeue_buffer() else {
                        return;
                    };
                    let cursor_info = buffer.find_meta::<MetaCursor>().map(|c| {
                        let p = c.position();
                        (c.is_valid(), p.x, p.y)
                    });
                    if LOGGED_CURSOR_META
                        .compare_exchange(false, true, Ordering::Relaxed, Ordering::Relaxed)
                        .is_ok()
                    {
                        match cursor_info {
                            Some((valid, x, y)) => cap_log(
                                "PORTAL",
                                "cursor",
                                &format!(
                                    "meta=yes valid={valid} pos={x},{y} stream={}x{} dest={}x{}+{}+{}",
                                    stream_size.width,
                                    stream_size.height,
                                    dest.w,
                                    dest.h,
                                    dest.x,
                                    dest.y
                                ),
                            ),
                            None => cap_log("PORTAL", "cursor", "meta=no"),
                        }
                    }
                    if let Some((true, lx, ly)) = cursor_info {
                        note_portal_cursor(dest, lx, ly, stream_size.width, stream_size.height);
                    }
                    let datas = buffer.datas_mut();
                    if datas.is_empty() {
                        return;
                    }
                    let data = &mut datas[0];
                    if data.chunk().flags().contains(ChunkFlags::CORRUPTED) {
                        return;
                    }
                    let offset = data.chunk().offset() as usize;
                    let size = data.chunk().size() as usize;
                    let row_stride = if data.chunk().stride() > 0 {
                        data.chunk().stride() as usize
                    } else {
                        0
                    };
                    if size == 0 {
                        return;
                    }
                    let ty = data.type_();
                    if !logged_type_stream.swap(true, Ordering::SeqCst) {
                        cap_log(
                            "PORTAL",
                            "pw",
                            &format!("data={ty:?} offset={offset} size={size} stride={row_stride}"),
                        );
                    }
                    let width = user_data.format.size().width.max(1);
                    let height = user_data.format.size().height.max(1);
                    let format = user_data.format.format();
                    if user_data.monitor_rect.w <= 0 {
                        user_data.monitor_rect.w = width as i32;
                    }
                    if user_data.monitor_rect.h <= 0 {
                        user_data.monitor_rect.h = height as i32;
                    }
                    let dest = user_data.monitor_rect;
                    let copy_result = with_spa_chunk_bytes(data, offset, size, |bytes| {
                        let src_stride = if row_stride > 0 {
                            row_stride
                        } else {
                            bytes.len().max(1)
                        };
                        let (lock, cvar) = &*frame_stream;
                        let mut slot = lock.lock();
                        ensure_cache_contains(&mut slot.cache, dest);
                        let cache = &mut slot.cache;
                        let vb = cache.virtual_bounds;
                        let dst_x = (dest.x - vb.x).max(0) as usize;
                        let dst_y = (dest.y - vb.y).max(0) as usize;
                        let copied = copy_pw_frame_into_rect(
                            bytes,
                            bytes.len(),
                            src_stride,
                            width,
                            height,
                            format,
                            &mut cache.pixels,
                            cache.stride,
                            dst_x,
                            dst_y,
                            dest.w.max(1) as u32,
                            dest.h.max(1) as u32,
                        );
                        if copied.is_ok() {
                            cache.ready = true;
                            if !first_ready_stream.swap(true, Ordering::SeqCst) {
                                let _ = ready_stream.send(Ok(()));
                            }
                            note_region_copy(&mut slot, dest);
                            cvar.notify_all();
                        }
                        copied
                    });
                    match copy_result {
                        None => {
                            if !logged_unmap_stream.swap(true, Ordering::SeqCst) {
                                cap_log("PORTAL", "pw", "buffer not mapped");
                            }
                        }
                        Some(Err(e)) => {
                            if !logged_copy_stream.swap(true, Ordering::SeqCst) {
                                cap_log(
                                    "PORTAL",
                                    "pw",
                                    &format!("copy failed format={format:?} {e}"),
                                );
                            }
                        }
                        Some(Ok(())) => {}
                    }
                })
                .register()
                .map_err(|e| CaptureError::Message(format!("PipeWire listener: {e}"))),
        )?;

        let format_pod = propagate_ready(
            &ready,
            Pod::from_bytes(&values)
                .ok_or_else(|| CaptureError::Message("PipeWire pod bytes invalid".into())),
        )?;
        let meta_pod = propagate_ready(
            &ready,
            Pod::from_bytes(&cursor_meta)
                .ok_or_else(|| CaptureError::Message("PipeWire cursor meta pod invalid".into())),
        )?;
        let mut params = [format_pod, meta_pod];
        let target = if serial.is_some() {
            None
        } else {
            Some(node_id)
        };
        propagate_ready(
            &ready,
            stream
                .connect(
                    pw::spa::utils::Direction::Input,
                    target,
                    pw::stream::StreamFlags::AUTOCONNECT
                        | pw::stream::StreamFlags::MAP_BUFFERS
                        | pw::stream::StreamFlags::DONT_RECONNECT,
                    &mut params,
                )
                .map_err(|e| CaptureError::Message(format!("PipeWire stream connect: {e}"))),
        )?;
        listeners.push(listener);
        stream_holds.push(stream);
    }
    let _registry = registry;
    if let Some(fd) = eis_fd {
        spawn_eis_thread(fd);
    }

    while !shutdown.load(Ordering::SeqCst) {
        match msg_rx.try_recv() {
            Ok(PwThreadMsg::Quit) => break,
            Err(mpsc::TryRecvError::Empty) => {}
            Err(mpsc::TryRecvError::Disconnected) => break,
        }
        loop_ref.iterate(pw::loop_::Timeout::Finite(Duration::from_millis(20)));
    }

    drop(listeners);
    drop(stream_holds);
    stop_eis_thread();
    REMOTE_DESKTOP_GRANTED.store(false, Ordering::SeqCst);
    drop(portal_hold);

    if !first_ready.load(Ordering::Acquire) {
        return propagate_ready(
            &ready,
            Err(CaptureError::Message(
                "portal PipeWire stream ended before first frame".into(),
            )),
        );
    }
    Ok(())
}

fn pw_collect_node_serials(
    core: &pw::core::CoreRc,
    registry: &pw::registry::RegistryRc,
    loop_ref: &pw::loop_::Loop,
) -> Result<HashMap<u32, String>, CaptureError> {
    let serials = Rc::new(RefCell::new(HashMap::<u32, String>::new()));
    let serials_cb = Rc::clone(&serials);
    let _reg = registry
        .add_listener_local()
        .global(move |global| {
            if global.type_ != pw::types::ObjectType::Node {
                return;
            }
            let serial = global
                .props
                .and_then(|p| p.get("object.serial"))
                .unwrap_or("")
                .to_string();
            let name = global
                .props
                .and_then(|p| p.get("node.name"))
                .unwrap_or("")
                .to_string();
            cap_log(
                "PORTAL",
                "pw",
                &format!("node id={} serial={serial} name={name}", global.id),
            );
            if !serial.is_empty() {
                serials_cb.borrow_mut().insert(global.id, serial);
            }
        })
        .register();
    let done = Rc::new(Cell::new(false));
    let pending = core
        .sync(0)
        .map_err(|e| CaptureError::Message(format!("PipeWire sync: {e}")))?;
    let done_cb = Rc::clone(&done);
    let _core = core
        .add_listener_local()
        .done(move |id, seq| {
            if id == pw::core::PW_ID_CORE && seq == pending {
                done_cb.set(true);
            }
        })
        .register();
    let deadline = Instant::now() + Duration::from_secs(2);
    while !done.get() {
        if Instant::now() > deadline {
            break;
        }
        loop_ref.iterate(pw::loop_::Timeout::Finite(Duration::from_millis(10)));
    }
    let map = serials.borrow().clone();
    Ok(map)
}

fn pw_video_enum_format_bytes() -> Result<Vec<u8>, CaptureError> {
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
        // Mutter fixates VideoFramerate at 0/1 (emit-on-damage). A non-zero
        // VideoFramerate range fails to negotiate. Periodic frames come from
        // VideoMaxFramerate (same as xdg-desktop-portal-wlr).
        pw::spa::pod::property!(
            pw::spa::param::format::FormatProperties::VideoFramerate,
            Fraction,
            pw::spa::utils::Fraction { num: 0, denom: 1 }
        ),
        pw::spa::pod::property!(
            pw::spa::param::format::FormatProperties::VideoMaxFramerate,
            Choice,
            Range,
            Fraction,
            pw::spa::utils::Fraction { num: 30, denom: 1 },
            pw::spa::utils::Fraction { num: 1, denom: 1 },
            pw::spa::utils::Fraction { num: 30, denom: 1 }
        ),
    );
    pw::spa::pod::serialize::PodSerializer::serialize(
        std::io::Cursor::new(Vec::new()),
        &pw::spa::pod::Value::Object(obj),
    )
    .map(|v| v.0.into_inner())
    .map_err(|e| CaptureError::Message(format!("PipeWire format pod: {e}")))
}

/// `SPA_PARAM_Meta` keys (`enum spa_param_meta` in spa/param/buffers.h).
struct ParamMetaKey(u32);
impl ParamMetaKey {
    const TYPE: Self = Self(1);
    const SIZE: Self = Self(2);
    const fn as_raw(&self) -> u32 {
        self.0
    }
}

/// `SPA_PARAM_BUFFERS_*` keys (`enum spa_param_buffers`).
struct ParamBuffersKey(u32);
impl ParamBuffersKey {
    const BUFFERS: Self = Self(1);
    const BLOCKS: Self = Self(2);
    const SIZE: Self = Self(3);
    const STRIDE: Self = Self(4);
    const DATA_TYPE: Self = Self(6);
    const fn as_raw(&self) -> u32 {
        self.0
    }
}

struct SpaId(u32);
impl SpaId {
    const fn as_raw(&self) -> u32 {
        self.0
    }
}

fn cursor_meta_size(w: u32, h: u32) -> i32 {
    let base = std::mem::size_of::<pw::spa::sys::spa_meta_cursor>()
        + std::mem::size_of::<pw::spa::sys::spa_meta_bitmap>();
    (base + (w as usize) * (h as usize) * 4) as i32
}

fn serialize_spa_object(obj: pw::spa::pod::Object) -> Result<Vec<u8>, CaptureError> {
    pw::spa::pod::serialize::PodSerializer::serialize(
        std::io::Cursor::new(Vec::new()),
        &pw::spa::pod::Value::Object(obj),
    )
    .map(|v| v.0.into_inner())
    .map_err(|e| CaptureError::Message(format!("PipeWire pod: {e}")))
}

fn int_choice_range(key: u32, default: i32, min: i32, max: i32) -> pw::spa::pod::Property {
    pw::spa::pod::Property::new(
        key,
        pw::spa::pod::Value::Choice(pw::spa::pod::ChoiceValue::Int(pw::spa::utils::Choice(
            pw::spa::utils::ChoiceFlags::empty(),
            pw::spa::utils::ChoiceEnum::Range { default, min, max },
        ))),
    )
}

fn int_choice_flags(key: u32, default: i32, flags: Vec<i32>) -> pw::spa::pod::Property {
    pw::spa::pod::Property::new(
        key,
        pw::spa::pod::Value::Choice(pw::spa::pod::ChoiceValue::Int(pw::spa::utils::Choice(
            pw::spa::utils::ChoiceFlags::empty(),
            pw::spa::utils::ChoiceEnum::Flags { default, flags },
        ))),
    )
}

fn pw_cursor_meta_bytes() -> Result<Vec<u8>, CaptureError> {
    let obj = pw::spa::pod::Object {
        type_: pw::spa::utils::SpaTypes::ObjectParamMeta.as_raw(),
        id: pw::spa::param::ParamType::Meta.as_raw(),
        properties: vec![
            pw::spa::pod::property!(ParamMetaKey::TYPE, Id, SpaId(MetaCursor::META_TYPE)),
            int_choice_range(
                ParamMetaKey::SIZE.as_raw(),
                cursor_meta_size(64, 64),
                cursor_meta_size(1, 1),
                cursor_meta_size(384, 384),
            ),
        ],
    };
    serialize_spa_object(obj)
}

fn pw_header_meta_bytes() -> Result<Vec<u8>, CaptureError> {
    let obj = pw::spa::pod::object!(
        pw::spa::utils::SpaTypes::ObjectParamMeta,
        pw::spa::param::ParamType::Meta,
        pw::spa::pod::property!(ParamMetaKey::TYPE, Id, SpaId(MetaHeader::META_TYPE)),
        pw::spa::pod::property!(
            ParamMetaKey::SIZE,
            Int,
            std::mem::size_of::<pw::spa::sys::spa_meta_header>() as i32
        ),
    );
    serialize_spa_object(obj)
}

fn pw_buffers_param_bytes(size: i32, stride: i32) -> Result<Vec<u8>, CaptureError> {
    let data_mask = (1 << DataType::MemPtr.as_raw())
        | (1 << DataType::MemFd.as_raw())
        | (1 << DataType::DmaBuf.as_raw());
    let obj = pw::spa::pod::Object {
        type_: pw::spa::utils::SpaTypes::ObjectParamBuffers.as_raw(),
        id: pw::spa::param::ParamType::Buffers.as_raw(),
        properties: vec![
            int_choice_range(ParamBuffersKey::BUFFERS.as_raw(), 8, 1, 32),
            pw::spa::pod::property!(ParamBuffersKey::BLOCKS, Int, 1),
            pw::spa::pod::Property::new(
                ParamBuffersKey::SIZE.as_raw(),
                pw::spa::pod::Value::Int(size),
            ),
            pw::spa::pod::Property::new(
                ParamBuffersKey::STRIDE.as_raw(),
                pw::spa::pod::Value::Int(stride),
            ),
            int_choice_flags(
                ParamBuffersKey::DATA_TYPE.as_raw(),
                data_mask,
                vec![data_mask],
            ),
        ],
    };
    serialize_spa_object(obj)
}

/// Mutter only attaches `SPA_META_Cursor` after the client finishes negotiation
/// with `pw_stream_update_params` (connect-time Meta is not enough).
fn apply_stream_buffer_params(
    stream: &pw::stream::Stream,
    size: i32,
    stride: i32,
) -> Result<(), CaptureError> {
    let buffers = pw_buffers_param_bytes(size, stride)?;
    let header = pw_header_meta_bytes()?;
    let cursor = pw_cursor_meta_bytes()?;
    let buffers_pod = Pod::from_bytes(&buffers)
        .ok_or_else(|| CaptureError::Message("PipeWire buffers pod invalid".into()))?;
    let header_pod = Pod::from_bytes(&header)
        .ok_or_else(|| CaptureError::Message("PipeWire header meta pod invalid".into()))?;
    let cursor_pod = Pod::from_bytes(&cursor)
        .ok_or_else(|| CaptureError::Message("PipeWire cursor meta pod invalid".into()))?;
    let mut params = [buffers_pod, header_pod, cursor_pod];
    stream
        .update_params(&mut params)
        .map_err(|e| CaptureError::Message(format!("PipeWire update_params: {e}")))
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
    monitor_rect: DesktopRect,
}

/// Holds portal DBus session open for the lifetime of the PipeWire loop.
enum PortalHold {
    Combined {
        _remote: ashpd::desktop::remote_desktop::RemoteDesktop<'static>,
        _screencast: ashpd::desktop::screencast::Screencast<'static>,
        _session: ashpd::desktop::Session<
            'static,
            ashpd::desktop::remote_desktop::RemoteDesktop<'static>,
        >,
    },
    ScreenCastOnly {
        _proxy: ashpd::desktop::screencast::Screencast<'static>,
        _session: ashpd::desktop::Session<'static, ashpd::desktop::screencast::Screencast<'static>>,
    },
}

/// Same id as `sqyre-app` (`com.sqyre.app.desktop`). GNOME persist keys off this.
const PORTAL_APP_ID: &str = "com.sqyre.app";

static FORCE_SCREENCAST_PICKER: AtomicBool = AtomicBool::new(false);
static SCREENCAST_SESSION_GRANTED: AtomicBool = AtomicBool::new(false);

/// Portal Start succeeded this process, or a restore token is on disk from a prior grant.
pub fn portal_screencast_granted() -> bool {
    SCREENCAST_SESSION_GRANTED.load(Ordering::SeqCst)
        || read_restore_token_at(&restore_token_path()).is_some()
        || read_restore_token_at(&legacy_screencast_token_path()).is_some()
}

/// Remote Desktop devices were granted on the live combined portal session.
pub fn portal_remote_desktop_granted() -> bool {
    REMOTE_DESKTOP_GRANTED.load(Ordering::SeqCst)
}

/// Whether EIS is ready to inject pointer/keyboard.
pub fn portal_input_ready() -> bool {
    EIS_READY.load(Ordering::SeqCst)
}

/// Compositor cursor from ScreenCast `CursorMode::Metadata`, in desktop pixels.
/// Works over native Wayland surfaces; XQueryPointer does not.
pub fn portal_cursor_position() -> Option<(i32, i32)> {
    *PORTAL_CURSOR.lock()
}

/// Map a stream-local cursor into the virtual desktop using the stream dest rect.
pub(crate) fn stream_cursor_to_desktop(
    dest: DesktopRect,
    lx: i32,
    ly: i32,
    stream_w: u32,
    stream_h: u32,
) -> (i32, i32) {
    let map = |local: i32, dest_origin: i32, dest_span: i32, stream_span: u32| {
        if stream_span > 0 && dest_span > 0 {
            dest_origin
                .saturating_add(((local as i64) * (dest_span as i64) / (stream_span as i64)) as i32)
        } else {
            dest_origin.saturating_add(local)
        }
    };
    (
        map(lx, dest.x, dest.w, stream_w),
        map(ly, dest.y, dest.h, stream_h),
    )
}

fn note_portal_cursor(dest: DesktopRect, lx: i32, ly: i32, stream_w: u32, stream_h: u32) {
    let pos = stream_cursor_to_desktop(dest, lx, ly, stream_w, stream_h);
    let mut g = PORTAL_CURSOR.lock();
    if *g == Some(pos) {
        return;
    }
    if LOGGED_PORTAL_CURSOR
        .compare_exchange(false, true, Ordering::Relaxed, Ordering::Relaxed)
        .is_ok()
    {
        cap_log(
            "PORTAL",
            "cursor",
            &format!(
                "desktop={},{} stream={},{} dest={}x{}+{}+{}",
                pos.0, pos.1, lx, ly, dest.w, dest.h, dest.x, dest.y
            ),
        );
    }
    *g = Some(pos);
}

pub fn portal_input_move(x: i32, y: i32) -> Result<(), AutomationError> {
    let (reply, rx) = mpsc::channel();
    send_eis(EisCmd::Move { x, y, reply })?;
    recv_eis(rx)
}

/// Last absolute pointer sent over EIS this session, if any.
pub fn portal_input_last_pos() -> Option<(i32, i32)> {
    *LAST_ABS.lock()
}

pub fn portal_input_click(button: &str, down: bool) -> Result<(), AutomationError> {
    let code = match button {
        "right" => BTN_RIGHT,
        "center" | "middle" => BTN_MIDDLE,
        _ => BTN_LEFT,
    };
    let (reply, rx) = mpsc::channel();
    send_eis(EisCmd::Click {
        button: code,
        down,
        reply,
    })?;
    recv_eis(rx)
}

pub fn portal_input_scroll(up: bool) -> Result<(), AutomationError> {
    let (reply, rx) = mpsc::channel();
    send_eis(EisCmd::Scroll { up, reply })?;
    recv_eis(rx)
}

pub fn portal_input_key(evdev: u32, down: bool) -> Result<(), AutomationError> {
    let (reply, rx) = mpsc::channel();
    send_eis(EisCmd::Key { evdev, down, reply })?;
    recv_eis(rx)
}

fn send_eis(cmd: EisCmd) -> Result<(), AutomationError> {
    let tx = INPUT_TX.lock().clone().ok_or_else(|| {
        AutomationError::Backend(
            "desktop control not granted (enable Allow Remote Interaction, then Share)".into(),
        )
    })?;
    tx.send(cmd)
        .map_err(|_| AutomationError::Backend("portal input thread exited".into()))
}

fn recv_eis(rx: Receiver<Result<(), AutomationError>>) -> Result<(), AutomationError> {
    rx.recv()
        .map_err(|_| AutomationError::Backend("portal input reply dropped".into()))?
}

fn dispatch_eis(eis: &mut EisInput, cmd: EisCmd) {
    match cmd {
        EisCmd::Move { x, y, reply } => {
            let result = eis.move_to(x, y);
            if result.is_ok() {
                *LAST_ABS.lock() = Some((x, y));
            }
            let _ = reply.send(result);
        }
        EisCmd::Click {
            button,
            down,
            reply,
        } => {
            let _ = reply.send(eis.click(button, down));
        }
        EisCmd::Scroll { up, reply } => {
            let _ = reply.send(eis.scroll(up));
        }
        EisCmd::Key { evdev, down, reply } => {
            let _ = reply.send(eis.key(evdev, down));
        }
    }
}

fn stop_eis_thread() {
    EIS_SHUTDOWN.store(true, Ordering::SeqCst);
    EIS_READY.store(false, Ordering::SeqCst);
    REMOTE_DESKTOP_GRANTED.store(false, Ordering::SeqCst);
    *INPUT_TX.lock() = None;
    *LAST_ABS.lock() = None;
    *PORTAL_CURSOR.lock() = None;
    LOGGED_PORTAL_CURSOR.store(false, Ordering::Relaxed);
    LOGGED_CURSOR_META.store(false, Ordering::Relaxed);
}

fn spawn_eis_thread(fd: OwnedFd) {
    EIS_SHUTDOWN.store(false, Ordering::SeqCst);
    cap_log("INPUT", "eis", "thread start");
    let _ = thread::Builder::new()
        .name("sqyre-eis".into())
        .spawn(move || {
            match EisInput::connect(fd, &EIS_SHUTDOWN) {
                Ok(mut eis) => {
                    let (tx, rx) = mpsc::channel();
                    *INPUT_TX.lock() = Some(tx);
                    EIS_READY.store(true, Ordering::SeqCst);
                    cap_log("INPUT", "ok", "backend=eis");
                    while !EIS_SHUTDOWN.load(Ordering::SeqCst) {
                        match rx.recv_timeout(Duration::from_millis(100)) {
                            Ok(cmd) => dispatch_eis(&mut eis, cmd),
                            Err(mpsc::RecvTimeoutError::Timeout) => {}
                            Err(mpsc::RecvTimeoutError::Disconnected) => break,
                        }
                    }
                }
                Err(e) => cap_log("INPUT", "fail", &format!("eis handshake: {e}")),
            }
            EIS_READY.store(false, Ordering::SeqCst);
            *INPUT_TX.lock() = None;
            *PENDING_EIS_FD.lock() = None;
        });
}

/// Drop the current ScreenCast session and show the share picker again (ignores restore token).
pub fn request_portal_screencast_picker() {
    FORCE_SCREENCAST_PICKER.store(true, Ordering::SeqCst);
    SCREENCAST_SESSION_GRANTED.store(false, Ordering::SeqCst);
    stop_eis_thread();
    write_restore_token_at(&restore_token_path(), None);
    write_restore_token_at(&legacy_screencast_token_path(), None);
    crate::linux::reset_shared_capturer();
    cap_log("PORTAL", "picker", "requested");
    let _ = thread::Builder::new()
        .name("sqyre-portal-picker".into())
        .spawn(|| {
            if let Err(e) = crate::linux::shared_capturer() {
                cap_log("PORTAL", "picker", &format!("reopen failed: {e}"));
            }
        });
}

fn take_force_screencast_picker() -> bool {
    FORCE_SCREENCAST_PICKER.swap(false, Ordering::SeqCst)
}

fn restore_token_path() -> std::path::PathBuf {
    dirs_home().join(".sqyre").join("wayland-remote.token")
}

fn legacy_screencast_token_path() -> std::path::PathBuf {
    dirs_home().join(".sqyre").join("wayland-screencast.token")
}

fn dirs_home() -> std::path::PathBuf {
    std::env::var_os("HOME")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| std::path::PathBuf::from("."))
}

fn read_restore_token_at(path: &std::path::Path) -> Option<String> {
    let raw = std::fs::read_to_string(path).ok()?;
    let token = raw.trim();
    if token.is_empty() || token.bytes().any(|b| b.is_ascii_whitespace()) {
        return None;
    }
    Some(token.to_string())
}

fn write_restore_token_at(path: &std::path::Path, token: Option<&str>) {
    match token.map(str::trim).filter(|t| !t.is_empty()) {
        Some(token) => {
            if let Some(parent) = path.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            if let Err(e) = std::fs::write(path, token) {
                cap_log("PORTAL", "token", &format!("save failed: {e}"));
            } else {
                cap_log("PORTAL", "token", "saved");
            }
        }
        None => {
            if path.exists() {
                let _ = std::fs::remove_file(path);
                cap_log("PORTAL", "token", "cleared");
            }
        }
    }
}

fn stream_rect(stream: &ashpd::desktop::screencast::Stream) -> DesktopRect {
    let (w, h) = stream.size().unwrap_or((0, 0));
    let (x, y) = stream.position().unwrap_or((0, 0));
    DesktopRect { x, y, w, h }
}

fn note_region_copy(slot: &mut FrameSlot, dest: DesktopRect) {
    slot.generation = slot.generation.saturating_add(1);
    let gen = slot.generation;
    if let Some((_, g)) = slot.region_gen.iter_mut().find(|(r, _)| *r == dest) {
        *g = gen;
    } else {
        slot.region_gen.push((dest, gen));
    }
}

fn region_generation(slot: &FrameSlot, rect: DesktopRect) -> u64 {
    slot.region_gen
        .iter()
        .filter(|(r, _)| rects_overlap(*r, rect))
        .map(|(_, g)| *g)
        .max()
        .unwrap_or(0)
}

fn overlapping_stream_dest(slot: &FrameSlot, rect: DesktopRect) -> DesktopRect {
    slot.cache
        .monitor_rects
        .iter()
        .copied()
        .find(|d| rects_overlap(*d, rect))
        .unwrap_or(rect)
}

#[derive(Clone, Copy)]
struct CacheCrop {
    src_x: u32,
    src_y: u32,
    out_w: u32,
    out_h: u32,
}

fn cache_crop_geom(cache: &FrameCache, rect: DesktopRect) -> Result<CacheCrop, CaptureError> {
    if rect.is_empty() {
        return Err(CaptureError::EmptyRect);
    }
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

    Ok(CacheCrop {
        src_x: (left - vb.x) as u32,
        src_y: (top - vb.y) as u32,
        out_w: (right - left) as u32,
        out_h: (bottom - top) as u32,
    })
}

fn copy_cache_rgba(cache: &FrameCache, crop: CacheCrop) -> Result<RgbaImage, CaptureError> {
    let mut out = vec![0u8; crop.out_w as usize * crop.out_h as usize * 4];
    let src_stride = cache.stride;
    for row in 0..crop.out_h {
        let dst_off = row as usize * crop.out_w as usize * 4;
        let src_off = (crop.src_y + row) as usize * src_stride + crop.src_x as usize * 4;
        let end = src_off + crop.out_w as usize * 4;
        if end > cache.pixels.len() {
            return Err(CaptureError::Message(
                "portal capture: frame buffer shorter than expected".into(),
            ));
        }
        out[dst_off..dst_off + crop.out_w as usize * 4]
            .copy_from_slice(&cache.pixels[src_off..end]);
    }
    RgbaImage::from_raw(crop.out_w, crop.out_h, out)
        .ok_or_else(|| CaptureError::Message("portal capture: RGBA size mismatch".into()))
}

fn copy_cache_rgb(cache: &FrameCache, crop: CacheCrop) -> Result<RgbCapture, CaptureError> {
    let out_w = crop.out_w as usize;
    let mut out = vec![0u8; out_w * crop.out_h as usize * 3];
    let src_stride = cache.stride;
    for row in 0..crop.out_h {
        let src_off = (crop.src_y + row) as usize * src_stride + crop.src_x as usize * 4;
        let end = src_off + out_w * 4;
        if end > cache.pixels.len() {
            return Err(CaptureError::Message(
                "portal capture: frame buffer shorter than expected".into(),
            ));
        }
        let dst_off = row as usize * out_w * 3;
        let src = &cache.pixels[src_off..end];
        let dst = &mut out[dst_off..dst_off + out_w * 3];
        for (d, s) in dst.chunks_exact_mut(3).zip(src.chunks_exact(4)) {
            d.copy_from_slice(&s[..3]);
        }
    }
    Ok(RgbCapture {
        width: crop.out_w,
        height: crop.out_h,
        data: out,
    })
}

/// Wait until a stream dest overlapping `rect` has generation `> min_gen`.
fn wait_until_region_after(
    lock: &Mutex<FrameSlot>,
    cvar: &Condvar,
    rect: DesktopRect,
    min_gen: u64,
    timeout: Duration,
) -> u64 {
    let deadline = Instant::now() + timeout;
    let mut slot = lock.lock();
    while region_generation(&slot, rect) <= min_gen {
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            cap_log(
                "PORTAL",
                "wait",
                &format!(
                    "region frame timeout after gen={min_gen} now={} region={}",
                    slot.generation,
                    region_generation(&slot, rect)
                ),
            );
            break;
        }
        cvar.wait_for(&mut slot, remaining);
    }
    region_generation(&slot, rect)
}

fn ensure_cache_contains(cache: &mut FrameCache, rect: DesktopRect) {
    if rect.w <= 0 || rect.h <= 0 {
        return;
    }
    let vb = if cache.virtual_bounds.w <= 0 || cache.virtual_bounds.h <= 0 {
        rect
    } else {
        union_rect(cache.virtual_bounds, rect)
    };
    if vb == cache.virtual_bounds && !cache.pixels.is_empty() {
        return;
    }
    cache.virtual_bounds = vb;
    cache.width = vb.w.max(0) as u32;
    cache.height = vb.h.max(0) as u32;
    cache.stride = cache.width as usize * 4;
    let len = cache.stride.saturating_mul(cache.height as usize);
    cache.pixels.resize(len, 0);
}

async fn register_portal_app() {
    let Ok(app_id) = PORTAL_APP_ID.parse::<ashpd::AppID>() else {
        return;
    };
    match ashpd::register_host_app(app_id).await {
        Ok(()) => cap_log("PORTAL", "appid", PORTAL_APP_ID),
        Err(e) => cap_log("PORTAL", "appid", &format!("register skipped: {e}")),
    }
}

async fn cursor_mode(
    proxy: &ashpd::desktop::screencast::Screencast<'_>,
) -> ashpd::desktop::screencast::CursorMode {
    use ashpd::desktop::screencast::CursorMode;
    let available = proxy.available_cursor_modes().await;
    let mode = match &available {
        Ok(modes) if modes.contains(CursorMode::Metadata) => CursorMode::Metadata,
        Ok(modes) if modes.contains(CursorMode::Hidden) => CursorMode::Hidden,
        Ok(modes) if modes.contains(CursorMode::Embedded) => CursorMode::Embedded,
        _ => CursorMode::Hidden,
    };
    cap_log(
        "PORTAL",
        "cursor",
        &format!("available={available:?} selected={mode:?}"),
    );
    mode
}

async fn source_types(
    proxy: &ashpd::desktop::screencast::Screencast<'_>,
) -> enumflags2::BitFlags<ashpd::desktop::screencast::SourceType> {
    use ashpd::desktop::screencast::SourceType;
    match proxy.available_source_types().await {
        Ok(types) if types.contains(SourceType::Monitor) => SourceType::Monitor.into(),
        Ok(types) if types.contains(SourceType::Virtual) => SourceType::Virtual.into(),
        Ok(types) if !types.is_empty() => types,
        _ => SourceType::Monitor.into(),
    }
}

async fn open_portal_session() -> Result<(PortalHold, PwSetup), CaptureError> {
    cap_log("PORTAL", "start", "interface=RemoteDesktop+ScreenCast");
    register_portal_app().await;

    let stored = if take_force_screencast_picker() {
        cap_log("PORTAL", "picker", "forcing dialog");
        None
    } else {
        read_restore_token_at(&restore_token_path())
    };
    match open_combined_session_with_token(stored.as_deref()).await {
        Ok(ok) => Ok(ok),
        Err(e) if stored.is_some() => {
            cap_log(
                "PORTAL",
                "token",
                &format!("restore failed ({e}); retrying with picker"),
            );
            write_restore_token_at(&restore_token_path(), None);
            open_combined_session_with_token(None).await
        }
        Err(e) => Err(e),
    }
}

async fn open_combined_session_with_token(
    restore_token: Option<&str>,
) -> Result<(PortalHold, PwSetup), CaptureError> {
    use ashpd::desktop::remote_desktop::{DeviceType, RemoteDesktop};
    use ashpd::desktop::screencast::Screencast;
    use ashpd::desktop::PersistMode;

    let remote = match RemoteDesktop::new().await {
        Ok(remote) => remote,
        Err(e) => {
            cap_log(
                "PORTAL",
                "rd",
                &format!("RemoteDesktop unavailable ({e}); ScreenCast only"),
            );
            return open_screencast_only_with_token(restore_token).await;
        }
    };
    let proxy = Screencast::new()
        .await
        .map_err(portal_err("Screencast proxy"))?;
    let types = source_types(&proxy).await;
    let cursor = cursor_mode(&proxy).await;
    let session = remote
        .create_session()
        .await
        .map_err(portal_err("create_session"))?;
    if restore_token.is_some() {
        cap_log("PORTAL", "token", "restoring");
    }
    remote
        .select_devices(
            &session,
            DeviceType::Keyboard | DeviceType::Pointer,
            restore_token,
            PersistMode::ExplicitlyRevoked,
        )
        .await
        .map_err(portal_err("select_devices"))?;
    proxy
        .select_sources(&session, cursor, types, true, None, PersistMode::DoNot)
        .await
        .map_err(portal_err("select_sources"))?;
    let response = remote
        .start(&session, None)
        .await
        .map_err(portal_err("start"))?
        .response()
        .map_err(portal_err("start response"))?;
    write_restore_token_at(&restore_token_path(), response.restore_token());
    let has_input = response.devices().contains(DeviceType::Keyboard)
        || response.devices().contains(DeviceType::Pointer);
    REMOTE_DESKTOP_GRANTED.store(has_input, Ordering::SeqCst);
    let streams_meta = response.streams().unwrap_or(&[]);
    if streams_meta.is_empty() {
        return Err(CaptureError::Message(
            "portal ScreenCast returned no streams".into(),
        ));
    }
    SCREENCAST_SESSION_GRANTED.store(true, Ordering::SeqCst);
    cap_log(
        "PORTAL",
        "grant",
        &format!(
            "streams={} input={has_input} cursor={cursor:?} persist={}",
            streams_meta.len(),
            response.restore_token().is_some()
        ),
    );
    let eis_fd = if has_input {
        match remote.connect_to_eis(&session).await {
            Ok(fd) => {
                cap_log("INPUT", "eis", "ConnectToEIS fd");
                Some(fd)
            }
            Err(e) => {
                cap_log("INPUT", "fail", &format!("ConnectToEIS: {e}"));
                None
            }
        }
    } else {
        cap_log(
            "INPUT",
            "fail",
            "reason=no_remote_interaction enable Allow Remote Interaction in the share dialog",
        );
        None
    };
    let opened = finish_pipewire_setup(
        streams_meta,
        proxy,
        PortalSessionKind::Combined { remote, session },
    )
    .await?;
    *PENDING_EIS_FD.lock() = eis_fd;
    Ok(opened)
}

enum PortalSessionKind {
    Combined {
        remote: ashpd::desktop::remote_desktop::RemoteDesktop<'static>,
        session: ashpd::desktop::Session<
            'static,
            ashpd::desktop::remote_desktop::RemoteDesktop<'static>,
        >,
    },
    ScreenCast {
        session: ashpd::desktop::Session<'static, ashpd::desktop::screencast::Screencast<'static>>,
    },
}

async fn finish_pipewire_setup(
    streams_meta: &[ashpd::desktop::screencast::Stream],
    proxy: ashpd::desktop::screencast::Screencast<'static>,
    kind: PortalSessionKind,
) -> Result<(PortalHold, PwSetup), CaptureError> {
    use ashpd::desktop::screencast::Stream;

    let mut streams: Vec<PwStreamSetup> = streams_meta
        .iter()
        .map(|s: &Stream| PwStreamSetup {
            node_id: s.pipe_wire_node_id(),
            rect: stream_rect(s),
        })
        .collect();
    let x11_layout = crate::x11_capture::query_x11_monitor_rects();
    let monitor_rects = shared_monitor_rects(&mut streams, &x11_layout);
    cap_log(
        "PORTAL",
        "pw",
        &format!(
            "layout=shared streams={} monitors={} x11={}",
            streams.len(),
            monitor_rects.len(),
            x11_layout.len()
        ),
    );
    for (i, s) in streams.iter().enumerate() {
        cap_log(
            "PORTAL",
            "pw",
            &format!(
                "stream {i} node={} rect={}x{}+{}+{}",
                s.node_id, s.rect.w, s.rect.h, s.rect.x, s.rect.y
            ),
        );
    }
    let virtual_bounds = monitor_rects
        .iter()
        .copied()
        .filter(|r| r.w > 0 && r.h > 0)
        .reduce(union_rect)
        .unwrap_or_default();
    let fd =
        match &kind {
            PortalSessionKind::Combined { session, .. } => proxy
                .open_pipe_wire_remote(session)
                .await
                .map_err(portal_err("open_pipe_wire_remote"))?,
            PortalSessionKind::ScreenCast { session } => proxy
                .open_pipe_wire_remote(session)
                .await
                .map_err(portal_err("open_pipe_wire_remote"))?,
        };
    let hold = match kind {
        PortalSessionKind::Combined { remote, session } => PortalHold::Combined {
            _remote: remote,
            _screencast: proxy,
            _session: session,
        },
        PortalSessionKind::ScreenCast { session } => PortalHold::ScreenCastOnly {
            _proxy: proxy,
            _session: session,
        },
    };
    Ok((
        hold,
        PwSetup {
            fd,
            virtual_bounds,
            monitor_rects,
            streams,
        },
    ))
}

async fn open_screencast_only_with_token(
    restore_token: Option<&str>,
) -> Result<(PortalHold, PwSetup), CaptureError> {
    use ashpd::desktop::screencast::Screencast;
    use ashpd::desktop::PersistMode;

    let proxy = Screencast::new()
        .await
        .map_err(portal_err("Screencast proxy"))?;
    let types = source_types(&proxy).await;
    let cursor = cursor_mode(&proxy).await;
    let session = proxy
        .create_session()
        .await
        .map_err(portal_err("create_session"))?;
    if restore_token.is_some() {
        cap_log("PORTAL", "token", "restoring");
    }
    proxy
        .select_sources(
            &session,
            cursor,
            types,
            true,
            restore_token,
            PersistMode::ExplicitlyRevoked,
        )
        .await
        .map_err(portal_err("select_sources"))?;
    let response = proxy
        .start(&session, None)
        .await
        .map_err(portal_err("start"))?
        .response()
        .map_err(portal_err("start response"))?;
    write_restore_token_at(&restore_token_path(), response.restore_token());
    let streams_meta = response.streams();
    if streams_meta.is_empty() {
        return Err(CaptureError::Message(
            "portal ScreenCast returned no streams".into(),
        ));
    }
    SCREENCAST_SESSION_GRANTED.store(true, Ordering::SeqCst);
    cap_log(
        "PORTAL",
        "grant",
        &format!(
            "streams={} cursor={cursor:?} persist={}",
            streams_meta.len(),
            response.restore_token().is_some()
        ),
    );
    finish_pipewire_setup(
        streams_meta,
        proxy,
        PortalSessionKind::ScreenCast { session },
    )
    .await
}

/// Place PipeWire streams on the shared-output layout.
///
/// Xinerama is only a position hint: the returned monitor list is the streams
/// themselves (what the user shared), never extra outputs the Sqyre window
/// happens to see.
fn shared_monitor_rects(
    streams: &mut [PwStreamSetup],
    x11_layout: &[DesktopRect],
) -> Vec<DesktopRect> {
    if x11_layout.len() >= 2 {
        assign_streams_to_layout(streams, x11_layout);
    } else {
        layout_stream_rects(streams);
    }
    let mut rects: Vec<DesktopRect> = streams
        .iter()
        .map(|s| s.rect)
        .filter(|r| r.w > 0 && r.h > 0)
        .collect();
    rects.sort_by_key(|r| (r.x, r.y, r.w, r.h));
    rects.dedup();
    rects
}

fn assign_streams_to_layout(streams: &mut [PwStreamSetup], layout: &[DesktopRect]) {
    if layout.is_empty() || streams.is_empty() {
        return;
    }
    let mut remaining: Vec<DesktopRect> = layout.to_vec();
    remaining.sort_by_key(|r| (r.x, r.y));
    for stream in streams.iter_mut() {
        let Some(idx) = take_layout_match(&remaining, stream.rect) else {
            continue;
        };
        stream.rect = remaining.remove(idx);
    }
    layout_stream_rects(streams);
}

fn take_layout_match(remaining: &[DesktopRect], hint: DesktopRect) -> Option<usize> {
    if hint.w > 0 && hint.h > 0 {
        if let Some(i) = remaining.iter().position(|d| *d == hint) {
            return Some(i);
        }
        let size_hits: Vec<usize> = remaining
            .iter()
            .enumerate()
            .filter(|(_, d)| d.w == hint.w && d.h == hint.h)
            .map(|(i, _)| i)
            .collect();
        if !size_hits.is_empty() {
            return size_hits.into_iter().min_by_key(|&i| {
                let d = remaining[i];
                d.x.abs_diff(hint.x) as u64 + d.y.abs_diff(hint.y) as u64
            });
        }
        return None;
    }
    remaining.first().map(|_| 0)
}

fn layout_stream_rects(streams: &mut [PwStreamSetup]) {
    if streams.len() < 2 {
        return;
    }
    let missing_or_overlap = streams.iter().any(|s| s.rect.w <= 0 || s.rect.h <= 0)
        || streams.iter().enumerate().any(|(i, a)| {
            streams
                .iter()
                .skip(i + 1)
                .any(|b| rects_overlap(a.rect, b.rect))
        });
    if !missing_or_overlap {
        return;
    }
    let mut x = 0;
    for stream in streams.iter_mut() {
        let w = stream.rect.w.max(1);
        let h = stream.rect.h.max(1);
        stream.rect = DesktopRect { x, y: 0, w, h };
        x = x.saturating_add(w);
    }
}

fn rects_overlap(a: DesktopRect, b: DesktopRect) -> bool {
    a.w > 0
        && a.h > 0
        && b.w > 0
        && b.h > 0
        && a.x < b.x + b.w
        && b.x < a.x + a.w
        && a.y < b.y + b.h
        && b.y < a.y + a.h
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

/// linux/dma-buf.h `DMA_BUF_IOCTL_SYNC` (`_IOW('b', 0, struct dma_buf_sync)`).
const DMA_BUF_IOCTL_SYNC: libc::c_ulong = 0x4008_6200;
const DMA_BUF_SYNC_READ: u64 = 1 << 0;
const DMA_BUF_SYNC_END: u64 = 1 << 2;

#[repr(C)]
struct DmaBufSync {
    flags: u64,
}

/// CPU-map coherency for GNOME ScreenCast DMA-BUF / memfd. Without START/END
/// the mapping often stays on the first GPU write for the whole wait-until-found.
struct SpaBufSync {
    fd: i32,
    active: bool,
}

impl SpaBufSync {
    fn begin(ty: DataType, fd: i32) -> Self {
        let active = fd >= 0 && matches!(ty, DataType::DmaBuf | DataType::MemFd);
        if active {
            dma_buf_sync(fd, DMA_BUF_SYNC_READ);
        }
        Self { fd, active }
    }
}

impl Drop for SpaBufSync {
    fn drop(&mut self) {
        if self.active {
            dma_buf_sync(self.fd, DMA_BUF_SYNC_READ | DMA_BUF_SYNC_END);
        }
    }
}

fn dma_buf_sync(fd: i32, flags: u64) {
    let mut sync = DmaBufSync { flags };
    // SAFETY: `fd` is the live spa_data fd; `sync` is `struct dma_buf_sync`.
    let _ = unsafe { libc::ioctl(fd, DMA_BUF_IOCTL_SYNC, &mut sync) };
}

struct MappedFd {
    ptr: *mut libc::c_void,
    len: usize,
}

impl MappedFd {
    fn as_slice(&self) -> &[u8] {
        // SAFETY: `ptr`/`len` come from a successful `mmap` of `len` bytes.
        unsafe { std::slice::from_raw_parts(self.ptr.cast(), self.len) }
    }
}

impl Drop for MappedFd {
    fn drop(&mut self) {
        if !self.ptr.is_null() && self.len > 0 {
            // SAFETY: mapping created by `mmap_spa_fd` and not yet unmapped.
            unsafe {
                libc::munmap(self.ptr, self.len);
            }
        }
    }
}

fn mmap_spa_fd(data: &pw::spa::buffer::Data) -> Option<MappedFd> {
    let raw = data.as_raw();
    let len = raw.maxsize as usize;
    let fd = data.fd();
    if len == 0 || fd < 0 {
        return None;
    }
    // SAFETY: `fd` is PipeWire's buffer fd; `mapoffset`/`maxsize` are spa_data.
    let ptr = unsafe {
        libc::mmap(
            std::ptr::null_mut(),
            len,
            libc::PROT_READ,
            libc::MAP_SHARED,
            fd,
            raw.mapoffset as libc::off_t,
        )
    };
    if ptr == libc::MAP_FAILED {
        return None;
    }
    Some(MappedFd { ptr, len })
}

fn chunk_byte_range(
    mapped_len: usize,
    offset: usize,
    size: usize,
) -> Option<std::ops::Range<usize>> {
    let end = offset.checked_add(size)?;
    (end <= mapped_len && size > 0).then_some(offset..end)
}

fn with_spa_chunk_bytes<T>(
    data: &mut pw::spa::buffer::Data,
    offset: usize,
    size: usize,
    f: impl FnOnce(&[u8]) -> T,
) -> Option<T> {
    let ty = data.type_();
    let fd = data.fd();
    let _sync = SpaBufSync::begin(ty, fd);
    if let Some(mapped) = data.data() {
        let range = chunk_byte_range(mapped.len(), offset, size)?;
        return Some(f(&mapped[range]));
    }
    let map = mmap_spa_fd(data)?;
    let range = chunk_byte_range(map.as_slice().len(), offset, size)?;
    Some(f(&map.as_slice()[range]))
}

#[allow(clippy::too_many_arguments)] // src frame + dest rect in one blit
fn copy_pw_frame_into_rect(
    src: &[u8],
    size: usize,
    src_stride: usize,
    src_w: u32,
    src_h: u32,
    format: VideoFormat,
    dst: &mut [u8],
    dst_stride: usize,
    dst_x: usize,
    dst_y: usize,
    dst_w: u32,
    dst_h: u32,
) -> Result<(), CaptureError> {
    if src_w == dst_w && src_h == dst_h {
        return copy_pw_frame_to_rgba_at(
            src, size, src_stride, src_w, src_h, format, dst, dst_stride, dst_x, dst_y,
        );
    }
    let mut tmp = vec![0u8; src_w as usize * src_h as usize * 4];
    copy_pw_frame_to_rgba_at(
        src,
        size,
        src_stride,
        src_w,
        src_h,
        format,
        &mut tmp,
        src_w as usize * 4,
        0,
        0,
    )?;
    let sw = src_w as usize;
    let sh = src_h as usize;
    let dw = dst_w as usize;
    let dh = dst_h as usize;
    for y in 0..dh {
        let sy = y * sh / dh;
        for x in 0..dw {
            let sx = x * sw / dw;
            let src_off = (sy * sw + sx) * 4;
            let dst_off = (dst_y + y) * dst_stride + (dst_x + x) * 4;
            if src_off + 4 > tmp.len() || dst_off + 4 > dst.len() {
                return Err(CaptureError::Message(
                    "portal capture: RGBA buffer too small".into(),
                ));
            }
            dst[dst_off..dst_off + 4].copy_from_slice(&tmp[src_off..src_off + 4]);
        }
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)] // row copy needs src/dst geometry in one pass
fn copy_pw_frame_to_rgba_at(
    src: &[u8],
    size: usize,
    src_stride: usize,
    width: u32,
    height: u32,
    format: VideoFormat,
    dst: &mut [u8],
    dst_stride: usize,
    dst_x: usize,
    dst_y: usize,
) -> Result<(), CaptureError> {
    let w = width as usize;
    let h = height as usize;
    for y in 0..h {
        let src_off = y * src_stride;
        if src_off >= size.min(src.len()) {
            break;
        }
        let row_len = src_stride.min(src.len().saturating_sub(src_off));
        let row = &src[src_off..src_off + row_len];
        let dst_row_off = (dst_y + y) * dst_stride + dst_x * 4;
        if dst_row_off + w * 4 > dst.len() {
            return Err(CaptureError::Message(
                "portal capture: RGBA buffer too small".into(),
            ));
        }
        let dst_row = &mut dst[dst_row_off..dst_row_off + w * 4];
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
    fn chunk_byte_range_skips_prefix_offset() {
        assert_eq!(chunk_byte_range(12, 4, 4), Some(4..8));
        assert_eq!(chunk_byte_range(8, 0, 8), Some(0..8));
        assert_eq!(chunk_byte_range(8, 6, 4), None);
        assert_eq!(chunk_byte_range(8, 0, 0), None);
    }

    #[test]
    fn restore_token_roundtrip() {
        let dir = std::env::temp_dir().join(format!(
            "sqyre-screencast-token-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("wayland-screencast.token");
        write_restore_token_at(&path, Some(" token-value \n"));
        assert_eq!(read_restore_token_at(&path).as_deref(), Some("token-value"));
        write_restore_token_at(&path, None);
        assert!(read_restore_token_at(&path).is_none());
        let _ = std::fs::remove_dir_all(dir);
    }

    fn empty_slot(generation: u64) -> FrameSlot {
        FrameSlot {
            cache: FrameCache {
                virtual_bounds: DesktopRect::default(),
                monitor_rects: Vec::new(),
                width: 0,
                height: 0,
                stride: 0,
                pixels: Vec::new(),
                ready: false,
            },
            generation,
            region_gen: Vec::new(),
        }
    }

    fn monitor_rect(x: i32, w: i32, h: i32) -> DesktopRect {
        DesktopRect { x, y: 0, w, h }
    }

    #[test]
    fn stream_cursor_maps_into_dest_rect() {
        let dest = DesktopRect {
            x: 1920,
            y: 0,
            w: 2560,
            h: 1440,
        };
        assert_eq!(
            stream_cursor_to_desktop(dest, 100, 50, 2560, 1440),
            (2020, 50)
        );
        assert_eq!(
            stream_cursor_to_desktop(dest, 1280, 720, 1280, 720),
            (4480, 1440)
        );
        assert_eq!(stream_cursor_to_desktop(dest, 10, 20, 0, 0), (1930, 20));
    }

    #[test]
    fn pipewire_cursor_and_buffer_pods_serialize() {
        let cursor = pw_cursor_meta_bytes().expect("cursor meta");
        let header = pw_header_meta_bytes().expect("header meta");
        let buffers = pw_buffers_param_bytes(8294400, 7680).expect("buffers");
        assert!(Pod::from_bytes(&cursor).is_some());
        assert!(Pod::from_bytes(&header).is_some());
        assert!(Pod::from_bytes(&buffers).is_some());
        assert!(cursor_meta_size(64, 64) > cursor_meta_size(1, 1));
    }

    #[test]
    fn region_generation_uses_overlapping_stream_only() {
        let mut slot = empty_slot(10);
        slot.region_gen = vec![
            (monitor_rect(0, 1920, 1080), 10),
            (monitor_rect(1920, 1280, 1440), 7),
        ];
        assert_eq!(region_generation(&slot, monitor_rect(1920, 1280, 1440)), 7);
        assert_eq!(region_generation(&slot, monitor_rect(0, 1920, 1080)), 10);
    }

    #[test]
    fn wait_until_region_after_ignores_other_monitor_copies() {
        let lock = Arc::new(Mutex::new(empty_slot(1)));
        {
            let mut slot = lock.lock();
            slot.region_gen = vec![
                (monitor_rect(0, 1920, 1080), 1),
                (monitor_rect(1920, 1280, 1440), 1),
            ];
        }
        let cvar = Arc::new(Condvar::new());
        let wait_lock = Arc::clone(&lock);
        let wait_cvar = Arc::clone(&cvar);
        let search = monitor_rect(1920, 1280, 1440);
        let handle = thread::spawn(move || {
            wait_until_region_after(&wait_lock, &wait_cvar, search, 1, Duration::from_secs(2))
        });
        thread::sleep(Duration::from_millis(20));
        {
            let mut slot = lock.lock();
            note_region_copy(&mut slot, monitor_rect(0, 1920, 1080));
            cvar.notify_all();
        }
        thread::sleep(Duration::from_millis(30));
        {
            let mut slot = lock.lock();
            note_region_copy(&mut slot, monitor_rect(1920, 1280, 1440));
            cvar.notify_all();
        }
        assert_eq!(handle.join().unwrap(), 3);
    }

    #[test]
    fn ensure_cache_grows_from_empty() {
        let mut cache = FrameCache {
            virtual_bounds: DesktopRect::default(),
            monitor_rects: Vec::new(),
            width: 0,
            height: 0,
            stride: 0,
            pixels: Vec::new(),
            ready: false,
        };
        ensure_cache_contains(
            &mut cache,
            DesktopRect {
                x: 0,
                y: 0,
                w: 10,
                h: 4,
            },
        );
        assert_eq!(cache.width, 10);
        assert_eq!(cache.height, 4);
        assert_eq!(cache.pixels.len(), 10 * 4 * 4);
    }

    fn ready_cache(w: u32, h: u32, pixels: Vec<u8>) -> FrameCache {
        FrameCache {
            virtual_bounds: DesktopRect {
                x: 0,
                y: 0,
                w: w as i32,
                h: h as i32,
            },
            monitor_rects: Vec::new(),
            width: w,
            height: h,
            stride: w as usize * 4,
            pixels,
            ready: true,
        }
    }

    #[test]
    fn copy_cache_rgb_skips_alpha() {
        let cache = ready_cache(2, 1, vec![1, 2, 3, 255, 4, 5, 6, 128]);
        let crop = cache_crop_geom(
            &cache,
            DesktopRect {
                x: 0,
                y: 0,
                w: 2,
                h: 1,
            },
        )
        .unwrap();
        let rgb = copy_cache_rgb(&cache, crop).unwrap();
        assert_eq!(rgb.width, 2);
        assert_eq!(rgb.height, 1);
        assert_eq!(rgb.data, vec![1, 2, 3, 4, 5, 6]);
    }

    #[test]
    fn copy_cache_rgb_crops_subrect() {
        // 2x2 RGBA: row0 (1,2,3,a)(4,5,6,a)  row1 (7,8,9,a)(10,11,12,a)
        let cache = ready_cache(
            2,
            2,
            vec![1, 2, 3, 9, 4, 5, 6, 9, 7, 8, 9, 9, 10, 11, 12, 9],
        );
        let crop = cache_crop_geom(
            &cache,
            DesktopRect {
                x: 1,
                y: 1,
                w: 1,
                h: 1,
            },
        )
        .unwrap();
        let rgb = copy_cache_rgb(&cache, crop).unwrap();
        assert_eq!(rgb.data, vec![10, 11, 12]);
        let rgba = copy_cache_rgba(&cache, crop).unwrap();
        assert_eq!(rgba.as_raw(), &[10, 11, 12, 9]);
    }

    #[test]
    fn assign_streams_uses_x11_y_offsets() {
        let mut streams = vec![
            PwStreamSetup {
                node_id: 10,
                rect: DesktopRect {
                    x: 0,
                    y: 0,
                    w: 1920,
                    h: 1080,
                },
            },
            PwStreamSetup {
                node_id: 11,
                rect: DesktopRect {
                    x: 1920,
                    y: 0,
                    w: 2560,
                    h: 1440,
                },
            },
        ];
        let layout = [
            DesktopRect {
                x: 0,
                y: 360,
                w: 1920,
                h: 1080,
            },
            DesktopRect {
                x: 1920,
                y: 0,
                w: 2560,
                h: 1440,
            },
        ];
        assign_streams_to_layout(&mut streams, &layout);
        assert_eq!(streams[0].rect.y, 360);
        assert_eq!(streams[1].rect.x, 1920);
        assert_eq!(streams[1].rect.y, 0);
    }

    #[test]
    fn shared_rects_follow_streams_not_full_x11_layout() {
        let mut streams = vec![PwStreamSetup {
            node_id: 11,
            rect: DesktopRect {
                x: 0,
                y: 0,
                w: 2560,
                h: 1440,
            },
        }];
        let layout = [
            DesktopRect {
                x: 0,
                y: 360,
                w: 1920,
                h: 1080,
            },
            DesktopRect {
                x: 1920,
                y: 0,
                w: 2560,
                h: 1440,
            },
        ];
        let rects = shared_monitor_rects(&mut streams, &layout);
        assert_eq!(rects.len(), 1);
        assert_eq!(rects[0].w, 2560);
        assert_eq!(rects[0].h, 1440);
        assert_eq!(rects[0].x, 1920);
    }

    #[test]
    fn ensure_cache_does_not_invent_monitors() {
        let mut cache = FrameCache {
            virtual_bounds: DesktopRect::default(),
            monitor_rects: vec![DesktopRect {
                x: 0,
                y: 0,
                w: 10,
                h: 4,
            }],
            width: 0,
            height: 0,
            stride: 0,
            pixels: Vec::new(),
            ready: false,
        };
        ensure_cache_contains(
            &mut cache,
            DesktopRect {
                x: 0,
                y: 0,
                w: 20,
                h: 4,
            },
        );
        assert_eq!(cache.monitor_rects.len(), 1);
        assert_eq!(cache.width, 20);
    }

    #[test]
    fn overlapping_streams_layout_ltr() {
        let mut streams = vec![
            PwStreamSetup {
                node_id: 1,
                rect: DesktopRect {
                    x: 0,
                    y: 0,
                    w: 1920,
                    h: 1080,
                },
            },
            PwStreamSetup {
                node_id: 2,
                rect: DesktopRect {
                    x: 0,
                    y: 0,
                    w: 1920,
                    h: 1080,
                },
            },
        ];
        layout_stream_rects(&mut streams);
        assert_eq!(streams[0].rect.x, 0);
        assert_eq!(streams[1].rect.x, 1920);
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

    #[test]
    fn composite_frame_at_offset() {
        let row = [0u8, 1, 2, 255, 10, 11, 12, 255];
        let mut dst = vec![0u8; 16];
        copy_pw_frame_to_rgba_at(
            &row,
            row.len(),
            8,
            2,
            1,
            VideoFormat::RGBA,
            &mut dst,
            8,
            1,
            0,
        )
        .unwrap();
        assert_eq!(&dst[4..8], &[0, 1, 2, 255]);
        assert_eq!(&dst[0..4], &[0, 0, 0, 0]);
    }

    #[test]
    fn scale_frame_into_smaller_rect() {
        let mut src = vec![0u8; 8];
        src[0..4].copy_from_slice(&[10, 20, 30, 255]);
        src[4..8].copy_from_slice(&[40, 50, 60, 255]);
        let mut dst = vec![0u8; 4];
        copy_pw_frame_into_rect(
            &src,
            src.len(),
            8,
            2,
            1,
            VideoFormat::RGBA,
            &mut dst,
            4,
            0,
            0,
            1,
            1,
        )
        .unwrap();
        assert_eq!(&dst, &[10, 20, 30, 255]);
    }
}
