//! Pure preview geometry: capture bounds, outline rects, display fitting.

use super::{DISPLAY_MAX_H, DISPLAY_MAX_W, PANEL_MAX_DIM};
use eframe::egui::{self, Vec2};
use image::RgbaImage;
use sqyre_ports::DesktopRect;

const MIN_CAPTURE_SIZE: i32 = 320;
const CAPTURE_PADDING: i32 = 48;
const PANEL_MIN_W: f32 = 160.0;
const PANEL_MIN_H: f32 = 120.0;
/// Slack so filling remaining height does not trip ScrollArea overflow (rounding / bar hysteresis).
const PANEL_FILL_SLACK: f32 = 1.0;
/// Half-size of the desktop outline box drawn around a point (px).
const POINT_OUTLINE_HALF: i32 = 10;

#[derive(Clone, Copy)]
pub(super) enum PreviewCoords {
    Point {
        x: i32,
        y: i32,
    },
    SearchArea {
        left: i32,
        top: i32,
        right: i32,
        bottom: i32,
        grid: Option<(i32, i32)>,
    },
}

/// True when both describe the same absolute desktop region (grid overlay ignored).
pub(super) fn same_capture_region(a: PreviewCoords, b: PreviewCoords) -> bool {
    match (a, b) {
        (PreviewCoords::Point { x: x1, y: y1 }, PreviewCoords::Point { x: x2, y: y2 }) => {
            x1 == x2 && y1 == y2
        }
        (
            PreviewCoords::SearchArea {
                left: l1,
                top: t1,
                right: r1,
                bottom: b1,
                ..
            },
            PreviewCoords::SearchArea {
                left: l2,
                top: t2,
                right: r2,
                bottom: b2,
                ..
            },
        ) => l1 == l2 && t1 == t2 && r1 == r2 && b1 == b2,
        _ => false,
    }
}

pub(super) fn panel_viewport_size(ui: &egui::Ui) -> Vec2 {
    let w = ui.available_width().max(PANEL_MIN_W);
    let avail_h = ui.available_height();
    // Fill remaining height; only exceed it (scrollbar) when below the minimum.
    let h = if avail_h < PANEL_MIN_H {
        PANEL_MIN_H
    } else {
        (avail_h - PANEL_FILL_SLACK).max(PANEL_MIN_H)
    };
    Vec2::new(w, h)
}

pub(super) fn fit_display(ui: &egui::Ui, w: f32, h: f32) -> Vec2 {
    let w = w.max(1.0);
    let h = h.max(1.0);
    // Cap at the visible bound: tip bodies live in vertical-only scroll areas
    // that do not clip horizontally, so a fixed-size image would paint past
    // the tip edge when the host is narrower than DISPLAY_MAX_W.
    let max_w = DISPLAY_MAX_W.min(crate::widgets::visible_content_width(ui));
    let scale = (max_w / w).min(DISPLAY_MAX_H / h).min(1.0);
    Vec2::new(w * scale, h * scale)
}

pub(super) fn normalize_rect(lx: i32, ty: i32, rx: i32, by: i32) -> (i32, i32, i32, i32) {
    DesktopRect::normalize_corners(lx, ty, rx, by)
}

pub(super) fn desktop_outline_rect(coords: PreviewCoords) -> (i32, i32, i32, i32) {
    match coords {
        PreviewCoords::Point { x, y } => {
            let h = POINT_OUTLINE_HALF;
            (x - h, y - h, x + h + 1, y + h + 1)
        }
        PreviewCoords::SearchArea {
            left,
            top,
            right,
            bottom,
            grid: _,
        } => normalize_rect(left, top, right, bottom),
    }
}

pub(super) fn preview_bounds_for_point(px: i32, py: i32, vb: DesktopRect) -> DesktopRect {
    let half = MIN_CAPTURE_SIZE / 2;
    let desired = DesktopRect {
        x: px - half,
        y: py - half,
        w: MIN_CAPTURE_SIZE,
        h: MIN_CAPTURE_SIZE,
    };
    shift_into_virtual(desired, vb)
}

pub(super) fn preview_bounds_for_search_area(
    lx: i32,
    ty: i32,
    rx: i32,
    by: i32,
    vb: DesktopRect,
) -> DesktopRect {
    let aw = (rx - lx).max(0);
    let ah = (by - ty).max(0);
    let pad_x = CAPTURE_PADDING.max(aw / 4);
    let pad_y = CAPTURE_PADDING.max(ah / 4);
    let desired = expand_to_min(
        DesktopRect {
            x: lx - pad_x,
            y: ty - pad_y,
            w: aw + pad_x * 2,
            h: ah + pad_y * 2,
        },
        MIN_CAPTURE_SIZE,
        MIN_CAPTURE_SIZE,
    );
    shift_into_virtual(desired, vb)
}

