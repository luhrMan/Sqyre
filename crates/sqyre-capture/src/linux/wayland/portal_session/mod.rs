//! Portal ScreenCast / Remote Desktop session, restore tokens, and EIS input.

use crate::error::CaptureError;
use crate::linux::wayland::eis::EisInput;
use parking_lot::Mutex;
use sqyre_ports::cap_log;
use sqyre_ports::{AutomationError, DesktopRect, APP_DIR_NAME};
use std::os::fd::OwnedFd;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;
use std::time::{Duration, Instant};

mod eis;
mod grants;
mod tokens;

pub use eis::{
    portal_cursor_position, portal_input_click, portal_input_key, portal_input_last_pos,
    portal_input_move, portal_input_ready, portal_input_scroll, portal_input_wait,
};
pub use grants::{
    portal_remote_desktop_granted, portal_screencast_granted, request_portal_screencast_picker,
    revoke_portal_grants,
};

static INPUT_TX: Mutex<Option<Sender<EisCmd>>> = Mutex::new(None);
static LAST_ABS: Mutex<Option<(i32, i32)>> = Mutex::new(None);
/// When [`LAST_ABS`] was last sent. A click soon after trusts it over a
/// ScreenCast cursor that has not caught up yet.
static LAST_ABS_AT: Mutex<Option<Instant>> = Mutex::new(None);
/// How long a click keeps the last warp if it disagrees with the live cursor.
const RESEAT_TRUST_WARP: Duration = Duration::from_millis(400);
static PORTAL_CURSOR: Mutex<Option<(i32, i32)>> = Mutex::new(None);
/// Last click/key/scroll over EIS — ScreenCast may still be catching up.
static LAST_STAGE_MUTATION: Mutex<Option<Instant>> = Mutex::new(None);
static LOGGED_PORTAL_CURSOR: AtomicBool = AtomicBool::new(false);
pub(super) static LOGGED_CURSOR_META: AtomicBool = AtomicBool::new(false);
static EIS_READY: AtomicBool = AtomicBool::new(false);
static EIS_SHUTDOWN: AtomicBool = AtomicBool::new(false);
/// Handshake thread is running and has not yet resumed a usable device or failed.
static EIS_CONNECTING: AtomicBool = AtomicBool::new(false);
/// `portal_input_wait` already spent its budget while still connecting.
static EIS_WAIT_EXHAUSTED: AtomicBool = AtomicBool::new(false);
static EIS_FAILURE: Mutex<Option<String>> = Mutex::new(None);
pub(super) static REMOTE_DESKTOP_GRANTED: AtomicBool = AtomicBool::new(false);

/// How long callers wait for COSMIC (and other compositors) to resume an EIS device
/// after Remote Desktop is granted. The first PipeWire frame can arrive before that.
const EIS_BECOME_READY_LIMIT: Duration = Duration::from_secs(6);

