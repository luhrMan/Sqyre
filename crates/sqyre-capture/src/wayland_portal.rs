//! XDG Desktop Portal helpers for Wayland (ashpd + pollster, async-io zbus).
//!
//! Screen-capture *permission* uses ScreenCast (persist when available). Frame
//! pixels use the Screenshot portal cropped to [`DesktopRect`]. A PipeWire
//! stream consumer can replace the screenshot path when libpipewire is available.
//!
//! RemoteDesktop input is owned by a background thread so the portal session
//! stays alive across inject calls without `'static` transmute.

use crate::wayland_permissions;
use crate::CaptureError;
use ashpd::desktop::global_shortcuts::{GlobalShortcuts, NewShortcut};
use ashpd::desktop::remote_desktop::{
    DeviceType, KeyState, RemoteDesktop, SelectDevicesOptions,
};
use ashpd::desktop::screencast::{
    CursorMode, Screencast, SelectSourcesOptions, SourceType,
};
use ashpd::desktop::screenshot::Screenshot;
use ashpd::desktop::PersistMode;
use ashpd::enumflags2::BitFlags;
use image::RgbaImage;
use parking_lot::Mutex;
use sqyre_ports::DesktopRect;
use std::fs;
use std::path::PathBuf;
use std::sync::mpsc::{self, SyncSender};
use std::thread;

fn block_on<F: std::future::Future>(fut: F) -> F::Output {
    pollster::block_on(fut)
}

fn portal_err(e: impl std::fmt::Display) -> CaptureError {
    CaptureError::PortalUnavailable(e.to_string())
}

static MONITOR_RECTS: Mutex<Vec<DesktopRect>> = Mutex::new(Vec::new());
static LAST_BOUNDS: Mutex<Option<DesktopRect>> = Mutex::new(None);

fn set_monitor_layout(rects: Vec<DesktopRect>) {
    if let Some(bounds) = union_rects(&rects) {
        *LAST_BOUNDS.lock() = Some(bounds);
    }
    *MONITOR_RECTS.lock() = rects;
}

fn union_rects(rects: &[DesktopRect]) -> Option<DesktopRect> {
    let mut iter = rects.iter().filter(|r| r.w > 0 && r.h > 0);
    let first = iter.next()?;
    let mut min_x = first.x;
    let mut min_y = first.y;
    let mut max_x = first.x + first.w;
    let mut max_y = first.y + first.h;
    for r in iter {
        min_x = min_x.min(r.x);
        min_y = min_y.min(r.y);
        max_x = max_x.max(r.x + r.w);
        max_y = max_y.max(r.y + r.h);
    }
    Some(DesktopRect {
        x: min_x,
        y: min_y,
        w: (max_x - min_x).max(1),
        h: (max_y - min_y).max(1),
    })
}

/// One-shot ScreenCast: user picks monitors; we store layout for coordinates.
pub fn ensure_screencast_session() -> Result<(), CaptureError> {
    if !wayland_permissions::screen_capture_enabled() {
        return Err(CaptureError::PermissionDenied {
            capability: "screen capture",
        });
    }
    if !MONITOR_RECTS.lock().is_empty() {
        return Ok(());
    }
    let rects = block_on(async {
        let proxy = Screencast::new().await.map_err(portal_err)?;
        let session = proxy
            .create_session(Default::default())
            .await
            .map_err(portal_err)?;
        proxy
            .select_sources(
                &session,
                SelectSourcesOptions::default()
                    .set_cursor_mode(CursorMode::Embedded)
                    .set_sources(BitFlags::from_flag(SourceType::Monitor))
                    .set_multiple(true)
                    .set_persist_mode(PersistMode::Application),
            )
            .await
            .map_err(portal_err)?;
        let response = proxy
            .start(&session, None, Default::default())
            .await
            .map_err(portal_err)?
            .response()
            .map_err(portal_err)?;
        let mut rects = Vec::new();
        for stream in response.streams() {
            let (w, h) = stream.size().unwrap_or((0, 0));
            let (x, y) = stream.position().unwrap_or((0, 0));
            if w > 0 && h > 0 {
                rects.push(DesktopRect { x, y, w, h });
            }
        }
        // Session ends when `session` drops; layout + Screenshot portal remain usable.
        Ok::<_, CaptureError>(rects)
    })?;
    if rects.is_empty() {
        return Err(CaptureError::PortalUnavailable(
            "ScreenCast returned no monitor streams".into(),
        ));
    }
    set_monitor_layout(rects);
    Ok(())
}

