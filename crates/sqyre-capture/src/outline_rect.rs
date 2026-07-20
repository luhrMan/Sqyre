//! Axis-aligned selection rectangle in absolute desktop corners.

/// Inclusive left/top, exclusive-or-equal right/bottom corners (empty when right≤left or bottom≤top).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OutlineRect {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

impl OutlineRect {
    /// Normalize two corners so left≤right and top≤bottom.
    pub fn normalize(ax: i32, ay: i32, bx: i32, by: i32) -> Self {
        let (left, top, right, bottom) =
            sqyre_executor::DesktopRect::normalize_corners(ax, ay, bx, by);
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

    pub fn width(self) -> i32 {
        (self.right - self.left).max(0)
    }

    pub fn height(self) -> i32 {
        (self.bottom - self.top).max(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_and_empty() {
        let r = OutlineRect::normalize(10, 20, 5, 40);
        assert_eq!(
            r,
            OutlineRect {
                left: 5,
                top: 20,
                right: 10,
                bottom: 40
            }
        );
        assert!(!r.is_empty());
        assert_eq!(r.width(), 5);
        assert_eq!(r.height(), 20);
        assert!(OutlineRect::normalize(1, 1, 1, 1).is_empty());
    }
}
