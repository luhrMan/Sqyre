//! Screen capture in absolute virtual-desktop coordinates.

mod diag;
mod error;
#[cfg(not(any(target_os = "linux", target_os = "windows")))]
mod grab_stub;
#[cfg(target_os = "linux")]
pub mod linux;
mod outline_geometry;
mod outline_rect;
#[cfg(not(any(target_os = "linux", target_os = "windows")))]
mod outline_stub;
mod pixel_convert;
mod selection_grab;
#[macro_use]
mod shared_run;
mod stub;
#[cfg(target_os = "windows")]
mod win_capture;
#[cfg(target_os = "windows")]
mod win_focus;
#[cfg(target_os = "windows")]
mod win_grab;
#[cfg(target_os = "windows")]
mod win_outline;
mod window_match;
#[cfg(target_os = "linux")]
mod x11_capture;
#[cfg(target_os = "linux")]
mod x11_errors;
#[cfg(target_os = "linux")]
mod x11_focus;
#[cfg(target_os = "linux")]
mod x11_grab;
#[cfg(target_os = "linux")]
mod x11_outline;
#[cfg(target_os = "linux")]
mod x11_secondary;
#[cfg(target_os = "linux")]
mod x11_snapshot_overlay;

pub use diag::{
    cap_log, disk_logging_enabled, event_log, mark_site, note, read_last_site, set_disk_logging,
    set_log_dir, CRASH_LOG_FILE, DIAG_LOG_FILE, LAST_SITE_FILE,
};
pub use error::{linux_session_capture_warning, CaptureError};
#[cfg(target_os = "linux")]
pub use linux::{
    reset_shared_capturer, shared_capturer, shared_capturer_if_ready, shared_capturer_is_opening,
    shared_capturer_open_superseded, LinuxCaptureBackend, LinuxSessionInfo, LinuxSessionKind,
    OsCapturer, SharedRunCapturer,
};
pub use outline_rect::OutlineRect;
pub use pixel_convert::{zpixmap_to_rgb, zpixmap_to_rgba};
pub use selection_grab::GrabPoll;
pub use stub::{NullCapturer, SolidCapturer};

#[cfg(target_os = "windows")]
pub use win_capture::{
    reset_shared_capturer, shared_capturer, shared_capturer_if_ready, shared_capturer_is_opening,
    shared_capturer_open_superseded, OsCapturer, SharedRunCapturer,
};

#[cfg(target_os = "linux")]
pub use linux::wayland::OsWindowFocuser;

#[cfg(target_os = "windows")]
pub use win_focus::OsWindowFocuser;

#[cfg(target_os = "linux")]
pub use x11_outline::SelectionOutline;

#[cfg(target_os = "windows")]
pub use win_outline::SelectionOutline;

#[cfg(target_os = "linux")]
pub use x11_grab::SelectionGrab;

#[cfg(target_os = "linux")]
pub use x11_snapshot_overlay::{FrozenFrame, FrozenSelectionOverlay};

#[cfg(target_os = "windows")]
pub use win_grab::SelectionGrab;

/// True if `display` is a Sqyre secondary X11 connection (for winit error hooks).
#[cfg(target_os = "linux")]
pub fn owns_secondary_x_display(display: *mut std::ffi::c_void) -> bool {
    x11_secondary::owns(display)
}

#[cfg(not(any(target_os = "linux", target_os = "windows")))]
pub use outline_stub::SelectionOutline;

#[cfg(not(any(target_os = "linux", target_os = "windows")))]
pub use grab_stub::SelectionGrab;

/// macOS / other: capture not implemented yet.
#[cfg(not(any(target_os = "linux", target_os = "windows")))]
pub type OsCapturer = NullCapturer;

#[cfg(not(any(target_os = "linux", target_os = "windows")))]
pub fn shared_capturer() -> Result<std::sync::Arc<OsCapturer>, CaptureError> {
    Err(CaptureError::UnsupportedPlatform)
}

#[cfg(not(any(target_os = "linux", target_os = "windows")))]
pub fn shared_capturer_if_ready() -> Option<Result<std::sync::Arc<OsCapturer>, CaptureError>> {
    None
}

