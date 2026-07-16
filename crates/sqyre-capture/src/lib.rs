//! Screen capture in absolute virtual-desktop coordinates.

mod diag;
mod stub;
#[cfg(target_os = "linux")]
mod x11_capture;
#[cfg(target_os = "linux")]
mod x11_focus;
#[cfg(target_os = "linux")]
mod x11_outline;
#[cfg(not(target_os = "linux"))]
mod outline_stub;

pub use diag::{
    mark_site, note, read_last_site, set_log_dir, CRASH_LOG_FILE, DIAG_LOG_FILE, LAST_SITE_FILE,
};
pub use stub::{NullCapturer, SolidCapturer};

#[cfg(target_os = "linux")]
pub use x11_capture::X11Capturer;

#[cfg(target_os = "linux")]
pub use x11_focus::X11WindowFocuser;

#[cfg(target_os = "linux")]
pub use x11_outline::{OutlineRect, SelectionOutline};

#[cfg(not(target_os = "linux"))]
pub use outline_stub::{OutlineRect, SelectionOutline};

#[cfg(not(target_os = "linux"))]
pub type X11Capturer = NullCapturer;

/// One top-level application window for Focus Window picker UI.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WindowInfo {
    pub title: String,
    pub process_name: String,
    pub process_path: String,
}

impl WindowInfo {
    /// Human-readable list line: `title  (name — path)`.
    pub fn label(&self) -> String {
        let title = self.title.trim();
        let title = if title.is_empty() { "(untitled)" } else { title };
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

/// Primary monitor resolution key (`"{w}x{h}"`).
/// Uses the first entry from [`ScreenCapturer::monitor_sizes`] (display 0 / primary).
/// Returns `None` when no display is available (headless / CI).
pub fn main_monitor_resolution_key() -> Option<String> {
    use sqyre_executor::ScreenCapturer;
    let mut capturer = X11Capturer::open().ok()?;
    let sizes = capturer.monitor_sizes().ok()?;
    let &(w, h) = sizes.first()?;
    if w > 0 && h > 0 {
        Some(format!("{w}x{h}"))
    } else {
        None
    }
}

/// Number of displays from the live capturer, or `1` when capture is unavailable.
pub fn monitor_count() -> usize {
    use sqyre_executor::ScreenCapturer;
    let Ok(mut capturer) = X11Capturer::open() else {
        return 1;
    };
    capturer
        .monitor_sizes()
        .map(|s| s.len().max(1))
        .unwrap_or(1)
}

/// Open top-level windows with stable executable path and title.
#[cfg(target_os = "linux")]
pub fn list_open_windows() -> Result<Vec<WindowInfo>, String> {
    x11_focus::list_open_windows()
}

#[cfg(not(target_os = "linux"))]
pub fn list_open_windows() -> Result<Vec<WindowInfo>, String> {
    Err("list windows: not supported on this platform".into())
}

/// Currently focused top-level window, if any.
#[cfg(target_os = "linux")]
pub fn get_active_window() -> Result<Option<WindowInfo>, String> {
    x11_focus::get_active_window()
}

#[cfg(not(target_os = "linux"))]
pub fn get_active_window() -> Result<Option<WindowInfo>, String> {
    Err("active window: not supported on this platform".into())
}

/// X11: hide overlay tool windows from Alt-Tab / taskbar (no-op elsewhere).
#[cfg(target_os = "linux")]
pub fn skip_taskbar_for_overlay_windows() -> Result<(), String> {
    x11_focus::skip_taskbar_for_overlay_windows()
}

#[cfg(not(target_os = "linux"))]
pub fn skip_taskbar_for_overlay_windows() -> Result<(), String> {
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
    let Ok(exe) = std::env::current_exe() else {
        return false;
    };
    window_matches_process(win, &exe.to_string_lossy())
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

/// No-op focuser for non-Linux (or tests without a display).
#[cfg(not(target_os = "linux"))]
#[derive(Debug, Default, Clone, Copy)]
pub struct X11WindowFocuser;

#[cfg(not(target_os = "linux"))]
impl sqyre_executor::WindowFocuser for X11WindowFocuser {
    fn focus(&self, _process_path: &str, _window_title: &str) -> Result<(), String> {
        Err("focus window: not supported on this platform".into())
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
        };
        assert_eq!(w.label(), "Notes  (gedit — /usr/bin/gedit)");
        assert_eq!(
            WindowInfo {
                title: "  ".into(),
                process_name: "x".into(),
                process_path: String::new(),
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
        };
        assert!(super::window_matches_process(
            &w,
            "/opt/demo-game/bin/DemoGame"
        ));
        assert!(super::window_matches_process(&w, "DemoGame"));
        assert!(super::window_matches_process(
            &w,
            "/elsewhere/DemoGame"
        ));
        assert!(!super::window_matches_process(&w, "/opt/other/OtherApp"));
        assert!(super::window_matches_process(&w, ""));
    }
}