/// Clear cached ScreenCast layout (settings toggle / re-request).
pub fn drop_screencast_session() {
    MONITOR_RECTS.lock().clear();
    *LAST_BOUNDS.lock() = None;
}

pub fn virtual_bounds() -> Result<DesktopRect, CaptureError> {
    if let Some(b) = *LAST_BOUNDS.lock() {
        return Ok(b);
    }
    ensure_screencast_session()?;
    if let Some(b) = *LAST_BOUNDS.lock() {
        return Ok(b);
    }
    let img = capture_full_screenshot()?;
    let bounds = DesktopRect {
        x: 0,
        y: 0,
        w: img.width() as i32,
        h: img.height() as i32,
    };
    *LAST_BOUNDS.lock() = Some(bounds);
    Ok(bounds)
}

pub fn monitor_rects() -> Result<Vec<DesktopRect>, CaptureError> {
    ensure_screencast_session()?;
    let rects = MONITOR_RECTS.lock().clone();
    if rects.is_empty() {
        Ok(vec![virtual_bounds()?])
    } else {
        Ok(rects)
    }
}

fn capture_full_screenshot() -> Result<RgbaImage, CaptureError> {
    if !wayland_permissions::screen_capture_enabled() {
        return Err(CaptureError::PermissionDenied {
            capability: "screen capture",
        });
    }
    let response = block_on(async {
        Screenshot::request()
            .interactive(false)
            .modal(false)
            .send()
            .await
            .map_err(portal_err)?
            .response()
            .map_err(portal_err)
    })?;
    let path = uri_to_path(response.uri().as_str())?;
    let bytes =
        fs::read(&path).map_err(|e| CaptureError::Message(format!("read screenshot: {e}")))?;
    let img = image::load_from_memory(&bytes)
        .map_err(|e| CaptureError::Message(format!("decode screenshot: {e}")))?
        .to_rgba8();
    let _ = fs::remove_file(&path);
    Ok(img)
}

fn uri_to_path(uri: &str) -> Result<PathBuf, CaptureError> {
    let path = uri
        .strip_prefix("file://")
        .ok_or_else(|| CaptureError::Message(format!("unexpected screenshot URI: {uri}")))?;
    Ok(PathBuf::from(percent_decode(path)))
}

fn percent_decode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let (Some(h), Some(l)) = (
                (bytes[i + 1] as char).to_digit(16),
                (bytes[i + 2] as char).to_digit(16),
            ) {
                out.push(char::from(((h << 4) | l) as u8));
                i += 3;
                continue;
            }
        }
        out.push(bytes[i] as char);
        i += 1;
    }
    out
}

/// Capture `rect` via Screenshot portal + crop into virtual-desktop coords.
pub fn capture_rect(rect: DesktopRect) -> Result<RgbaImage, CaptureError> {
    if rect.is_empty() {
        return Err(CaptureError::EmptyRect);
    }
    let _ = ensure_screencast_session();
    let full = capture_full_screenshot()?;
    let bounds = *LAST_BOUNDS.lock().get_or_insert(DesktopRect {
        x: 0,
        y: 0,
        w: full.width() as i32,
        h: full.height() as i32,
    });
    let sx = (rect.x - bounds.x).max(0);
    let sy = (rect.y - bounds.y).max(0);
    let sw = rect.w.min(full.width() as i32 - sx).max(0);
    let sh = rect.h.min(full.height() as i32 - sy).max(0);
    if sw <= 0 || sh <= 0 {
        return Err(CaptureError::OutsideVirtualDesktop);
    }
    Ok(image::imageops::crop_imm(&full, sx as u32, sy as u32, sw as u32, sh as u32).to_image())
}

pub fn request_screen_capture() -> Result<(), CaptureError> {
    drop_screencast_session();
    ensure_screencast_session()
}

pub fn request_input_control() -> Result<(), CaptureError> {
    if !wayland_permissions::input_control_enabled() {
        return Err(CaptureError::PermissionDenied {
            capability: "input control",
        });
    }
    wayland_input_session::ensure_session()
}

pub fn request_global_shortcuts() -> Result<(), CaptureError> {
    if !wayland_permissions::global_shortcuts_enabled() {
        return Err(CaptureError::PermissionDenied {
            capability: "global shortcuts",
        });
    }
    wayland_shortcuts_session::ensure_session()
}