#[cfg(not(any(target_os = "linux", target_os = "windows")))]
pub fn shared_capturer_is_opening() -> bool {
    false
}

#[cfg(not(any(target_os = "linux", target_os = "windows")))]
pub fn reset_shared_capturer() {}

#[cfg(not(any(target_os = "linux", target_os = "windows")))]
pub fn shared_capturer_open_superseded() -> bool {
    false
}

#[cfg(not(any(target_os = "linux", target_os = "windows")))]
pub struct SharedRunCapturer(pub std::sync::Arc<OsCapturer>);

#[cfg(not(any(target_os = "linux", target_os = "windows")))]
impl sqyre_ports::ScreenCapturer for SharedRunCapturer {
    fn capture_monitor(
        &mut self,
        _display_index: i32,
    ) -> Result<image::RgbaImage, sqyre_ports::CaptureError> {
        Err(sqyre_ports::CaptureError::UnsupportedPlatform)
    }
    fn capture_rect(
        &mut self,
        _rect: sqyre_ports::DesktopRect,
    ) -> Result<image::RgbaImage, sqyre_ports::CaptureError> {
        Err(sqyre_ports::CaptureError::UnsupportedPlatform)
    }
    fn virtual_bounds(&mut self) -> Result<sqyre_ports::DesktopRect, sqyre_ports::CaptureError> {
        Ok(sqyre_ports::DesktopRect {
            x: 0,
            y: 0,
            w: 1,
            h: 1,
        })
    }
}

/// RGBA icon extracted from an OS window or executable.
#[derive(Debug, Clone)]
pub struct ProcessIcon {
    pub width: u32,
    pub height: u32,
    /// Unmultiplied RGBA, `width * height * 4` bytes.
    pub rgba: Vec<u8>,
}

/// One top-level application window for Focus Window picker UI.
#[derive(Debug, Clone)]
pub struct WindowInfo {
    pub title: String,
    pub process_name: String,
    pub process_path: String,
    /// Best-effort OS icon (filled by [`list_open_windows`] when available).
    pub icon: Option<ProcessIcon>,
}

impl PartialEq for WindowInfo {
    fn eq(&self, other: &Self) -> bool {
        self.title == other.title
            && self.process_name == other.process_name
            && self.process_path == other.process_path
    }
}

impl Eq for WindowInfo {}

impl WindowInfo {
    /// Human-readable list line: `title  (name — path)`.
    pub fn label(&self) -> String {
        let title = self.title.trim();
        let title = if title.is_empty() {
            "(untitled)"
        } else {
            title
        };
        let name = self.process_name.trim();
        let path = self.process_path.trim();
        match (name.is_empty(), path.is_empty()) {
            (false, false) => format!("{title}  ({name} — {path})"),
            (false, true) => format!("{title}  ({name})"),
            (true, false) => format!("{title}  ({path})"),
            (true, true) => title.to_string(),
        }
    }
}

/// Preferred pixel size when picking among multi-resolution OS icons.
pub const PROCESS_ICON_TARGET_PX: u32 = 48;

/// Best-effort icon for a bound process (`process_path` + optional `window_title`).
///
/// Linux: `_NET_WM_ICON` from a matching open window. Windows: icon resource from the
/// executable (works even when the app is not running). Other platforms: always `None`.
#[cfg(target_os = "linux")]
pub fn process_icon(process_path: &str, window_title: &str) -> Option<ProcessIcon> {
    x11_focus::process_icon(process_path, window_title)
}

#[cfg(target_os = "windows")]
pub fn process_icon(process_path: &str, window_title: &str) -> Option<ProcessIcon> {
    win_focus::process_icon(process_path, window_title)
}

#[cfg(not(any(target_os = "linux", target_os = "windows")))]
pub fn process_icon(_process_path: &str, _window_title: &str) -> Option<ProcessIcon> {
    None
}

