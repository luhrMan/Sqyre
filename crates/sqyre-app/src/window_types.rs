//! Window/process icon data for pickers — shared between native capture and WASM stubs.

/// RGBA icon extracted from an OS window or executable.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProcessIcon {
    pub width: u32,
    pub height: u32,
    /// Unmultiplied RGBA, `width * height * 4` bytes.
    pub rgba: Vec<u8>,
}

/// One top-level application window for Focus Window picker UI.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WindowInfo {
    pub title: String,
    pub process_name: String,
    pub process_path: String,
    pub icon: Option<ProcessIcon>,
}

impl WindowInfo {
    /// Human-readable list line: `title  (name — path)`.
    pub fn label(&self) -> String {
        let title = self.title.trim();
        let title = if title.is_empty() {
            "(untitled)"
        } else {
            title
        };
        format!("{}  ({} — {})", title, self.process_name, self.process_path)
    }
}

#[cfg(feature = "native-runtime")]
pub fn window_info_from_capture(w: sqyre_capture::WindowInfo) -> WindowInfo {
    WindowInfo {
        title: w.title,
        process_name: w.process_name,
        process_path: w.process_path,
        icon: w.icon.map(|i| ProcessIcon {
            width: i.width,
            height: i.height,
            rgba: i.rgba,
        }),
    }
}