enum EisCmd {
    Move {
        x: i32,
        y: i32,
        reply: Sender<Result<(), AutomationError>>,
    },
    Click {
        button: u32,
        down: bool,
        /// Last absolute pointer we sent — framed with the button edge when possible.
        reseat: Option<(i32, i32)>,
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

pub(super) struct PwStreamSetup {
    pub(super) node_id: u32,
    pub(super) rect: DesktopRect,
}

pub(super) struct PwSetup {
    pub(super) fd: OwnedFd,
    pub(super) virtual_bounds: DesktopRect,
    pub(super) monitor_rects: Vec<DesktopRect>,
    pub(super) streams: Vec<PwStreamSetup>,
}

/// Holds portal DBus session open for the lifetime of the PipeWire loop.
pub(super) enum PortalHold {
    Combined {
        _remote: ashpd::desktop::remote_desktop::RemoteDesktop,
        _screencast: ashpd::desktop::screencast::Screencast,
        _session: ashpd::desktop::Session<ashpd::desktop::remote_desktop::RemoteDesktop>,
    },
    ScreenCastOnly {
        _proxy: ashpd::desktop::screencast::Screencast,
        _session: ashpd::desktop::Session<ashpd::desktop::screencast::Screencast>,
    },
}

/// Same id as `sqyre-app` (`com.sqyre.app.desktop`). GNOME persist keys off this.
const PORTAL_APP_ID: &str = "com.sqyre.app";

static FORCE_SCREENCAST_PICKER: AtomicBool = AtomicBool::new(false);
static SCREENCAST_SESSION_GRANTED: AtomicBool = AtomicBool::new(false);

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

pub(super) fn note_portal_cursor(
    dest: DesktopRect,
    lx: i32,
    ly: i32,
    stream_w: u32,
    stream_h: u32,
) {
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

/// Record that portal input may have changed on-screen pixels (ScreenCast lag).
pub(super) fn note_stage_mutation() {
    *LAST_STAGE_MUTATION.lock() = Some(Instant::now());
}

/// True if EIS click/key/scroll happened within `within` (ScreenCast may still catch up).
pub(super) fn stage_mutation_recent(within: Duration) -> bool {
    LAST_STAGE_MUTATION
        .lock()
        .map(|t| t.elapsed() <= within)
        .unwrap_or(false)
}

pub(super) fn stop_eis_thread() {
    EIS_SHUTDOWN.store(true, Ordering::SeqCst);
    EIS_READY.store(false, Ordering::SeqCst);
    REMOTE_DESKTOP_GRANTED.store(false, Ordering::SeqCst);
    eis::clear_eis_status();
    *INPUT_TX.lock() = None;
    *LAST_ABS.lock() = None;
    *LAST_ABS_AT.lock() = None;
    *PORTAL_CURSOR.lock() = None;
    LOGGED_PORTAL_CURSOR.store(false, Ordering::Relaxed);
    LOGGED_CURSOR_META.store(false, Ordering::Relaxed);
}

pub(super) fn spawn_eis_thread(fd: OwnedFd) {
    EIS_SHUTDOWN.store(false, Ordering::SeqCst);
    eis::clear_eis_status();
    EIS_CONNECTING.store(true, Ordering::SeqCst);
    cap_log("INPUT", "eis", "thread start");
    let spawned = thread::Builder::new()
        .name("sqyre-eis".into())
        .spawn(move || {
            let mut failure = None;
            match EisInput::connect(fd, &EIS_SHUTDOWN) {
                Ok(mut eis) => {
                    if !EIS_SHUTDOWN.load(Ordering::SeqCst) {
                        let (tx, rx) = mpsc::channel();
                        *INPUT_TX.lock() = Some(tx);
                        // COSMIC resumes devices when it handles the seat bind, which can
                        // land after this handshake returns. Stay connected until then.
                        eis::publish_eis_ready(&eis);
                        while !EIS_SHUTDOWN.load(Ordering::SeqCst) {
                            match rx.recv_timeout(Duration::from_millis(100)) {
                                Ok(cmd) => eis::dispatch_eis(&mut eis, cmd),
                                Err(mpsc::RecvTimeoutError::Timeout) => eis.drain(),
                                Err(mpsc::RecvTimeoutError::Disconnected) => break,
                            }
                            eis::publish_eis_ready(&eis);
                        }
                    }
                }
                Err(e) => {
                    if !EIS_SHUTDOWN.load(Ordering::SeqCst) {
                        failure = Some(format!("eis handshake: {e}"));
                    }
                }
            }
            if EIS_SHUTDOWN.load(Ordering::SeqCst) {
                *INPUT_TX.lock() = None;
                return;
            }
            EIS_READY.store(false, Ordering::SeqCst);
            EIS_CONNECTING.store(false, Ordering::SeqCst);
            *INPUT_TX.lock() = None;
            let err = failure.unwrap_or_else(|| "portal input thread exited".into());
            eis::note_eis_failure(err);
        });
    if let Err(e) = spawned {
        eis::note_eis_failure(format!("failed to start EIS thread: {e}"));
    }
}

fn stream_rect(stream: &ashpd::desktop::screencast::Stream) -> DesktopRect {
    let (w, h) = stream.size().unwrap_or((0, 0));
    let (x, y) = stream.position().unwrap_or((0, 0));
    DesktopRect { x, y, w, h }
}
enum HostRegister {
    /// This connection is associated with [`PORTAL_APP_ID`].
    Ready,
    /// Desktop file is not in GIO's cache yet (`App info not found`).
    Retry,
    /// No session bus, or another error. Do not open a portal method after this
    /// on a fresh connection — the next call caches an empty host id.
    Stop,
}

/// Classify a `Register` failure. The host portal keeps the first successful
/// method's app id for the life of the D-Bus connection.
fn host_register_outcome(err: &str) -> HostRegister {
    let lower = err.to_ascii_lowercase();
    if lower.contains("already associated") {
        HostRegister::Ready
    } else if lower.contains("not found") || lower.contains("could not find") {
        HostRegister::Retry
    } else {
        HostRegister::Stop
    }
}

async fn register_portal_app_once() -> HostRegister {
    let Ok(app_id) = PORTAL_APP_ID.parse::<ashpd::AppID>() else {
        return HostRegister::Stop;
    };
    match ashpd::register_host_app(app_id).await {
        Ok(()) => {
            cap_log("PORTAL", "appid", PORTAL_APP_ID);
            HostRegister::Ready
        }
        Err(e) => {
            let msg = e.to_string();
            match host_register_outcome(&msg) {
                HostRegister::Ready => {
                    cap_log("PORTAL", "appid", "already associated");
                    HostRegister::Ready
                }
                HostRegister::Retry => HostRegister::Retry,
                HostRegister::Stop => {
                    cap_log("PORTAL", "appid", &format!("register skipped: {msg}"));
                    HostRegister::Stop
                }
            }
        }
    }
}

async fn register_portal_app() {
    if matches!(register_portal_app_once().await, HostRegister::Retry) {
        cap_log(
            "PORTAL",
            "appid",
            &format!("register skipped: {PORTAL_APP_ID} not in desktop cache"),
        );
    }
}

/// Register `com.sqyre.app` before any other portal method on this process.
///
/// xdg-desktop-portal caches the app id from the first method on the
/// connection. GlobalShortcuts (started with the UI) otherwise caches an empty
/// id, and the macro ScreenCast / RemoteDesktop dialog stays "unknown application".
/// GIO indexes a just-written desktop file asynchronously; retry that case only.
pub fn register_portal_app_now() {
    const ATTEMPTS: u32 = 8;
    for attempt in 0..ATTEMPTS {
        match pollster::block_on(register_portal_app_once()) {
            HostRegister::Ready | HostRegister::Stop => return,
            HostRegister::Retry => {
                if attempt + 1 == ATTEMPTS {
                    cap_log(
                        "PORTAL",
                        "appid",
                        &format!("register skipped: {PORTAL_APP_ID} not in desktop cache"),
                    );
                    return;
                }
                thread::sleep(Duration::from_millis(50));
            }
        }
    }
}

async fn cursor_mode(
    proxy: &ashpd::desktop::screencast::Screencast,
) -> ashpd::desktop::screencast::CursorMode {
    use ashpd::desktop::screencast::CursorMode;
    let available = proxy.available_cursor_modes().await;
    let mode = match &available {
        Ok(modes) if modes.contains(CursorMode::METADATA) => CursorMode::METADATA,
        Ok(modes) if modes.contains(CursorMode::HIDDEN) => CursorMode::HIDDEN,
        Ok(modes) if modes.contains(CursorMode::EMBEDDED) => CursorMode::EMBEDDED,
        _ => CursorMode::HIDDEN,
    };
    cap_log(
        "PORTAL",
        "cursor",
        &format!("available={available:?} selected={mode:?}"),
    );
    mode
}

async fn source_types(
    proxy: &ashpd::desktop::screencast::Screencast,
) -> ashpd::desktop::screencast::SourceType {
    use ashpd::desktop::screencast::SourceType;
    match proxy.available_source_types().await {
        Ok(types) if types.contains(SourceType::MONITOR) => SourceType::MONITOR,
        Ok(types) if types.contains(SourceType::VIRTUAL) => SourceType::VIRTUAL,
        Ok(types) if !types.is_empty() => types,
        _ => SourceType::MONITOR,
    }
}

pub(super) async fn open_portal_session() -> Result<(PortalHold, PwSetup), CaptureError> {
    cap_log("PORTAL", "start", "interface=RemoteDesktop+ScreenCast");
    register_portal_app().await;

    let stored = if grants::take_force_screencast_picker() {
        cap_log("PORTAL", "picker", "forcing dialog");
        None
    } else {
        tokens::read_restore_token_at(&tokens::restore_token_path())
    };
    match open_combined_session_with_token(stored.as_deref()).await {
        Ok(ok) => Ok(ok),
        Err(e) if stored.is_some() => {
            cap_log(
                "PORTAL",
                "token",
                &format!("restore failed ({e}); retrying with picker"),
            );
            tokens::write_restore_token_at(&tokens::restore_token_path(), None);
            open_combined_session_with_token(None).await
        }
        Err(e) => Err(e),
    }
}

async fn open_combined_session_with_token(
    restore_token: Option<&str>,
) -> Result<(PortalHold, PwSetup), CaptureError> {
    use ashpd::desktop::remote_desktop::{
        ConnectToEISOptions, DeviceType, RemoteDesktop, SelectDevicesOptions, StartOptions,
    };
    use ashpd::desktop::screencast::{Screencast, SelectSourcesOptions};
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
        .create_session(ashpd::desktop::CreateSessionOptions::default())
        .await
        .map_err(portal_err("create_session"))?;
    if restore_token.is_some() {
        cap_log("PORTAL", "token", "restoring");
    }
    remote
        .select_devices(
            &session,
            SelectDevicesOptions::default()
                .set_devices(DeviceType::KEYBOARD | DeviceType::POINTER)
                .set_restore_token(restore_token)
                .set_persist_mode(PersistMode::ExplicitlyRevoked),
        )
        .await
        .map_err(portal_err("select_devices"))?;
    proxy
        .select_sources(
            &session,
            SelectSourcesOptions::default()
                .set_cursor_mode(cursor)
                .set_sources(types)
                .set_multiple(true)
                .set_persist_mode(PersistMode::DoNot),
        )
        .await
        .map_err(portal_err("select_sources"))?;
    let response = remote
        .start(&session, None, StartOptions::default())
        .await
        .map_err(portal_err("start"))?
        .response()
        .map_err(portal_err("start response"))?;
    tokens::write_restore_token_at(&tokens::restore_token_path(), response.restore_token());
    let has_input = response.devices().contains(DeviceType::KEYBOARD)
        || response.devices().contains(DeviceType::POINTER);
    REMOTE_DESKTOP_GRANTED.store(has_input, Ordering::SeqCst);
    let streams_meta = response.streams();
    grants::grant_screencast_streams(streams_meta)?;
    cap_log(
        "PORTAL",
        "grant",
        &format!(
            "streams={} input={has_input} cursor={cursor:?} persist={}",
            streams_meta.len(),
            response.restore_token().is_some()
        ),
    );
    // Handshake before PipeWire setup. COSMIC can deliver a frame on the first
    // iterate, and sampling EIS then used to report a granted session as not ready.
    let spawned = if has_input {
        match remote
            .connect_to_eis(&session, ConnectToEISOptions::default())
            .await
        {
            Ok(fd) => {
                cap_log("INPUT", "eis", "ConnectToEIS fd");
                spawn_eis_thread(fd);
                true
            }
            Err(e) => {
                eis::note_eis_failure(format!("ConnectToEIS: {e}"));
                false
            }
        }
    } else {
        cap_log(
            "INPUT",
            "fail",
            "reason=no_remote_interaction enable Allow Remote Interaction in the share dialog",
        );
        eis::clear_eis_status();
        false
    };
    let opened = finish_pipewire_setup(
        streams_meta,
        proxy,
        PortalSessionKind::Combined { remote, session },
    )
    .await;
    if opened.is_err() && spawned {
        stop_eis_thread();
    }
    opened
}

enum PortalSessionKind {
    Combined {
        remote: ashpd::desktop::remote_desktop::RemoteDesktop,
        session: ashpd::desktop::Session<ashpd::desktop::remote_desktop::RemoteDesktop>,
    },
    ScreenCast {
        session: ashpd::desktop::Session<ashpd::desktop::screencast::Screencast>,
    },
}

async fn finish_pipewire_setup(
    streams_meta: &[ashpd::desktop::screencast::Stream],
    proxy: ashpd::desktop::screencast::Screencast,
    kind: PortalSessionKind,
) -> Result<(PortalHold, PwSetup), CaptureError> {
    use ashpd::desktop::screencast::{OpenPipeWireRemoteOptions, Stream};

    let mut streams: Vec<PwStreamSetup> = streams_meta
        .iter()
        .map(|s: &Stream| PwStreamSetup {
            node_id: s.pipe_wire_node_id(),
            rect: stream_rect(s),
        })
        .collect();
    // COSMIC advertises logical positions and physical sizes. Sqyre's desktop
    // (and the PipeWire frame) is physical, so shift the origin before the
    // overlap pack — a vertical pair overlaps in logical space and would
    // otherwise be laid out side by side.
    if sqyre_ports::cosmic_desktop_session() {
        place_cosmic_streams(
            &mut streams,
            &super::compositor_kick::snapshot_output_geoms(),
        );
    }
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
    // Partial share (restore token or picker) leaves unshared outputs uncapturable
    // and used to shrink the selection cover — reject and clear tokens so the next
    // open re-prompts for every display.
    let usable_x11 = x11_layout.iter().filter(|r| r.w > 1 && r.h > 1).count();
    if usable_x11 >= 2 && streams.len() < usable_x11 {
        tokens::clear_restore_tokens();
        return Err(CaptureError::Message(format!(
            "portal shared {} monitor(s) but desktop has {usable_x11}; share all displays",
            streams.len()
        )));
    }
    let virtual_bounds = monitor_rects
        .iter()
        .copied()
        .filter(|r| r.w > 0 && r.h > 0)
        .reduce(union_rect)
        .unwrap_or_default();
    let fd = match &kind {
        PortalSessionKind::Combined { session, .. } => proxy
            .open_pipe_wire_remote(session, OpenPipeWireRemoteOptions::default())
            .await
            .map_err(portal_err("open_pipe_wire_remote"))?,
        PortalSessionKind::ScreenCast { session } => proxy
            .open_pipe_wire_remote(session, OpenPipeWireRemoteOptions::default())
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
    use ashpd::desktop::screencast::{Screencast, SelectSourcesOptions, StartCastOptions};
    use ashpd::desktop::PersistMode;

    let proxy = Screencast::new()
        .await
        .map_err(portal_err("Screencast proxy"))?;
    let types = source_types(&proxy).await;
    let cursor = cursor_mode(&proxy).await;
    let session = proxy
        .create_session(ashpd::desktop::CreateSessionOptions::default())
        .await
        .map_err(portal_err("create_session"))?;
    if restore_token.is_some() {
        cap_log("PORTAL", "token", "restoring");
    }
    proxy
        .select_sources(
            &session,
            SelectSourcesOptions::default()
                .set_cursor_mode(cursor)
                .set_sources(types)
                .set_multiple(true)
                .set_restore_token(restore_token)
                .set_persist_mode(PersistMode::ExplicitlyRevoked),
        )
        .await
        .map_err(portal_err("select_sources"))?;
    let response = proxy
        .start(&session, None, StartCastOptions::default())
        .await
        .map_err(portal_err("start"))?
        .response()
        .map_err(portal_err("start response"))?;
    tokens::write_restore_token_at(&tokens::restore_token_path(), response.restore_token());
    let streams_meta = response.streams();
    grants::grant_screencast_streams(streams_meta)?;
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

/// Move a COSMIC stream from its logical portal position onto the physical
/// pixel grid when `wl_output` scale is greater than 1 and the stream size is
/// the hardware mode. A logical-sized stream (GNOME) does not match the mode
/// and is left alone.
fn place_cosmic_stream(
    rect: DesktopRect,
    outputs: &[super::compositor_kick::WlOutputGeom],
) -> DesktopRect {
    let Some(output) = outputs.iter().find(|output| {
        output.scale > 1
            && output.x == rect.x
            && output.y == rect.y
            && output.width == rect.w
            && output.height == rect.h
    }) else {
        return rect;
    };
    DesktopRect {
        x: output.x.saturating_mul(output.scale),
        y: output.y.saturating_mul(output.scale),
        w: rect.w,
        h: rect.h,
    }
}

fn place_cosmic_streams(
    streams: &mut [PwStreamSetup],
    outputs: &[super::compositor_kick::WlOutputGeom],
) {
    for stream in streams.iter_mut() {
        stream.rect = place_cosmic_stream(stream.rect, outputs);
    }
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

pub(super) fn rects_overlap(a: DesktopRect, b: DesktopRect) -> bool {
    a.w > 0
        && a.h > 0
        && b.w > 0
        && b.h > 0
        && a.x < b.x + b.w
        && b.x < a.x + a.w
        && a.y < b.y + b.h
        && b.y < a.y + a.h
}

pub(super) fn union_rect(a: DesktopRect, b: DesktopRect) -> DesktopRect {
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
        CaptureError::Portal(format!("{step}: {e}"))
    }
}

#[cfg(test)]
mod tests;
