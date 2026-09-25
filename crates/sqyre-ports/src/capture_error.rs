//! Typed errors for [`crate::ScreenCapturer`] (and platform capture backends).

use thiserror::Error;

/// Capture is not broken, just not ready to hand out a frame yet.
///
/// Callers should retry on the next poll instead of surfacing a failure — see
/// [`CaptureError::is_retryable`].
#[derive(Debug, Error, Clone, Copy, PartialEq, Eq)]
pub enum NotReady {
    #[error("screen capture is still waiting for portal permission")]
    AwaitingPortalPermission,
    #[error("screen capture is starting (waiting for the first frame)")]
    AwaitingFirstFrame,
    #[error("portal capture: no frame yet from PipeWire")]
    NoFrameYet,
}

/// Failure capturing screen pixels or querying display geometry.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum CaptureError {
    #[error("open display failed (need X11 or XWayland; Wayland-only sessions are not supported)")]
    OpenDisplay,
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
    /// Transient startup state, not a failure.
    #[error(transparent)]
    NotReady(#[from] NotReady),
    /// A Win32 entry point returned failure. `api` is the function name.
    #[error("{api} failed: {detail}")]
    Win32 { api: &'static str, detail: String },
    /// An Xlib request failed. `op` names the request or helper.
    #[error("X11 {op}: {detail}")]
    X11 { op: &'static str, detail: String },
    /// A Wayland protocol or D-Bus step failed. `op` names the step.
    #[error("wayland {op}: {detail}")]
    Wayland { op: &'static str, detail: String },
    /// The xdg-desktop-portal ScreenCast session failed or was refused.
    #[error("portal capture: {0}")]
    Portal(String),
    /// Anything without a dedicated variant. Prefer adding one over reaching
    /// for this — callers cannot match on a string.
    #[error("{0}")]
    Message(String),
}

impl CaptureError {
    /// True when capture has not started yet and the caller should retry.
    ///
    /// Replaces matching on message text, which silently broke whenever a
    /// message was reworded.
    pub fn is_retryable(&self) -> bool {
        matches!(self, Self::NotReady(_))
    }
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
    fn only_not_ready_is_retryable() {
        for r in [
            NotReady::AwaitingPortalPermission,
            NotReady::AwaitingFirstFrame,
            NotReady::NoFrameYet,
        ] {
            assert!(CaptureError::from(r).is_retryable(), "{r:?}");
        }
        for e in [
            CaptureError::OpenDisplay,
            CaptureError::UnsupportedPlatform,
            CaptureError::Portal("session closed".into()),
            CaptureError::Message("waiting for portal".into()),
        ] {
            assert!(!e.is_retryable(), "{e:?}");
        }
    }

    #[test]
    fn platform_variants_render_api_and_detail() {
        assert_eq!(
            CaptureError::Win32 {
                api: "CreateWindowExW",
                detail: "0x5".into(),
            }
            .to_string(),
            "CreateWindowExW failed: 0x5"
        );
        assert_eq!(
            CaptureError::X11 {
                op: "XInternAtom",
                detail: "_NET_WM_NAME".into(),
            }
            .to_string(),
            "X11 XInternAtom: _NET_WM_NAME"
        );
    }
}
