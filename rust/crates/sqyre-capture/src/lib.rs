//! Screen capture in absolute virtual-desktop coordinates.

mod stub;
#[cfg(target_os = "linux")]
mod x11_capture;
#[cfg(target_os = "linux")]
mod x11_focus;

pub use stub::{NullCapturer, SolidCapturer};

#[cfg(target_os = "linux")]
pub use x11_capture::X11Capturer;

#[cfg(target_os = "linux")]
pub use x11_focus::X11WindowFocuser;

#[cfg(not(target_os = "linux"))]
pub type X11Capturer = NullCapturer;

/// One top-level application window for Focus Window picker UI (Go `WindowInfo`).
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

/// Open top-level windows with stable executable path and title (Go `ActiveWindows`).
#[cfg(target_os = "linux")]
pub fn list_open_windows() -> Result<Vec<WindowInfo>, String> {
    x11_focus::list_open_windows()
}

#[cfg(not(target_os = "linux"))]
pub fn list_open_windows() -> Result<Vec<WindowInfo>, String> {
    Err("list windows: not supported on this platform".into())
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
}