fn expand_to_min(r: DesktopRect, min_w: i32, min_h: i32) -> DesktopRect {
    if r.is_empty() {
        return r;
    }
    let w = r.w.max(min_w);
    let h = r.h.max(min_h);
    let cx = r.x + r.w / 2;
    let cy = r.y + r.h / 2;
    DesktopRect {
        x: cx - w / 2,
        y: cy - h / 2,
        w,
        h,
    }
}

fn shift_into_virtual(desired: DesktopRect, vb: DesktopRect) -> DesktopRect {
    if desired.is_empty() || vb.is_empty() {
        return DesktopRect::from_corners(
            desired.x.max(vb.x),
            desired.y.max(vb.y),
            (desired.x + desired.w).min(vb.x + vb.w),
            (desired.y + desired.h).min(vb.y + vb.h),
        );
    }
    let mut w = desired.w;
    let mut h = desired.h;
    if w <= 0 || h <= 0 {
        return DesktopRect::default();
    }
    if w >= vb.w && h >= vb.h {
        return vb;
    }
    let mut x0 = desired.x;
    let mut y0 = desired.y;
    if w > vb.w {
        x0 = vb.x;
        w = vb.w;
    } else {
        if x0 < vb.x {
            x0 = vb.x;
        }
        if x0 + w > vb.x + vb.w {
            x0 = vb.x + vb.w - w;
        }
    }
    if h > vb.h {
        y0 = vb.y;
        h = vb.h;
    } else {
        if y0 < vb.y {
            y0 = vb.y;
        }
        if y0 + h > vb.y + vb.h {
            y0 = vb.y + vb.h - h;
        }
    }
    DesktopRect { x: x0, y: y0, w, h }
}

pub(super) fn downscale_max_dim(img: RgbaImage, max_dim: u32) -> RgbaImage {
    let (w, h) = img.dimensions();
    if w <= max_dim && h <= max_dim {
        return img;
    }
    let longest = w.max(h).max(1);
    let nw = ((w as u64 * max_dim as u64) / longest as u64).max(1) as u32;
    let nh = ((h as u64 * max_dim as u64) / longest as u64).max(1) as u32;
    let filter = if max_dim >= PANEL_MAX_DIM {
        image::imageops::FilterType::CatmullRom
    } else {
        image::imageops::FilterType::Triangle
    };
    image::imageops::resize(&img, nw, nh, filter)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn point_bounds_centered_and_clamped() {
        let vb = DesktopRect {
            x: 0,
            y: 0,
            w: 1920,
            h: 1080,
        };
        let b = preview_bounds_for_point(100, 100, vb);
        assert_eq!(b.w, MIN_CAPTURE_SIZE);
        assert_eq!(b.h, MIN_CAPTURE_SIZE);
        assert!(b.x >= 0 && b.y >= 0);
        assert!(b.x + b.w <= vb.w);
        assert!(b.y + b.h <= vb.h);
    }

    #[test]
    fn search_area_bounds_include_padding() {
        let vb = DesktopRect {
            x: 0,
            y: 0,
            w: 2560,
            h: 1440,
        };
        let b = preview_bounds_for_search_area(100, 100, 200, 180, vb);
        assert!(b.w >= MIN_CAPTURE_SIZE);
        assert!(b.h >= MIN_CAPTURE_SIZE);
        assert!(b.x <= 100);
        assert!(b.y <= 100);
        assert!(b.x + b.w >= 200);
        assert!(b.y + b.h >= 180);
    }

    #[test]
    fn same_capture_region_ignores_grid_overlay() {
        let a = PreviewCoords::SearchArea {
            left: 1,
            top: 2,
            right: 10,
            bottom: 20,
            grid: None,
        };
        let b = PreviewCoords::SearchArea {
            left: 1,
            top: 2,
            right: 10,
            bottom: 20,
            grid: Some((3, 4)),
        };
        let c = PreviewCoords::SearchArea {
            left: 1,
            top: 2,
            right: 11,
            bottom: 20,
            grid: None,
        };
        assert!(same_capture_region(a, b));
        assert!(!same_capture_region(a, c));
        assert!(!same_capture_region(PreviewCoords::Point { x: 1, y: 2 }, a));
    }
}
