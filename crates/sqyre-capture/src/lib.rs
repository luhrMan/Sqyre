//! Screen capture in absolute virtual-desktop coordinates.

mod diag;
mod error;
#[cfg(not(any(target_os = "linux", target_os = "windows")))]
mod grab_stub;
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
mod x11_focus;
#[cfg(target_os = "linux")]
mod x11_grab;
#[cfg(target_os = "linux")]
mod x11_outline;
#[cfg(target_os = "linux")]
mod x11_secondary;

pub use diag::{
    disk_logging_enabled, mark_site, note, read_last_site, set_disk_logging, set_log_dir,
    CRASH_LOG_FILE, DIAG_LOG_FILE, LAST_SITE_FILE,
};
pub use error::{linux_session_capture_warning, CaptureError};
pub use outline_rect::OutlineRect;
pub use pixel_convert::{zpixmap_to_rgb, zpixmap_to_rgba};
pub use selection_grab::GrabPoll;
pub use stub::{NullCapturer, SolidCapturer};

#[cfg(target_os = "linux")]
pub use x11_capture::{shared_capturer, OsCapturer, SharedRunCapturer};

#[cfg(target_os = "windows")]
pub use win_capture::{shared_capturer, OsCapturer, SharedRunCapturer};

#[cfg(target_os = "linux")]
pub use x11_focus::OsWindowFocuser;

#[cfg(target_os = "windows")]
pub use win_focus::OsWindowFocuser;

#[cfg(target_os = "linux")]
pub use x11_outline::SelectionOutline;

#[cfg(target_os = "windows")]
pub use win_outline::SelectionOutline;

#[cfg(target_os = "linux")]
pub use x11_grab::SelectionGrab;

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

/// Primary monitor resolution key (`"{w}x{h}"`).
/// Uses the first entry from [`ScreenCapturer::monitor_sizes`] (display 0 / primary).
/// Returns `None` when no display is available (headless / CI).
pub fn main_monitor_resolution_key() -> Option<String> {
    use sqyre_ports::ScreenCapturer;
    let capturer = shared_capturer().ok()?;
    let mut wrap = SharedRunCapturer(capturer);
    let sizes = wrap.monitor_sizes().ok()?;
    let &(w, h) = sizes.first()?;
    if w > 0 && h > 0 {
        Some(format!("{w}x{h}"))
    } else {
        None
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
    x11_focus::list_open_windows()
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
    x11_focus::get_active_window()
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
    let got = win.process_path.trim();
    if got.is_empty() {
        return false;
    }
    if got.eq_ignore_ascii_case(want) {
        return true;
    }
    let want_base = std::path::Path::new(want)
        .file_name()
        .map(|n| n.to_string_lossy().to_lowercase())
        .unwrap_or_else(|| want.to_lowercase());
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
