//! Stub selection grab when no platform backend is available (e.g. macOS).

use crate::selection_grab::GrabPoll;
use crate::CaptureError;

/// No-op grab for platforms without a selection-grab backend.
#[derive(Debug, Default)]
pub struct SelectionGrab;

impl SelectionGrab {
    pub fn open() -> Result<Self, CaptureError> {
        Err(CaptureError::UnsupportedPlatform)
    }

    pub fn arm(&mut self) -> Result<(), CaptureError> {
        Ok(())
    }

    pub fn disarm(&mut self) {}

    pub fn is_armed(&self) -> bool {
        false
    }

    pub fn poll(&mut self) -> GrabPoll {
        GrabPoll::default()
    }
}
