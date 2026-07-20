//! Typed capture / focus errors (map to `String` at `ScreenCapturer` boundary).

use thiserror::Error;

#[derive(Debug, Error)]
pub enum CaptureError {
    #[error("open display failed (need a graphical session)")]
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
