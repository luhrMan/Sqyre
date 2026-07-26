//! Cross-platform outline geometry helpers shared by the X11/Windows selection outlines.
//!
//! Pure logic only — OS window creation/positioning stays in `x11_outline` / `win_outline`.

use crate::OutlineRect;

/// Thickness (px) of each outline edge window.
pub(crate) const EDGE_PX: i32 = 2;

/// True when `rect` is empty or too small to draw a hollow outline (the outline should be
/// cleared instead).
pub(crate) fn outline_should_clear(rect: OutlineRect) -> bool {
    rect.is_empty() || rect.width() < EDGE_PX * 2 || rect.height() < EDGE_PX * 2
}

/// `(x, y, w, h)` placements for the four edge windows — top, bottom, left, right, in that
/// order — that together form a hollow rectangle outline around `rect`.
pub(crate) fn edge_placements(rect: OutlineRect) -> [(i32, i32, i32, i32); 4] {
    let w = rect.width().max(1);
    let h = rect.height().max(1);
    let t = EDGE_PX;
    [
        (rect.left, rect.top, w, t),
        (rect.left, rect.bottom - EDGE_PX, w, t),
        (rect.left, rect.top, t, h),
        (rect.right - EDGE_PX, rect.top, t, h),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_clear_when_empty_or_too_small() {
        assert!(outline_should_clear(OutlineRect::normalize(0, 0, 0, 0)));
        assert!(outline_should_clear(OutlineRect::normalize(0, 0, 3, 3)));
        assert!(!outline_should_clear(OutlineRect::normalize(0, 0, 10, 10)));
    }

    #[test]
    fn edge_placements_match_rect_bounds() {
        let rect = OutlineRect::normalize(10, 20, 110, 220);
        let edges = edge_placements(rect);
        assert_eq!(edges[0], (10, 20, 100, EDGE_PX)); // top
        assert_eq!(edges[1], (10, 220 - EDGE_PX, 100, EDGE_PX)); // bottom
        assert_eq!(edges[2], (10, 20, EDGE_PX, 200)); // left
        assert_eq!(edges[3], (110 - EDGE_PX, 20, EDGE_PX, 200)); // right
    }
}