/// Xinerama / RandR output rects (physical layout). Independent of how many
/// ScreenCast streams the portal currently has open.
#[cfg(target_os = "linux")]
fn physical_monitor_rects() -> Vec<sqyre_ports::DesktopRect> {
    use sqyre_ports::DesktopRect;
    use std::sync::Mutex;
    use std::time::{Duration, Instant};

    static CACHE: Mutex<Option<(Instant, Vec<DesktopRect>)>> = Mutex::new(None);
    if let Ok(g) = CACHE.lock() {
        if let Some((at, rects)) = g.as_ref() {
            if at.elapsed() < Duration::from_millis(500) {
                return rects.clone();
            }
        }
    }
    let rects = crate::x11_capture::query_x11_monitor_rects();
    if let Ok(mut g) = CACHE.lock() {
        *g = Some((Instant::now(), rects.clone()));
    }
    rects
}

fn usable_desktop_rects(
    rects: impl IntoIterator<Item = sqyre_ports::DesktopRect>,
) -> Vec<sqyre_ports::DesktopRect> {
    let mut usable: Vec<sqyre_ports::DesktopRect> =
        rects.into_iter().filter(|r| r.w > 1 && r.h > 1).collect();
    usable.sort_by_key(|r| (r.x, r.y, r.w, r.h));
    usable.dedup();
    usable
}

/// Live ScreenCast/X11 capturer rects, preferring Linux Xinerama when it reports
/// more outputs than the portal currently has (e.g. one stream failed to connect).
pub fn preferred_monitor_rects() -> Vec<sqyre_ports::DesktopRect> {
    let capture = shared_capturer_nonblocking()
        .ok()
        .and_then(|c| c.monitor_rects_ref().ok())
        .map(usable_desktop_rects)
        .unwrap_or_default();
    #[cfg(target_os = "linux")]
    let x11 = usable_desktop_rects(physical_monitor_rects());
    #[cfg(not(target_os = "linux"))]
    let x11: Vec<sqyre_ports::DesktopRect> = Vec::new();
    if x11.len() > capture.len() {
        x11
    } else {
        capture
    }
}

/// Leftmost live monitor resolution key (`"{w}x{h}"`).
/// Uses capturer monitor rects sorted by position (shared outputs on portal),
/// not whichever screen the Sqyre window is on.
/// Returns `None` when no display is available (headless / CI).
///
/// Does not block on a portal ScreenCast picker: if opening may block and the
/// capturer is not ready yet, returns `None`.
pub fn main_monitor_resolution_key() -> Option<String> {
    let capturer = shared_capturer_nonblocking().ok()?;
    let mut rects = capturer.monitor_rects_ref().ok()?;
    rects.retain(|r| r.w > 0 && r.h > 0);
    rects.sort_by_key(|r| (r.x, r.y, r.w, r.h));
    let r = rects.first()?;
    Some(format!("{}x{}", r.w, r.h))
}

/// True when opening [`shared_capturer`] may block on a portal ScreenCast permission dialog.
#[cfg(target_os = "linux")]
pub fn shared_capturer_open_may_block() -> bool {
    linux::LinuxSessionInfo::detect().shared_capturer_open_may_block()
}

#[cfg(not(target_os = "linux"))]
pub fn shared_capturer_open_may_block() -> bool {
    false
}

/// ScreenCast Start succeeded, a restore token is stored, or the live capturer is open.
pub fn portal_screencast_granted() -> bool {
    #[cfg(all(target_os = "linux", feature = "portal-capture"))]
    {
        if linux::wayland::portal_screencast_granted() {
            return true;
        }
    }
    matches!(shared_capturer_if_ready(), Some(Ok(_)))
}

/// Remote Desktop pointer/keyboard granted on the live combined portal session.
pub fn portal_remote_desktop_granted() -> bool {
    #[cfg(all(target_os = "linux", feature = "portal-capture"))]
    {
        linux::wayland::portal_remote_desktop_granted()
    }
    #[cfg(not(all(target_os = "linux", feature = "portal-capture")))]
    {
        false
    }
}

/// EIS input backend is connected (Wayland combined portal session).
pub fn portal_input_ready() -> bool {
    #[cfg(all(target_os = "linux", feature = "portal-capture"))]
    {
        linux::wayland::portal_input_ready()
    }
    #[cfg(not(all(target_os = "linux", feature = "portal-capture")))]
    {
        false
    }
}

