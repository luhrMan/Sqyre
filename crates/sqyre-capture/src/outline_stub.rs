//! Stub selection outline when X11 is unavailable.

pub use crate::outline_rect::OutlineRect;

/// No-op outline for non-Linux builds.
#[derive(Debug, Default)]
pub struct SelectionOutline;

impl SelectionOutline {
    pub fn open() -> Result<Self, String> {
        Err("selection outline: X11 only".into())
    }

    pub fn set_rect(&mut self, _left: i32, _top: i32, _right: i32, _bottom: i32) {}

    pub fn clear(&mut self) {}
}
