//! Typed capture / focus errors (map to `String` at `ScreenCapturer` boundary).

use thiserror::Error;

#[derive(Debug, Error)]
pub enum CaptureError {
    #[error("XOpenDisplay failed (need X11, not Wayland-only)")]
    OpenDisplay,
    #[error("XQueryPointer failed")]
    QueryPointer,
    #[error("empty capture rect")]
    EmptyRect,
    #[error("XGetImage failed for {x},{y},{w},{h}")]
    GetImage { x: i32, y: i32, w: i32, h: i32 },
    #[error("unexpected bits_per_pixel {0}")]
    BitsPerPixel(i32),
    #[error("X11Capturer: only display 0 supported for now (got {0})")]
    UnsupportedDisplay(i32),
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
