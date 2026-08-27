//! Executor-only ports (OCR) and re-exports from [`sqyre_ports`].

pub use sqyre_match::ImageBuf;
pub use sqyre_ports::{
    clamp_search_rect, AutomationBackend, AutomationError, CaptureError, ContinueKeyWaiter,
    CoordinateResolver, DesktopRect, IconStore, ItemMeta, MacroLookup, MoveOptions, PortError,
    RgbCapture, ScreenCapturer, WindowFocuser,
};

/// Run OCR on a preprocessed image buffer.
pub trait OcrEngine: Send + Sync {
    fn recognize(&self, image: &ImageBuf) -> Result<sqyre_vision::OcrRecognition, PortError>;
}

/// Tesseract-backed engine (native only; not available on wasm32).
#[cfg(not(target_arch = "wasm32"))]
impl OcrEngine for sqyre_vision::LeptessOcr {
    fn recognize(&self, image: &ImageBuf) -> Result<sqyre_vision::OcrRecognition, PortError> {
        sqyre_vision::LeptessOcr::recognize(self, image)
    }
}