/// Run first-start permission flow: ScreenCast → RemoteDesktop → GlobalShortcuts.
pub fn request_all_permissions() -> WaylandPermissionResults {
    let screen = request_screen_capture();
    let input = request_input_control();
    let shortcuts = request_global_shortcuts();
    WaylandPermissionResults {
        screen_capture: screen.is_ok(),
        input_control: input.is_ok(),
        global_shortcuts: shortcuts.is_ok(),
        screen_error: screen.err().map(|e| e.to_string()),
        input_error: input.err().map(|e| e.to_string()),
        shortcuts_error: shortcuts.err().map(|e| e.to_string()),
    }
}

/// Outcomes of the first-run / re-request permission wizard.
#[derive(Debug, Clone, Default)]
pub struct WaylandPermissionResults {
    pub screen_capture: bool,
    pub input_control: bool,
    pub global_shortcuts: bool,
    pub screen_error: Option<String>,
    pub input_error: Option<String>,
    pub shortcuts_error: Option<String>,
}

/// RemoteDesktop worker (session stays on a dedicated thread).
pub mod wayland_input_session {
    use super::*;

    enum Cmd {
        MotionAbs { stream: u32, x: f64, y: f64 },
        Button { button: i32, pressed: bool },
        Axis { dx: f64, dy: f64 },
        Keysym { keysym: i32, pressed: bool },
        Shutdown,
    }

    struct Reply(Result<(), CaptureError>);

    type CmdTx = SyncSender<(Cmd, SyncSender<Reply>)>;

    static TX: Mutex<Option<CmdTx>> = Mutex::new(None);

    pub fn ensure_session() -> Result<(), CaptureError> {
        if TX.lock().is_some() {
            return Ok(());
        }
        let (cmd_tx, cmd_rx) = mpsc::sync_channel::<(Cmd, SyncSender<Reply>)>(32);
        let (ready_tx, ready_rx) = mpsc::sync_channel::<Result<(), CaptureError>>(1);
        thread::Builder::new()
            .name("sqyre-wayland-input".into())
            .spawn(move || {
                let started = block_on(async {
                    let remote = RemoteDesktop::new().await.map_err(portal_err)?;
                    let screencast = Screencast::new().await.map_err(portal_err)?;
                    let session = remote
                        .create_session(Default::default())
                        .await
                        .map_err(portal_err)?;
                    remote
                        .select_devices(
                            &session,
                            SelectDevicesOptions::default()
                                .set_devices(DeviceType::Keyboard | DeviceType::Pointer),
                        )
                        .await
                        .map_err(portal_err)?;
                    let _ = screencast
                        .select_sources(
                            &session,
                            SelectSourcesOptions::default()
                                .set_cursor_mode(CursorMode::Embedded)
                                .set_sources(BitFlags::from_flag(SourceType::Monitor))
                                .set_multiple(true)
                                .set_persist_mode(PersistMode::Application),
                        )
                        .await;
                    let response = remote
                        .start(&session, None, Default::default())
                        .await
                        .map_err(portal_err)?
                        .response()
                        .map_err(portal_err)?;
                    // Prefer first PipeWire node id when ScreenCast streams are present.
                    let stream_id = response
                        .streams()
                        .first()
                        .map(|s| s.pipe_wire_node_id())
                        .unwrap_or(0);
                    Ok::<_, CaptureError>((remote, session, stream_id))
                });
                let (remote, session, default_stream) = match started {
                    Ok(v) => {
                        let _ = ready_tx.send(Ok(()));
                        v
                    }
                    Err(e) => {
                        let _ = ready_tx.send(Err(e));
                        return;
                    }
                };
                while let Ok((cmd, reply_tx)) = cmd_rx.recv() {
                    let result = match cmd {
                        Cmd::Shutdown => {
                            let _ = reply_tx.send(Reply(Ok(())));
                            break;
                        }
                        Cmd::MotionAbs { stream, x, y } => block_on(async {
                            remote
                                .notify_pointer_motion_absolute(
                                    &session,
                                    if stream == 0 { default_stream } else { stream },
                                    x,
                                    y,
                                    Default::default(),
                                )
                                .await
                                .map_err(portal_err)
                        }),
                        Cmd::Button { button, pressed } => block_on(async {
                            let ks = if pressed {
                                KeyState::Pressed
                            } else {
                                KeyState::Released
                            };
                            remote
                                .notify_pointer_button(&session, button, ks, Default::default())
                                .await
                                .map_err(portal_err)
                        }),
                        Cmd::Axis { dx, dy } => block_on(async {
                            remote
                                .notify_pointer_axis(&session, dx, dy, Default::default())
                                .await
                                .map_err(portal_err)
                        }),
                        Cmd::Keysym { keysym, pressed } => block_on(async {
                            let ks = if pressed {
                                KeyState::Pressed
                            } else {
                                KeyState::Released
                            };
                            remote
                                .notify_keyboard_keysym(&session, keysym, ks, Default::default())
                                .await
                                .map_err(portal_err)
                        }),
                    };
                    let _ = reply_tx.send(Reply(result));
                }
            })
            .map_err(|e| CaptureError::Message(format!("spawn input thread: {e}")))?;
        ready_rx
            .recv()
            .map_err(|_| CaptureError::PortalUnavailable("input thread died".into()))??;
        *TX.lock() = Some(cmd_tx);
        Ok(())
    }