#[cfg(all(target_os = "linux", feature = "portal-capture"))]
pub use linux::wayland::{
    portal_cursor_position, portal_input_click, portal_input_key, portal_input_last_pos,
    portal_input_move, portal_input_scroll,
};

/// Compositor pointer from ScreenCast cursor metadata (Wayland). `None` on other
/// targets, without portal-capture, or before the first metadata sample.
#[cfg(not(all(target_os = "linux", feature = "portal-capture")))]
pub fn portal_cursor_position() -> Option<(i32, i32)> {
    None
}

/// Show the portal ScreenCast picker again (Wayland). No-op on other targets.
pub fn request_portal_screencast_picker() {
    #[cfg(all(target_os = "linux", feature = "portal-capture"))]
    {
        linux::wayland::request_portal_screencast_picker();
    }
}

/// [`shared_capturer`] unless that would wait on a portal picker from this thread.
///
/// Use from the UI thread. The deferred Linux probe (or a worker) should call
/// [`shared_capturer`] to start the open.
pub fn shared_capturer_nonblocking() -> Result<std::sync::Arc<OsCapturer>, CaptureError> {
    if shared_capturer_open_may_block() {
        match shared_capturer_if_ready() {
            Some(r) => r,
            None if shared_capturer_is_opening() || portal_screencast_granted() => {
                Err(CaptureError::Message(
                    "screen capture is starting (waiting for the first frame)".into(),
                ))
            }
            None => Err(CaptureError::Message(
                "screen capture is still waiting for portal permission".into(),
            )),
        }
    } else {
        shared_capturer()
    }
}

/// Primary monitor DPI scale factor (`dpi / 96`).
/// Returns `None` when no display is available (headless / CI).
pub fn main_monitor_scale() -> Option<f32> {
    #[cfg(target_os = "linux")]
    {
        x11_capture::primary_monitor_scale()
    }
    #[cfg(target_os = "windows")]
    {
        win_capture::primary_monitor_scale()
    }
    #[cfg(not(any(target_os = "linux", target_os = "windows")))]
    {
        Some(1.0)
    }
}

/// Make the process Per-Monitor DPI aware V2 (Windows). No-op elsewhere.
/// Call before creating windows / capture so GDI, metrics, and input agree on physical pixels.
pub fn enable_per_monitor_dpi_v2() {
    #[cfg(target_os = "windows")]
    {
        win_capture::enable_per_monitor_dpi_v2();
    }
}

/// Number of displays from the live capturer, or `1` when capture is unavailable.
pub fn monitor_count() -> usize {
    use sqyre_ports::ScreenCapturer;
    let Ok(capturer) = shared_capturer() else {
        return 1;
    };
    let mut wrap = SharedRunCapturer(capturer);
    wrap.monitor_sizes().map(|s| s.len().max(1)).unwrap_or(1)
}

/// Open top-level windows with stable executable path and title.
#[cfg(target_os = "linux")]
pub fn list_open_windows() -> Result<Vec<WindowInfo>, CaptureError> {
    linux::wayland::list_open_windows()
}

#[cfg(target_os = "windows")]
pub fn list_open_windows() -> Result<Vec<WindowInfo>, CaptureError> {
    win_focus::list_open_windows()
}

#[cfg(not(any(target_os = "linux", target_os = "windows")))]
pub fn list_open_windows() -> Result<Vec<WindowInfo>, CaptureError> {
    Err(CaptureError::UnsupportedPlatform)
}

/// Currently focused top-level window, if any.
#[cfg(target_os = "linux")]
pub fn get_active_window() -> Result<Option<WindowInfo>, CaptureError> {
    linux::wayland::get_active_window()
}

#[cfg(target_os = "windows")]
pub fn get_active_window() -> Result<Option<WindowInfo>, CaptureError> {
    win_focus::get_active_window()
}

#[cfg(not(any(target_os = "linux", target_os = "windows")))]
pub fn get_active_window() -> Result<Option<WindowInfo>, CaptureError> {
    Err(CaptureError::UnsupportedPlatform)
}

/// X11: hide overlay tool windows from Alt-Tab / taskbar (no-op elsewhere).
#[cfg(target_os = "linux")]
pub fn skip_taskbar_for_overlay_windows() -> Result<(), CaptureError> {
    x11_focus::skip_taskbar_for_overlay_windows()
}

