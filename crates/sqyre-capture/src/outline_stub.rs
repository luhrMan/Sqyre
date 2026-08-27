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
