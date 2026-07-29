//! Typed errors for [`crate::ScreenCapturer`] (and platform capture backends).

use thiserror::Error;

/// Failure capturing screen pixels or querying display geometry.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum CaptureError {
    #[error("open display failed (need X11, XWayland, or a Wayland portal session)")]
    OpenDisplay,
    /// User denied or disabled a required desktop permission (Wayland portals).
    #[error("permission denied for {capability}")]
    PermissionDenied { capability: &'static str },
    /// XDG Desktop Portal is missing or returned an error.
    #[error("desktop portal unavailable: {0}")]
    PortalUnavailable(String),
    #[error("query pointer failed")]
    QueryPointer,
    #[error("empty capture rect")]
    EmptyRect,
    #[error("empty search area {left},{top},{right},{bottom}")]
    EmptySearchArea {
        left: i32,
        top: i32,
        right: i32,
        bottom: i32,
    },
    #[error("search area outside virtual desktop")]
    OutsideVirtualDesktop,
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
    #[error("screen capture: not supported on this platform")]
    UnsupportedPlatform,
    #[error("{0}")]
    Message(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn open_display_mentions_wayland() {
        let msg = CaptureError::OpenDisplay.to_string();
        assert!(msg.contains("XWayland") || msg.contains("Wayland"), "{msg}");
    }

    #[test]
    fn permission_denied_names_capability() {
        let msg = CaptureError::PermissionDenied {
            capability: "screen capture",
        }
        .to_string();
        assert!(msg.contains("screen capture"), "{msg}");
    }
}
