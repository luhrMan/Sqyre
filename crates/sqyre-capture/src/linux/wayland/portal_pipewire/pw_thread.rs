//! PipeWire worker thread for portal ScreenCast streams.

use super::frame_slot::FrameSlot;
use super::*;
use crate::error::CaptureError;
use crate::linux::wayland::portal_dma::{copy_pw_frame_into_rect, with_spa_chunk_bytes};
use crate::linux::wayland::portal_session::{
    note_portal_cursor, open_portal_session, stop_eis_thread, LOGGED_CURSOR_META,
    REMOTE_DESKTOP_GRANTED,
};
use parking_lot::{Condvar, Mutex};
use pipewire as pw;
use pw::context::ContextRc;
use pw::main_loop::MainLoopRc;
use pw::properties::properties;
use pw::spa::buffer::{meta::MetaCursor, ChunkFlags};
use pw::spa::param::video::VideoInfoRaw;
use pw::spa::pod::Pod;
use pw::stream::StreamRc;
use sqyre_ports::{cap_log, mark_site};
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, SyncSender};
use std::sync::Arc;
use std::time::{Duration, Instant};

pub(super) fn portal_pw_thread(
    frame: Arc<(Mutex<FrameSlot>, Condvar)>,
    shutdown: Arc<AtomicBool>,
    stream_dead: Arc<AtomicBool>,
    ready: SyncSender<Result<(), CaptureError>>,
    msg_rx: Receiver<PwThreadMsg>,
) -> Result<(), CaptureError> {
    STREAMING.store(0, Ordering::SeqCst);
    let _clear_streaming = ClearStreaming;
    let ready = Arc::new(ready);
    let (portal_hold, setup) = propagate_ready(&ready, pollster::block_on(open_portal_session()))?;

    {
        let mut slot = frame.0.lock();
        slot.cache.virtual_bounds = setup.virtual_bounds;
        slot.cache.monitor_rects = setup.monitor_rects.clone();
        frame_slot::ensure_cache_contains(&mut slot.cache, setup.virtual_bounds);
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

    spa_params::ensure_spa_plugin_dir();
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
    let dead_cb = Arc::clone(&stream_dead);
    let first_ready = Arc::new(AtomicBool::new(false));
    let logged_unmap = Arc::new(AtomicBool::new(false));
    let logged_copy = Arc::new(AtomicBool::new(false));
    let logged_type = Arc::new(AtomicBool::new(false));
    // One bit per silent frame-drop reason so each logs exactly once — a benign
    // early dequeue-empty must not mask a later corrupted/size-0 drop.
    let logged_skip = Arc::new(AtomicU32::new(0));
    let ready_cb = Arc::clone(&ready);

    let values: Vec<u8> = propagate_ready(&ready, spa_params::pw_video_enum_format_bytes())?;
    let cursor_meta: Vec<u8> = propagate_ready(&ready, spa_params::pw_cursor_meta_bytes())?;
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

        let props =
            screencast_stream_props(serial.as_deref(), SUSPEND_ON_IDLE.load(Ordering::SeqCst));

        let stream = propagate_ready(
            &ready,
            StreamRc::new(core.clone(), &format!("sqyre-screencast-{index}"), props)
                .map_err(|e| CaptureError::Message(format!("PipeWire stream: {e}"))),
        )?;

        let frame_stream = Arc::clone(&frame_cb);
        let shutdown_stream = Arc::clone(&shutdown_cb);
        let dead_state = Arc::clone(&dead_cb);
        let saw_streaming = Arc::new(AtomicBool::new(false));
        let saw_streaming_state = Arc::clone(&saw_streaming);
        let ready_state = Arc::clone(&ready_cb);
        let first_ready_stream = Arc::clone(&first_ready);
        let logged_unmap_stream = Arc::clone(&logged_unmap);
        let logged_copy_stream = Arc::clone(&logged_copy);
        let logged_type_stream = Arc::clone(&logged_type);
        let logged_skip_stream = Arc::clone(&logged_skip);
        let ready_stream = Arc::clone(&ready_cb);
        let listener = propagate_ready(
            &ready,
            stream
                .add_local_listener_with_user_data(UserData {
                    format: VideoInfoRaw::default(),
                    monitor_rect: stream_setup.rect,
                })
                .state_changed(move |stream, _, old, new| {
                    cap_log(
                        "PORTAL",
                        "pw",
                        &format!("stream {index} {old:?} -> {new:?}"),
                    );
                    account_stream_state(&old, &new);
                    match &new {
                        // Pause-on-idle stops the node. Reactivate so the next frame
                        // is delivered. With suspend disabled this is a state flip;
                        // with it enabled the node may still be restarting its driver.
                        pw::stream::StreamState::Paused => {
                            if let Err(e) = stream.set_active(true) {
                                cap_log(
                                    "PORTAL",
                                    "pw",
                                    &format!("stream {index} resume failed: {e}"),
                                );
                            }
                        }
                        pw::stream::StreamState::Streaming => {
                            saw_streaming_state.store(true, Ordering::SeqCst);
                        }
                        pw::stream::StreamState::Error(e) => {
                            dead_state.store(true, Ordering::SeqCst);
                            let _ = ready_state.try_send(Err(CaptureError::Portal(format!(
                                "PipeWire stream {index} failed: {e}"
                            ))));
                        }
                        pw::stream::StreamState::Unconnected
                            if saw_streaming_state.load(Ordering::SeqCst) =>
                        {
                            // Producer dropped the stream after it streamed.
                            dead_state.store(true, Ordering::SeqCst);
                            let _ = ready_state.try_send(Err(CaptureError::Portal(format!(
                                "PipeWire stream {index} disconnected"
                            ))));
                        }
                        _ => {}
                    }
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
                            "format={:?} {}x{} fps={}/{} max={}/{} modifier={:#x}",
                            user_data.format.format(),
                            size.width,
                            size.height,
                            fps.num,
                            fps.denom,
                            max_fps.num,
                            max_fps.denom,
                            user_data.format.modifier()
                        ),
                    );
                    // An empty size is an intermediate pod. Advertising a 1×1
                    // buffer fails to intersect COSMIC's real frame size and
                    // errors that stream while a sibling stream stays up.
                    if size.width == 0 || size.height == 0 {
                        cap_log("PORTAL", "pw", &format!("stream {index} size empty"));
                        return;
                    }
                    let width = size.width as i32;
                    let height = size.height as i32;
                    let stride = width.saturating_mul(4);
                    let bytes = stride.saturating_mul(height);
                    match spa_params::apply_stream_buffer_params(stream, bytes, stride) {
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
                    let skip_once = |bit: u32, reason: &'static str| {
                        if logged_skip_stream.fetch_or(bit, Ordering::SeqCst) & bit == 0 {
                            cap_log("PORTAL", "pw", &format!("skip={reason}"));
                        }
                    };
                    let dest = user_data.monitor_rect;
                    let stream_size = user_data.format.size();
                    let Some(mut buffer) = stream.dequeue_buffer() else {
                        skip_once(1, "dequeue-empty");
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
                        skip_once(2, "datas-empty");
                        return;
                    }
                    let data = &mut datas[0];
                    if data.chunk().flags().contains(ChunkFlags::CORRUPTED) {
                        skip_once(4, "corrupted");
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
                        skip_once(8, "size-0");
                        return;
                    }
                    // FPS diag: stream 0 only (one monitor) so dual-head does not
                    // double the reported portal_fps. Count even when !buffering.
                    if index == 0 {
                        super::note_stream0_frame();
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
                        if !slot.buffering {
                            // Still dequeue/consume the PW buffer above; skip the
                            // full-desktop CPU mirror while idle after a macro.
                            return Ok(());
                        }
                        frame_slot::ensure_cache_contains(&mut slot.cache, dest);
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
                            frame_slot::note_region_copy(&mut slot, dest);
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

    while !shutdown.load(Ordering::SeqCst) {
        match msg_rx.try_recv() {
            Ok(PwThreadMsg::Quit) => break,
            Ok(PwThreadMsg::SetSuspendOnIdle(suspend)) => {
                apply_suspend_property(&stream_holds, suspend);
            }
            Err(mpsc::TryRecvError::Empty) => {}
            Err(mpsc::TryRecvError::Disconnected) => break,
        }
        // pw_loop_iterate returns <0 on error (-errno); without this a hard
        // failure would spin forever while captures starve.
        let res = loop_ref.iterate(pw::loop_::Timeout::Finite(Duration::from_millis(20)));
        if res < 0 && res != -libc::EINTR {
            stream_dead.store(true, Ordering::SeqCst);
            return Err(CaptureError::Portal(format!(
                "PipeWire main loop iterate failed ({res})"
            )));
        }
    }

    drop(listeners);
    drop(stream_holds);
    stop_eis_thread();
    REMOTE_DESKTOP_GRANTED.store(false, Ordering::SeqCst);
    // Session Drop may call portal Close over D-Bus (often ~1–2s on GNOME).
    // On process exit, forget the hold — disconnect tears the session down.
    if sqyre_ports::process_exiting() {
        mark_site("portal:pw:forget_session");
        std::mem::forget(portal_hold);
    } else {
        mark_site("portal:pw:before_session_drop");
        let t = Instant::now();
        drop(portal_hold);
        cap_log(
            "PORTAL",
            "pw",
            &format!("session_drop_ms={}", t.elapsed().as_millis()),
        );
    }

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

pub(super) fn pw_collect_node_serials(
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

pub(super) struct ClearStreaming;

impl Drop for ClearStreaming {
    fn drop(&mut self) {
        STREAMING.store(0, Ordering::SeqCst);
        STREAMING_SINCE.store(0, Ordering::SeqCst);
    }
}

pub(super) fn apply_suspend_property(streams: &[StreamRc], suspend: bool) {
    let value = if suspend { "true" } else { "false" };
    let props = properties! {
        "node.suspend-on-idle" => value,
    };
    for stream in streams {
        // SAFETY: `props` owns the dict for this call. Each stream was created
        // on this PipeWire thread and is still held. The C function copies the
        // dict items before it returns.
        let rc =
            unsafe { pw_stream_update_properties(stream.as_raw_ptr(), props.dict().as_raw_ptr()) };
        if rc < 0 {
            cap_log(
                "PORTAL",
                "pw",
                &format!("suspend-on-idle={value} update failed ({rc})"),
            );
        }
        if !suspend {
            if let Err(e) = stream.set_active(true) {
                cap_log(
                    "PORTAL",
                    "pw",
                    &format!("resume after suspend off failed: {e}"),
                );
            }
        }
    }
    cap_log(
        "PORTAL",
        "pw",
        &format!("suspend-on-idle={value} streams={}", streams.len()),
    );
}

/// Stream properties for a portal ScreenCast consumer.
///
/// `suspend_on_idle`: after the desktop sits still, PipeWire suspends an idle
/// node. The next capture then waits out driver restart before the first
/// buffer arrives. Off keeps the node paused so `set_active` resumes in one cycle.
pub(super) fn screencast_stream_props(
    target_serial: Option<&str>,
    suspend_on_idle: bool,
) -> pw::properties::PropertiesBox {
    let suspend = if suspend_on_idle { "true" } else { "false" };
    let mut props = properties! {
        *pw::keys::MEDIA_TYPE => "Video",
        *pw::keys::MEDIA_CATEGORY => "Capture",
        *pw::keys::MEDIA_ROLE => "Screen",
        "node.suspend-on-idle" => suspend,
    };
    if let Some(serial) = target_serial {
        props.insert("target.object", serial);
    }
    props
}
