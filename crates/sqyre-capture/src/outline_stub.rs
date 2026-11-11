//! Stub selection outline when no platform backend is available (e.g. macOS).

use crate::CaptureError;

/// No-op outline for platforms without a selection-outline backend.
#[derive(Debug, Default)]
pub struct SelectionOutline;

impl SelectionOutline {
    pub fn open() -> Result<Self, CaptureError> {
        Err(CaptureError::UnsupportedPlatform)
    }

    pub fn set_rect(&mut self, _left: i32, _top: i32, _right: i32, _bottom: i32) {}

    pub fn clear(&mut self) {}

    pub fn is_active(&self) -> bool {
        false
    }
}

/// No-op point markers where there is no desktop overlay backend.
#[derive(Debug, Default)]
pub struct PointMarkerOverlay;

impl PointMarkerOverlay {
    pub fn open() -> Result<Self, CaptureError> {
        Err(CaptureError::UnsupportedPlatform)
    }

    pub fn set_points(&mut self, _points: &[(i32, i32)]) {}

    pub fn clear(&mut self) {}
}

/// No-op click-marker host where there is no desktop overlay backend.
#[derive(Clone, Debug, Default)]
pub struct ClickFxHost;

impl ClickFxHost {
    pub fn start() -> Result<Self, CaptureError> {
        Err(CaptureError::UnsupportedPlatform)
    }

    pub fn set_flash_ms(&self, _ms: i32) {}

    pub fn flash(&self, _x: i32, _y: i32) {}
}
