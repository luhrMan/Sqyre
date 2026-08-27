//! Cross-platform outline geometry helpers shared by the X11/Windows selection outlines.
//!
//! Pure logic only — OS window creation/positioning stays in `x11_outline` / `win_outline`.

use crate::OutlineRect;

/// Thickness (px) of each outline edge window.
pub(crate) const EDGE_PX: i32 = 2;

/// Selection stroke (gold) shared by X11/Win32 edge windows and the snapshot cover.
pub(crate) const STROKE_R: u8 = 255;
pub(crate) const STROKE_G: u8 = 200;
pub(crate) const STROKE_B: u8 = 0;

/// True when `rect` is empty or too small to draw a hollow outline (the outline should be
/// cleared instead).
pub(crate) fn outline_should_clear(rect: OutlineRect) -> bool {
    rect.is_empty() || rect.width() < EDGE_PX * 2 || rect.height() < EDGE_PX * 2
}

/// `(x, y, w, h)` placements for the four edge windows — top, bottom, left, right, in that
/// order — that together form a hollow rectangle outline around `rect`.
///
/// Left/right are inset so they do not overlap top/bottom. Full-height sides cover all
/// four corners of the horizontal bars; compositors then cull those bars (the top edge
/// is created first, so it is lowest in the stack and is the one that usually vanishes).
pub(crate) fn edge_placements(rect: OutlineRect) -> [(i32, i32, i32, i32); 4] {
    let w = rect.width().max(1);
    let h = rect.height().max(1);
    let t = EDGE_PX;
    let inner_h = (h - 2 * t).max(1);
    [
        (rect.left, rect.top, w, t),
        (rect.left, rect.bottom - t, w, t),
        (rect.left, rect.top + t, t, inner_h),
        (rect.right - t, rect.top + t, t, inner_h),
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
        assert_eq!(edges[2], (10, 20 + EDGE_PX, EDGE_PX, 200 - 2 * EDGE_PX)); // left
        assert_eq!(
            edges[3],
            (110 - EDGE_PX, 20 + EDGE_PX, EDGE_PX, 200 - 2 * EDGE_PX)
        ); // right
    }

    #[test]
    fn edge_placements_do_not_overlap() {
        let rect = OutlineRect::normalize(10, 20, 110, 220);
        let edges = edge_placements(rect);
        for i in 0..4 {
            for j in (i + 1)..4 {
                assert!(
                    !aabb_overlap(edges[i], edges[j]),
                    "edge {i} overlaps {j}: {:?} vs {:?}",
                    edges[i],
                    edges[j]
                );
            }
        }
    }

    #[test]
    fn full_height_sides_would_cover_top_corners() {
        // Old layout: left/right spanned the full height, covering every corner of the
        // 2px top bar. Compositors that cull on covered corners then drop that bar.
        let rect = OutlineRect::normalize(10, 20, 110, 220);
        let t = EDGE_PX;
        let w = rect.width();
        let h = rect.height();
        let top = (rect.left, rect.top, w, t);
        let left = (rect.left, rect.top, t, h);
        let right = (rect.right - t, rect.top, t, h);
        assert!(aabb_contains_corner(left, top.0, top.1));
        assert!(aabb_contains_corner(left, top.0, top.1 + t - 1));
        assert!(aabb_contains_corner(right, top.0 + w - 1, top.1));
        assert!(aabb_contains_corner(right, top.0 + w - 1, top.1 + t - 1));
        let edges = edge_placements(rect);
        assert!(!aabb_contains_corner(edges[2], edges[0].0, edges[0].1));
        assert!(!aabb_contains_corner(
            edges[3],
            edges[0].0 + edges[0].2 - 1,
            edges[0].1
        ));
    }

    #[test]
    fn edge_placements_wide_and_tall() {
        let wide = OutlineRect::normalize(0, 0, 400, 20);
        assert!(!outline_should_clear(wide));
        let [top, bottom, left, right] = edge_placements(wide);
        assert_eq!(top, (0, 0, 400, EDGE_PX));
        assert_eq!(bottom, (0, 20 - EDGE_PX, 400, EDGE_PX));
        assert_eq!(left.2, EDGE_PX);
        assert_eq!(right.0, 400 - EDGE_PX);

        let tall = OutlineRect::normalize(5, 5, 15, 505);
        assert!(!outline_should_clear(tall));
        let edges = edge_placements(tall);
        assert_eq!(edges[0].2, 10);
        assert_eq!(edges[2].3, 500 - 2 * EDGE_PX);
    }

    #[test]
    fn edge_placements_minimum_drawable() {
        let rect = OutlineRect::normalize(0, 0, EDGE_PX * 2, EDGE_PX * 2);
        assert!(!outline_should_clear(rect));
        let edges = edge_placements(rect);
        assert_eq!(edges[0].2, EDGE_PX * 2);
        assert_eq!(edges[2].3, 1.max(rect.height() - 2 * EDGE_PX));
    }

    fn aabb_overlap(a: (i32, i32, i32, i32), b: (i32, i32, i32, i32)) -> bool {
        a.0 < b.0 + b.2 && b.0 < a.0 + a.2 && a.1 < b.1 + b.3 && b.1 < a.1 + a.3
    }

    fn aabb_contains_corner(bar: (i32, i32, i32, i32), x: i32, y: i32) -> bool {
        x >= bar.0 && x < bar.0 + bar.2 && y >= bar.1 && y < bar.1 + bar.3
    }
}
