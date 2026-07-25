//! Typed capture / focus errors (map to `String` at `ScreenCapturer` boundary).

use thiserror::Error;

#[derive(Debug, Error)]
pub enum CaptureError {
    #[error(
        "open display failed (need X11 or XWayland; Wayland-only sessions are not supported)"
    )]
    OpenDisplay,
    #[error("query pointer failed")]
    QueryPointer,
    #[error("empty capture rect")]
    EmptyRect,
    #[error("capture failed for {x},{y},{w},{h}")]
    GetImage { x: i32, y: i32, w: i32, h: i32 },
    #[error("unexpected bits_per_pixel {0}")]
    BitsPerPixel(i32),
    #[error("OsCapturer: only display 0 supported for now (got {0})")]
    UnsupportedDisplay(i32),
    #[error("GDI: {0}")]
    Gdi(String),
    #[error("mutex poisoned: {0}")]
    Mutex(String),
    #[error("{0}")]
    Message(String),
}

impl From<CaptureError> for String {
    fn from(e: CaptureError) -> Self {
        e.to_string()
    }
}

/// Human-readable warning when the Linux session cannot support X11 capture.
///
/// Returns `None` on non-Linux targets and when `DISPLAY` is available.
#[cfg(target_os = "linux")]
pub fn linux_session_capture_warning() -> Option<String> {
    let session = std::env::var("XDG_SESSION_TYPE").unwrap_or_default();
    let wayland = session.eq_ignore_ascii_case("wayland")
        || std::env::var_os("WAYLAND_DISPLAY").is_some();
    let has_x11 = std::env::var_os("DISPLAY").is_some();
    if wayland && !has_x11 {
        Some(
            "Pure Wayland session detected (no DISPLAY). Sqyre needs X11 or XWayland for screen capture, window focus, and overlays."
                .into(),
        )
    } else {
        None
    }
}

#[cfg(not(target_os = "linux"))]
pub fn linux_session_capture_warning() -> Option<String> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn open_display_mentions_wayland() {
        let msg = CaptureError::OpenDisplay.to_string();
        assert!(msg.contains("XWayland") || msg.contains("Wayland"), "{msg}");
    }
}
