//! Shared placement for OS-composited overlay popups (tips, chooser, context menus).
//!
//! Hosts draw these outside egui; keep every popup fully inside the monitor that
//! contains the anchor so edge-docked buttons never clip. Future OS menus should
//! call [`place_cursor_popup`] (or [`clamp_popup_origin`] with an explicit monitor)
//! instead of inventing per-host math.

use sqyre_persist::{DEFAULT_OVERLAY_FALLBACK_SCREEN_H, DEFAULT_OVERLAY_FALLBACK_SCREEN_W};

/// Gap between an overlay button and its tip.
pub const TIP_GAP_PX: i32 = 6;

/// Desktop monitor rectangle in physical pixels (`w`/`h` are extents).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MonitorRect {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

impl MonitorRect {
    fn fallback() -> Self {
        Self {
            x: 0,
            y: 0,
            w: DEFAULT_OVERLAY_FALLBACK_SCREEN_W as i32,
            h: DEFAULT_OVERLAY_FALLBACK_SCREEN_H as i32,
        }
    }

    fn contains_point(self, px: i32, py: i32) -> bool {
        px >= self.x
            && py >= self.y
            && px < self.x.saturating_add(self.w)
            && py < self.y.saturating_add(self.h)
    }
}

/// Monitor containing `(x, y)`, else primary / virtual-desktop fallback.
pub fn monitor_for_point(x: i32, y: i32) -> MonitorRect {
    let rects = sqyre_capture::preferred_monitor_rects();
    for r in &rects {
        let mon = MonitorRect {
            x: r.x,
            y: r.y,
            w: r.w,
            h: r.h,
        };
        if mon.contains_point(x, y) {
            return mon;
        }
    }
    rects
        .first()
        .map(|r| MonitorRect {
            x: r.x,
            y: r.y,
            w: r.w,
            h: r.h,
        })
        .unwrap_or_else(MonitorRect::fallback)
}

/// Keep a `w`×`h` popup fully inside `mon`, preferring the given top-left.
///
/// If the popup is larger than the monitor, it pins to the monitor origin.
pub fn clamp_popup_origin(x: i32, y: i32, w: i32, h: i32, mon: MonitorRect) -> (i32, i32) {
    let mon_right = mon.x.saturating_add(mon.w);
    let mon_bottom = mon.y.saturating_add(mon.h);
    let max_x = (mon_right - w).max(mon.x);
    let max_y = (mon_bottom - h).max(mon.y);
    (x.clamp(mon.x, max_x), y.clamp(mon.y, max_y))
}

/// Place a cursor / context menu with preferred top-left `(x, y)` on the
/// monitor that contains that point.
pub fn place_cursor_popup(x: i32, y: i32, w: i32, h: i32) -> (i32, i32) {
    let mon = monitor_for_point(x, y);
    clamp_popup_origin(x, y, w, h, mon)
}

/// Center under the button; flip above when needed; clamp into `mon`.
pub fn place_tip_near_button(
    btn_x: i32,
    btn_y: i32,
    btn_w: i32,
    btn_h: i32,
    tip_w: i32,
    tip_h: i32,
    mon: MonitorRect,
) -> (i32, i32) {
    let tip_x = btn_x + (btn_w - tip_w) / 2;
    let mut tip_y = btn_y + btn_h + TIP_GAP_PX;
    let mon_bottom = mon.y.saturating_add(mon.h);
    if tip_y.saturating_add(tip_h) > mon_bottom {
        tip_y = btn_y - TIP_GAP_PX - tip_h;
    }
    clamp_popup_origin(tip_x, tip_y, tip_w, tip_h, mon)
}

/// True when `(px, py)` lies inside the axis-aligned popup rect.
pub fn point_in_popup(px: i32, py: i32, x: i32, y: i32, w: i32, h: i32) -> bool {
    px >= x && py >= y && px < x.saturating_add(w) && py < y.saturating_add(h)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clamp_keeps_menu_on_screen_near_edges() {
        let mon = MonitorRect {
            x: 0,
            y: 0,
            w: 1000,
            h: 800,
        };
        assert_eq!(clamp_popup_origin(950, 750, 120, 80, mon), (880, 720));
        assert_eq!(clamp_popup_origin(-20, -10, 120, 80, mon), (0, 0));
        assert_eq!(clamp_popup_origin(100, 100, 120, 80, mon), (100, 100));
    }

    #[test]
    fn clamp_on_offset_monitor() {
        let mon = MonitorRect {
            x: 1920,
            y: 0,
            w: 1920,
            h: 1080,
        };
        assert_eq!(clamp_popup_origin(3800, 1000, 200, 150, mon), (3640, 930));
    }

    #[test]
    fn tip_flips_above_when_below_would_clip() {
        let mon = MonitorRect {
            x: 0,
            y: 0,
            w: 400,
            h: 300,
        };
        let (x, y) = place_tip_near_button(100, 260, 40, 40, 80, 30, mon);
        assert_eq!((x, y), (80, 260 - TIP_GAP_PX - 30));
        assert!(x >= mon.x && y >= mon.y);
        assert!(x + 80 <= mon.x + mon.w && y + 30 <= mon.y + mon.h);
    }

    #[test]
    fn tip_clamps_horizontally_at_edge() {
        let mon = MonitorRect {
            x: 0,
            y: 0,
            w: 200,
            h: 400,
        };
        let (x, y) = place_tip_near_button(180, 20, 40, 40, 100, 24, mon);
        assert_eq!(x, 100);
        assert_eq!(y, 20 + 40 + TIP_GAP_PX);
    }

    #[test]
    fn point_in_popup_edges() {
        assert!(point_in_popup(10, 10, 10, 10, 50, 40));
        assert!(!point_in_popup(60, 10, 10, 10, 50, 40));
        assert!(!point_in_popup(10, 50, 10, 10, 50, 40));
    }
}
