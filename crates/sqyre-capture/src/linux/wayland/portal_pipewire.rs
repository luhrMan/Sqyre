//! PipeWire stream connect and frame wait for portal ScreenCast.

use super::portal_dma::{copy_pw_frame_into_rect, with_spa_chunk_bytes};
use super::portal_session::{
    note_portal_cursor, open_portal_session, rects_overlap, spawn_eis_thread, stop_eis_thread,
    union_rect, LOGGED_CURSOR_META, PENDING_EIS_FD, REMOTE_DESKTOP_GRANTED,
};
use crate::cap_log;
use crate::error::CaptureError;
use crate::linux::session::{LinuxCaptureBackend, LinuxSessionInfo};
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
use sqyre_ports::{DesktopRect, RgbCapture};
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, Sender, SyncSender};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

/// Overall budget to obtain a PipeWire frame newer than the cache at call start.
const FRESH_CAPTURE_BUDGET: Duration = Duration::from_millis(800);
/// Post-kick wait slices (short first so a miss retries sooner than a full 120ms stall).
/// Emit-on-damage streams (games/Wine) often need the kick; continuous streams usually
/// land in the first slice. Multi-monitor deliveries that miss the search region also
/// benefit from re-pulsing instead of sitting out a long wait.
const POST_KICK_SLICES: &[Duration] = &[
    Duration::from_millis(40),
    Duration::from_millis(80),
    Duration::from_millis(120),
];
/// Log successful fresh waits that exceed this (stderr + diag when enabled).
const SLOW_FRESH_LOG: Duration = Duration::from_millis(100);

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

    /// Wait until a PipeWire stream that overlaps `rect` copies a frame newer than
    /// the cache at the start of this call. Pulses a transparent damage overlay
    /// (map+unmap) before each wait — leaving it mapped during the wait timed out
    /// on emit-on-damage games and could crop an overlay-tainted buffer.
    ///
    /// Latency is dominated by kick XSync + PipeWire frame arrival, not crop size:
    /// ~1 frame after a successful kick (~40–80ms) vs multiple pulse retries (~200–300ms)
    /// when the stream is idle or another monitor updates first.
    fn wait_for_overlapping_stream_frame(&self, rect: DesktopRect) {
        let min_gen = {
            let slot = self.frame.0.lock();
            region_generation(&slot, rect)
        };
        let kick_rect = {
            let slot = self.frame.0.lock();
            kick_damage_rect(&slot, rect)
        };
        let started = Instant::now();
        let deadline = started + FRESH_CAPTURE_BUDGET;
        let mut pulses = 0u32;
        while Instant::now() < deadline {
            self.pulse_compositor_kick(kick_rect);
            pulses = pulses.saturating_add(1);
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                break;
            }
            let slice_idx = (pulses as usize)
                .saturating_sub(1)
                .min(POST_KICK_SLICES.len() - 1);
            let slice = remaining.min(POST_KICK_SLICES[slice_idx]);
            let region_after =
                wait_until_region_after(&self.frame.0, &self.frame.1, rect, min_gen, slice);
            if region_after > min_gen {
                let elapsed = started.elapsed();
                if elapsed >= SLOW_FRESH_LOG {
                    cap_log(
                        "PORTAL",
                        "fresh",
                        &format!(
                            "wait_ms={} pulses={} gen={}->{}",
                            elapsed.as_millis(),
                            pulses,
                            min_gen,
                            region_after
                        ),
                    );
                }
                return;
            }
        }

        let (now, region) = {
            let slot = self.frame.0.lock();
            (slot.generation, region_generation(&slot, rect))
        };
        cap_log(
            "PORTAL",
            "wait",
            &format!(
                "fresh timeout after gen={min_gen} now={now} region={region} pulses={pulses} wait_ms={}",
                started.elapsed().as_millis()
            ),
        );
    }

    fn pulse_compositor_kick(&self, rect: DesktopRect) {
        let mut kick = self.kick.lock();
        if kick.is_none() {
            *kick = crate::x11_capture::CompositorKick::open();
        }
        if let Some(k) = kick.as_mut() {
            k.pulse_rect(rect);
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
        self.capture_rect_rgb_ref(rect)
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

/// Damage region for a compositor kick: the search crop, clamped into the
/// overlapping monitor (full-monitor overlays were slow and covered the wait).
fn kick_damage_rect(slot: &FrameSlot, rect: DesktopRect) -> DesktopRect {
    let dest = overlapping_stream_dest(slot, rect);
    let left = rect.x.max(dest.x);
    let top = rect.y.max(dest.y);
    let right = (rect.x + rect.w).min(dest.x + dest.w);
    let bottom = (rect.y + rect.h).min(dest.y + dest.h);
    if right - left < 2 || bottom - top < 2 {
        dest
    } else {
        DesktopRect {
            x: left,
            y: top,
            w: right - left,
            h: bottom - top,
        }
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use pw::spa::pod::Pod;
    use std::sync::Arc;
    use std::thread;
    use std::time::Duration;

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
    fn kick_damage_rect_clamps_to_overlapping_monitor() {
        let mut slot = empty_slot(1);
        let mon = monitor_rect(1920, 1280, 1440);
        slot.cache.monitor_rects = vec![monitor_rect(0, 1920, 1080), mon];
        let search = DesktopRect {
            x: 2000,
            y: 100,
            w: 400,
            h: 50,
        };
        let kick = kick_damage_rect(&slot, search);
        assert_eq!(kick.x, 2000);
        assert_eq!(kick.y, 100);
        assert_eq!(kick.w, 400);
        assert_eq!(kick.h, 50);
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
}
