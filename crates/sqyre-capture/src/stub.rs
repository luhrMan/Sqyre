use image::{Rgba, RgbaImage};
use sqyre_executor::{DesktopRect, ScreenCapturer};

/// Capturer that always fails — for headless CI / tests without display.
#[derive(Debug, Default)]
pub struct NullCapturer;

impl NullCapturer {
    pub fn open() -> Result<Self, String> {
        Err("NullCapturer: no display".into())
    }

    pub fn capture_rect_ref(&self, _rect: DesktopRect) -> Result<RgbaImage, String> {
        Err("NullCapturer: no display".into())
    }

    pub fn virtual_bounds_ref(&self) -> Result<DesktopRect, String> {
        Ok(DesktopRect {
            x: 0,
            y: 0,
            w: 1,
            h: 1,
        })
    }
}

impl ScreenCapturer for NullCapturer {
    fn capture_monitor(&mut self, _display_index: i32) -> Result<RgbaImage, String> {
        Err("NullCapturer: no display".into())
    }
    fn capture_rect(&mut self, _rect: DesktopRect) -> Result<RgbaImage, String> {
        Err("NullCapturer: no display".into())
    }
    fn virtual_bounds(&mut self) -> Result<DesktopRect, String> {
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
    fn capture_monitor(&mut self, _display_index: i32) -> Result<RgbaImage, String> {
        self.capture_rect(self.bounds)
    }
    fn capture_rect(&mut self, rect: DesktopRect) -> Result<RgbaImage, String> {
        if rect.is_empty() {
            return Err("empty rect".into());
        }
        Ok(RgbaImage::from_pixel(
            rect.w as u32,
            rect.h as u32,
            self.color,
        ))
    }
    fn virtual_bounds(&mut self) -> Result<DesktopRect, String> {
        Ok(self.bounds)
    }
}