    pub fn drop_session() {
        if let Some(tx) = TX.lock().take() {
            let (r_tx, r_rx) = mpsc::sync_channel(1);
            let _ = tx.send((Cmd::Shutdown, r_tx));
            let _ = r_rx.recv_timeout(std::time::Duration::from_secs(2));
        }
    }

    fn call(cmd: Cmd) -> Result<(), CaptureError> {
        ensure_session()?;
        let tx = TX.lock();
        let tx = tx.as_ref().ok_or(CaptureError::PermissionDenied {
            capability: "input control",
        })?;
        let (r_tx, r_rx) = mpsc::sync_channel(1);
        tx.send((cmd, r_tx))
            .map_err(|_| CaptureError::PortalUnavailable("input thread gone".into()))?;
        r_rx.recv()
            .map_err(|_| CaptureError::PortalUnavailable("input reply missing".into()))?
            .0
    }

    pub fn notify_pointer_motion_absolute(x: f64, y: f64) -> Result<(), CaptureError> {
        call(Cmd::MotionAbs {
            stream: 0,
            x,
            y,
        })
    }

    pub fn notify_pointer_button(button: i32, pressed: bool) -> Result<(), CaptureError> {
        call(Cmd::Button { button, pressed })
    }

    pub fn notify_pointer_axis(dx: f64, dy: f64) -> Result<(), CaptureError> {
        call(Cmd::Axis { dx, dy })
    }

    pub fn notify_keyboard_keysym(keysym: i32, pressed: bool) -> Result<(), CaptureError> {
        call(Cmd::Keysym { keysym, pressed })
    }
}

/// GlobalShortcuts portal (bind on demand).
pub mod wayland_shortcuts_session {
    use super::*;

    static BOUND: Mutex<bool> = Mutex::new(false);

    pub fn ensure_session() -> Result<(), CaptureError> {
        // Creating a session alone is enough to verify portal availability.
        block_on(async {
            let proxy = GlobalShortcuts::new().await.map_err(portal_err)?;
            let _session = proxy
                .create_session(Default::default())
                .await
                .map_err(portal_err)?;
            Ok(())
        })
    }

    pub fn drop_session() {
        *BOUND.lock() = false;
    }

    /// Bind preferred triggers; user confirms in the portal UI.
    pub fn bind_shortcuts(
        shortcuts: &[(String, String, Option<String>)],
    ) -> Result<(), CaptureError> {
        block_on(async {
            let proxy = GlobalShortcuts::new().await.map_err(portal_err)?;
            let session = proxy
                .create_session(Default::default())
                .await
                .map_err(portal_err)?;
            let list: Vec<NewShortcut> = shortcuts
                .iter()
                .map(|(id, desc, trigger)| {
                    let mut s = NewShortcut::new(id.clone(), desc.clone());
                    if let Some(t) = trigger {
                        s = s.preferred_trigger(t.as_str());
                    }
                    s
                })
                .collect();
            proxy
                .bind_shortcuts(&session, &list, None, Default::default())
                .await
                .map_err(portal_err)?
                .response()
                .map_err(portal_err)?;
            *BOUND.lock() = true;
            // Keep session alive by leaking for process lifetime (portal binds persist).
            std::mem::forget(session);
            std::mem::forget(proxy);
            Ok(())
        })
    }

    pub fn is_bound() -> bool {
        *BOUND.lock()
    }
}
