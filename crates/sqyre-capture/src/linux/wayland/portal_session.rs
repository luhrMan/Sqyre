//! Portal ScreenCast / Remote Desktop session, restore tokens, and EIS input.

use crate::cap_log;
use crate::error::CaptureError;
use crate::linux::wayland::eis::EisInput;
use parking_lot::Mutex;
use sqyre_ports::{AutomationError, DesktopRect};
use std::os::fd::OwnedFd;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;
use std::time::{Duration, Instant};

static INPUT_TX: Mutex<Option<Sender<EisCmd>>> = Mutex::new(None);
pub(super) static PENDING_EIS_FD: Mutex<Option<OwnedFd>> = Mutex::new(None);
static LAST_ABS: Mutex<Option<(i32, i32)>> = Mutex::new(None);
static PORTAL_CURSOR: Mutex<Option<(i32, i32)>> = Mutex::new(None);
/// Last click/key/scroll over EIS — ScreenCast may still be catching up.
static LAST_STAGE_MUTATION: Mutex<Option<Instant>> = Mutex::new(None);
static LOGGED_PORTAL_CURSOR: AtomicBool = AtomicBool::new(false);
pub(super) static LOGGED_CURSOR_META: AtomicBool = AtomicBool::new(false);
static EIS_READY: AtomicBool = AtomicBool::new(false);
static EIS_SHUTDOWN: AtomicBool = AtomicBool::new(false);
pub(super) static REMOTE_DESKTOP_GRANTED: AtomicBool = AtomicBool::new(false);

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
    let reseat = *LAST_ABS.lock();
    let pos = reseat
        .map(|(x, y)| format!("{x},{y}"))
        .unwrap_or_else(|| "none".into());
    let (reply, rx) = mpsc::channel();
    send_eis(EisCmd::Click {
        button: code,
        down,
        reseat,
        reply,
    })?;
    let r = recv_eis(rx);
    let edge = if down { "down" } else { "up" };
    match &r {
        Ok(()) => {
            note_stage_mutation();
            cap_log(
                "INPUT",
                "ok",
                &format!(
                    "click={edge} button={button} pos={pos} reseat={}",
                    if reseat.is_some() { "yes" } else { "no" }
                ),
            );
        }
        Err(e) => {
            cap_log(
                "INPUT",
                "fail",
                &format!("click={edge} button={button} pos={pos} err={e}"),
            );
        }
    }
    r
}

pub fn portal_input_scroll(up: bool) -> Result<(), AutomationError> {
    let (reply, rx) = mpsc::channel();
    send_eis(EisCmd::Scroll { up, reply })?;
    let r = recv_eis(rx);
    if r.is_ok() {
        note_stage_mutation();
    }
    r
}

pub fn portal_input_key(evdev: u32, down: bool) -> Result<(), AutomationError> {
    let (reply, rx) = mpsc::channel();
    send_eis(EisCmd::Key { evdev, down, reply })?;
    let r = recv_eis(rx);
    if r.is_ok() {
        note_stage_mutation();
    }
    r
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
            reseat,
            reply,
        } => {
            let _ = reply.send(eis.click(button, down, reseat));
        }
        EisCmd::Scroll { up, reply } => {
            let _ = reply.send(eis.scroll(up));
        }
        EisCmd::Key { evdev, down, reply } => {
            let _ = reply.send(eis.key(evdev, down));
        }
    }
}

pub(super) fn stop_eis_thread() {
    EIS_SHUTDOWN.store(true, Ordering::SeqCst);
    EIS_READY.store(false, Ordering::SeqCst);
    REMOTE_DESKTOP_GRANTED.store(false, Ordering::SeqCst);
    *INPUT_TX.lock() = None;
    *LAST_ABS.lock() = None;
    *PORTAL_CURSOR.lock() = None;
    LOGGED_PORTAL_CURSOR.store(false, Ordering::Relaxed);
    LOGGED_CURSOR_META.store(false, Ordering::Relaxed);
}

pub(super) fn spawn_eis_thread(fd: OwnedFd) {
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
                            // Drain pause/resume so the next click sees current device state.
                            Err(mpsc::RecvTimeoutError::Timeout) => eis.drain(),
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
    forget_live_portal_grant();
    clear_restore_tokens();
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

/// Forget persistent ScreenCast / Remote Desktop grants and drop the live session.
///
/// Unlike [`request_portal_screencast_picker`], this does not reopen capture.
pub fn revoke_portal_grants() {
    let mut tokens = restore_tokens_on_disk();
    forget_live_portal_grant();
    clear_restore_tokens();
    crate::linux::reset_shared_capturer();
    // An in-flight `open()` may rewrite a token after the first clear; pick it up.
    forget_live_portal_grant();
    tokens.extend(restore_tokens_on_disk());
    tokens.sort();
    tokens.dedup();
    clear_restore_tokens();
    wipe_portal_permission_store(&tokens);
    forget_live_portal_grant();
    cap_log("PORTAL", "revoke", &format!("tokens={}", tokens.len()));
}

fn forget_live_portal_grant() {
    SCREENCAST_SESSION_GRANTED.store(false, Ordering::SeqCst);
    stop_eis_thread();
}

fn restore_tokens_on_disk() -> Vec<String> {
    [restore_token_path(), legacy_screencast_token_path()]
        .iter()
        .filter_map(|p| read_restore_token_at(p))
        .collect()
}

fn clear_restore_tokens() {
    write_restore_token_at(&restore_token_path(), None);
    write_restore_token_at(&legacy_screencast_token_path(), None);
}

/// Best-effort: drop persist entries keyed by restore token and `com.sqyre.app`.
fn wipe_portal_permission_store(tokens: &[String]) {
    let Ok(conn) = zbus::blocking::Connection::session() else {
        cap_log("PORTAL", "revoke", "permission-store: no session bus");
        return;
    };
    let Ok(proxy) = zbus::blocking::Proxy::new(
        &conn,
        "org.freedesktop.impl.portal.PermissionStore",
        "/org/freedesktop/impl/portal/PermissionStore",
        "org.freedesktop.impl.portal.PermissionStore",
    ) else {
        cap_log("PORTAL", "revoke", "permission-store: proxy failed");
        return;
    };
    const TABLES: &[&str] = &["remote-desktop", "screencast"];
    for table in TABLES {
        for token in tokens {
            // Not-found is the usual case after a local token clear.
            let _ = proxy.call::<_, _, ()>("Delete", &(table, token.as_str()));
        }
        let Ok(ids) = proxy.call::<_, _, Vec<String>>("List", &(table,)) else {
            continue;
        };
        for id in ids {
            let _ =
                proxy.call::<_, _, ()>("DeletePermission", &(table, id.as_str(), PORTAL_APP_ID));
        }
    }
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

pub(super) async fn open_portal_session() -> Result<(PortalHold, PwSetup), CaptureError> {
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
        CaptureError::Message(format!("portal {step}: {e}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
