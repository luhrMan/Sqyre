//! Wayland selection outline via layer-shell-style deferred surfaces.
//!
//! Until a dedicated layer-shell helper is attached to egui/winit, the outline
//! opens successfully as a no-op geometry tracker so recording UI can proceed;
//! `set_rect` stores the last rect for future compositor surfaces.

use crate::outline_geometry::outline_should_clear;
pub use crate::outline_rect::OutlineRect;
use crate::CaptureError;

/// Selection outline on Wayland (geometry tracked; compositor surface TBD).
#[derive(Debug, Default)]
pub struct WaylandSelectionOutline {
    mapped: bool,
    last: Option<OutlineRect>,
}

impl WaylandSelectionOutline {
    pub fn open() -> Result<Self, CaptureError> {
        Ok(Self::default())
    }

    pub fn set_rect(&mut self, left: i32, top: i32, right: i32, bottom: i32) {
        let rect = OutlineRect::normalize(left, top, right, bottom);
        if outline_should_clear(rect) {
            self.clear();
            return;
        }
        self.last = Some(rect);
        self.mapped = true;
    }

    pub fn clear(&mut self) {
        self.mapped = false;
        self.last = None;
    }

    /// Last outline rect, if any (for layer-shell wiring / tests).
    pub fn last_rect(&self) -> Option<OutlineRect> {
        self.last
    }

    pub fn is_mapped(&self) -> bool {
        self.mapped
    }
}
