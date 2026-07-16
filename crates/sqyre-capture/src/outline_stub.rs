//! Stub selection outline when X11 is unavailable.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OutlineRect {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

impl OutlineRect {
    pub fn normalize(ax: i32, ay: i32, bx: i32, by: i32) -> Self {
        let (left, right) = if ax <= bx { (ax, bx) } else { (bx, ax) };
        let (top, bottom) = if ay <= by { (ay, by) } else { (by, ay) };
        Self {
            left,
            top,
            right,
            bottom,
        }
    }

    pub fn is_empty(self) -> bool {
        self.right <= self.left || self.bottom <= self.top
    }
}

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