#[cfg(not(target_os = "linux"))]
pub fn skip_taskbar_for_overlay_windows() -> Result<(), CaptureError> {
    Ok(())
}

/// Windows: enable DWM per-pixel alpha on overlay HWNDs (no-op elsewhere).
///
/// Needed because eframe/glow strips `transparent` when creating deferred viewports.
#[cfg(target_os = "windows")]
pub fn enable_overlay_window_transparency() -> Result<(), CaptureError> {
    win_focus::enable_overlay_window_transparency()
}

#[cfg(not(target_os = "windows"))]
pub fn enable_overlay_window_transparency() -> Result<(), CaptureError> {
    Ok(())
}

/// Stable WM title used by floating macro-overlay viewports.
#[cfg(target_os = "linux")]
pub use x11_focus::OVERLAY_WM_TITLE;

#[cfg(not(target_os = "linux"))]
pub const OVERLAY_WM_TITLE: &str = "sqyre-overlay";

/// True when the focused window belongs to this process (e.g. an overlay button).
pub fn active_window_is_our_process() -> bool {
    let Ok(Some(win)) = get_active_window() else {
        return false;
    };
    window_is_our_process(&win)
}

/// True when `win` is owned by this process's executable.
pub fn window_is_our_process(win: &WindowInfo) -> bool {
    // `current_exe` is unsupported / may panic on wasm32-unknown-unknown.
    #[cfg(target_arch = "wasm32")]
    {
        let _ = win;
        return false;
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let Ok(exe) = std::env::current_exe() else {
            return false;
        };
        window_matches_process(win, &exe.to_string_lossy())
    }
}

/// True when `program` is empty, or the focused window looks like that catalog program.
///
/// Matching is case-insensitive against process name, executable basename, or window title
/// (exact or contains). Catalog program names are user-defined labels, not OS process names.
pub fn active_window_matches_program(program: &str) -> bool {
    let program = program.trim();
    if program.is_empty() {
        return true;
    }
    let Ok(Some(win)) = get_active_window() else {
        return false;
    };
    window_matches_program(&win, program)
}

/// True when `process_path` is empty, or the focused window's executable matches it.
pub fn active_window_matches_process(process_path: &str) -> bool {
    let process_path = process_path.trim();
    if process_path.is_empty() {
        return true;
    }
    let Ok(Some(win)) = get_active_window() else {
        return false;
    };
    window_matches_process(&win, process_path)
}

/// Case-insensitive match of a window against a catalog program name.
pub fn window_matches_program(win: &WindowInfo, program: &str) -> bool {
    let needle = program.trim().to_lowercase();
    if needle.is_empty() {
        return true;
    }
    let name = win.process_name.trim().to_lowercase();
    let title = win.title.trim().to_lowercase();
    let basename = std::path::Path::new(win.process_path.trim())
        .file_name()
        .map(|n| n.to_string_lossy().to_lowercase())
        .unwrap_or_default();
    name == needle
        || basename == needle
        || title == needle
        || name.contains(&needle)
        || basename.contains(&needle)
        || title.contains(&needle)
}

/// Match by full executable path, or by basename when either side is a bare name.
pub fn window_matches_process(win: &WindowInfo, process_path: &str) -> bool {
    let want = process_path.trim();
    if want.is_empty() {
        return true;
    }
    let want_base = std::path::Path::new(want)
        .file_name()
        .map(|n| n.to_string_lossy().to_lowercase())
        .unwrap_or_else(|| want.to_lowercase());
    let got = win.process_path.trim();
    if got.is_empty() {
        let name = win.process_name.trim();
        return !name.is_empty()
            && (name.eq_ignore_ascii_case(want) || name.eq_ignore_ascii_case(&want_base));
    }
    if got.eq_ignore_ascii_case(want) {
        return true;
    }
    let got_base = std::path::Path::new(got)
        .file_name()
        .map(|n| n.to_string_lossy().to_lowercase())
        .unwrap_or_else(|| got.to_lowercase());
    !want_base.is_empty() && want_base == got_base
}

