use image::{Rgba, RgbaImage};
use sqyre_executor::{CaptureError, DesktopRect, ScreenCapturer};

/// Capturer that always fails — for headless CI / tests without display.
#[derive(Debug, Default)]
pub struct NullCapturer;

impl NullCapturer {
    pub fn open() -> Result<Self, CaptureError> {
        Err(CaptureError::Message("NullCapturer: no display".into()))
    }

    pub fn capture_rect_ref(&self, _rect: DesktopRect) -> Result<RgbaImage, CaptureError> {
        Err(CaptureError::Message("NullCapturer: no display".into()))
    }

    pub fn virtual_bounds_ref(&self) -> Result<DesktopRect, CaptureError> {
        Ok(DesktopRect {
            x: 0,
            y: 0,
            w: 1,
            h: 1,
        })
    }

    pub fn monitor_rects_ref(&self) -> Result<Vec<DesktopRect>, CaptureError> {
        Ok(vec![DesktopRect {
            x: 0,
            y: 0,
            w: 1,
            h: 1,
        }])
    }

    pub fn monitor_sizes_ref(&self) -> Result<Vec<(i32, i32)>, CaptureError> {
        Ok(self
            .monitor_rects_ref()?
            .into_iter()
            .map(|r| (r.w, r.h))
            .collect())
    }
}

impl ScreenCapturer for NullCapturer {
    fn capture_monitor(&mut self, _display_index: i32) -> Result<RgbaImage, CaptureError> {
        Err(CaptureError::Message("NullCapturer: no display".into()))
    }
    fn capture_rect(&mut self, _rect: DesktopRect) -> Result<RgbaImage, CaptureError> {
        Err(CaptureError::Message("NullCapturer: no display".into()))
    }
    fn virtual_bounds(&mut self) -> Result<DesktopRect, CaptureError> {
        Ok(DesktopRect {
            x: 0,
            y: 0,
            w: 1,
            h: 1,
        })
    }
}

/// Tiny solid-color capturer for unit tests.
#[derive(Debug)]
pub struct SolidCapturer {
    pub color: Rgba<u8>,
    pub bounds: DesktopRect,
}

impl Default for SolidCapturer {
    fn default() -> Self {
        Self {
            color: Rgba([0, 0, 0, 255]),
            bounds: DesktopRect {
                x: 0,
                y: 0,
                w: 100,
                h: 100,
            },
        }
    }
}

impl ScreenCapturer for SolidCapturer {
    fn capture_monitor(&mut self, _display_index: i32) -> Result<RgbaImage, CaptureError> {
        self.capture_rect(self.bounds)
    }
    fn capture_rect(&mut self, rect: DesktopRect) -> Result<RgbaImage, CaptureError> {
        if rect.is_empty() {
            return Err(CaptureError::EmptyRect);
        }
        Ok(RgbaImage::from_pixel(
            rect.w as u32,
            rect.h as u32,
            self.color,
        ))
    }
    fn virtual_bounds(&mut self) -> Result<DesktopRect, CaptureError> {
        Ok(self.bounds)
    }
}
