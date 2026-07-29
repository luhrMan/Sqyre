//! Wayland portal-backed screen capture (ScreenCast layout + Screenshot frames).

use crate::wayland_portal;
use crate::CaptureError;
use image::RgbaImage;
use parking_lot::Mutex;
use sqyre_ports::{DesktopRect, RgbCapture};

/// Pure-Wayland capturer using XDG Desktop Portals.
pub struct WaylandCapturer {
    /// Cached virtual bounds (refreshed from portal layout).
    bounds: Mutex<Option<DesktopRect>>,
}

impl WaylandCapturer {
    pub fn open() -> Result<Self, CaptureError> {
        // Defer portal UI until first capture / explicit permission request.
        Ok(Self {
            bounds: Mutex::new(None),
        })
    }

    pub fn capture_rect_ref(&self, rect: DesktopRect) -> Result<RgbaImage, CaptureError> {
        let img = wayland_portal::capture_rect(rect)?;
        *self.bounds.lock() = wayland_portal::virtual_bounds().ok();
        Ok(img)
    }

    pub fn capture_rect_rgb_ref(&self, rect: DesktopRect) -> Result<RgbCapture, CaptureError> {
        let rgba = self.capture_rect_ref(rect)?;
        Ok(RgbCapture::from_rgba(&rgba))
    }

    pub fn virtual_bounds_ref(&self) -> Result<DesktopRect, CaptureError> {
        if let Some(b) = *self.bounds.lock() {
            return Ok(b);
        }
        let b = wayland_portal::virtual_bounds()?;
        *self.bounds.lock() = Some(b);
        Ok(b)
    }

    pub fn monitor_rects_ref(&self) -> Result<Vec<DesktopRect>, CaptureError> {
        wayland_portal::monitor_rects()
    }

    pub fn monitor_sizes_ref(&self) -> Result<Vec<(i32, i32)>, CaptureError> {
        Ok(self
            .monitor_rects_ref()?
            .into_iter()
            .map(|r| (r.w, r.h))
            .collect())
    }
}