/// Exact trim match of window title (Focus Window / overlay binding parity).
/// Empty `window_title` always matches (process-only binding).
pub fn window_matches_title(win: &WindowInfo, window_title: &str) -> bool {
    let want = window_title.trim();
    if want.is_empty() {
        return true;
    }
    win.title.trim() == want
}

/// Match a program binding: process path required; title required when non-empty.
/// Disambiguates shared executables (e.g. multiple games under one `GameThread` binary).
pub fn window_matches_binding(win: &WindowInfo, process_path: &str, window_title: &str) -> bool {
    window_matches_process(win, process_path) && window_matches_title(win, window_title)
}

/// Stub focuser when OS window activation is not implemented.
#[cfg(not(any(target_os = "linux", target_os = "windows")))]
#[derive(Debug, Default, Clone, Copy)]
pub struct OsWindowFocuser;

#[cfg(not(any(target_os = "linux", target_os = "windows")))]
impl sqyre_ports::WindowFocuser for OsWindowFocuser {
    fn focus(
        &self,
        _process_path: &str,
        _window_title: &str,
    ) -> Result<(), sqyre_ports::AutomationError> {
        Err(sqyre_ports::AutomationError::Unsupported("focus window"))
    }
}

#[cfg(test)]
mod tests {
    use super::WindowInfo;

    #[test]
    fn window_info_label() {
        let w = WindowInfo {
            title: "Notes".into(),
            process_name: "gedit".into(),
            process_path: "/usr/bin/gedit".into(),
            icon: None,
        };
        assert_eq!(w.label(), "Notes  (gedit — /usr/bin/gedit)");
        assert_eq!(
            WindowInfo {
                title: "  ".into(),
                process_name: "x".into(),
                process_path: String::new(),
                icon: None,
            }
            .label(),
            "(untitled)  (x)"
        );
    }

    #[test]
    fn window_matches_program_name() {
        let w = WindowInfo {
            title: "Demo Game — Lobby".into(),
            process_name: "demo-game".into(),
            process_path: "/opt/demo-game/bin/DemoGame".into(),
            icon: None,
        };
        assert!(super::window_matches_program(&w, "Demo Game"));
        assert!(super::window_matches_program(&w, "demo-game"));
        assert!(super::window_matches_program(&w, "DemoGame"));
        assert!(!super::window_matches_program(&w, "OtherApp"));
        assert!(super::window_matches_program(&w, ""));
    }

    #[test]
    fn window_matches_process_path() {
        let w = WindowInfo {
            title: "Demo Game — Lobby".into(),
            process_name: "demo-game".into(),
            process_path: "/opt/demo-game/bin/DemoGame".into(),
            icon: None,
        };
        assert!(super::window_matches_process(
            &w,
            "/opt/demo-game/bin/DemoGame"
        ));
        assert!(super::window_matches_process(&w, "DemoGame"));
        assert!(super::window_matches_process(&w, "/elsewhere/DemoGame"));
        assert!(!super::window_matches_process(&w, "/opt/other/OtherApp"));
        assert!(super::window_matches_process(&w, ""));
        let unnamed = WindowInfo {
            title: "Firefox".into(),
            process_name: "firefox".into(),
            process_path: String::new(),
            icon: None,
        };
        assert!(super::window_matches_process(&unnamed, "/usr/bin/firefox"));
        assert!(super::window_matches_process(&unnamed, "firefox"));
        assert!(!super::window_matches_process(&unnamed, "chrome"));
    }

    #[test]
    fn window_matches_binding_shared_exe() {
        let w = WindowInfo {
            title: "Game A".into(),
            process_name: "GameThread".into(),
            process_path: "/opt/launcher/GameThread".into(),
            icon: None,
        };
        assert!(super::window_matches_binding(
            &w,
            "/opt/launcher/GameThread",
            "Game A"
        ));
        assert!(!super::window_matches_binding(
            &w,
            "/opt/launcher/GameThread",
            "Game B"
        ));
        // Empty title → process-only (single-exe programs).
        assert!(super::window_matches_binding(
            &w,
            "/opt/launcher/GameThread",
            ""
        ));
        assert!(super::window_matches_title(&w, " Game A "));
        assert!(!super::window_matches_title(&w, "Game B"));
    }
}
