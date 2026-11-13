//! Hub radial-fan travel shared by native overlay hosts.
//!
//! Animation runs on the host thread (not egui ROOT) so satellites keep moving
//! while Sqyre is unfocused — Wayland/XWayland starve background frames the
//! same way fullscreen games do for the busy spinner.

use sqyre_persist::{QUICK_ACCESS_HUB_DISMISS, QUICK_ACCESS_HUB_ID};
use std::collections::{HashMap, HashSet};
use std::time::Instant;

/// Fan open/close duration (seconds) for satellite travel.
pub const HUB_FAN_ANIM_SECS: f32 = 0.18;

/// Host poll interval while the fan is traveling.
pub const HUB_FAN_TICK_MS: u64 = 16;

/// Ease fan progress (`0`…`1`) with a smoothstep.
pub fn hub_fan_ease(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Radial action button id (travels with the fan).
pub fn is_hub_satellite_id(id: &str) -> bool {
    id.starts_with("__sqyre_hub") && id != QUICK_ACCESS_HUB_ID && id != QUICK_ACCESS_HUB_DISMISS
}

/// Hub face or satellite (outside-click dismiss chrome).
pub fn is_hub_chrome_id(id: &str) -> bool {
    id.starts_with("__sqyre_hub")
}

/// Host-owned fan progress and rested satellite slots.
#[derive(Debug, Clone)]
pub struct HubFanState {
    /// Open amount (`0` closed … `1` fully open).
    pub t: f32,
    /// Expanded/collapsed target.
    pub target: f32,
    last_tick: Instant,
    /// Fully-open top-left per satellite id (from app specs).
    rest: HashMap<String, (i32, i32)>,
}

impl Default for HubFanState {
    fn default() -> Self {
        Self {
            t: 0.0,
            target: 0.0,
            last_tick: Instant::now(),
            rest: HashMap::new(),
        }
    }
}

impl HubFanState {
    /// Aim the fan open or closed. `snap` skips travel (mode switches).
    pub fn set_expanded(&mut self, expanded: bool, snap: bool) {
        let target = if expanded { 1.0 } else { 0.0 };
        self.target = target;
        if snap {
            self.t = target;
            self.last_tick = Instant::now();
        }
    }

    pub fn animating(&self) -> bool {
        (self.t - self.target).abs() >= 0.001
    }

    pub fn fully_open(&self) -> bool {
        self.t >= 0.999 && self.target >= 0.999
    }

    /// True while open or still collapsing (outside-dismiss should stay armed).
    pub fn open_or_traveling(&self) -> bool {
        self.target > 0.001 || self.t > 0.001
    }

    /// Advance `t` toward `target` using wall time since the last tick.
    /// Returns true while travel is still in progress (including the settle frame).
    pub fn tick(&mut self) -> bool {
        let dt = self
            .last_tick
            .elapsed()
            .as_secs_f32()
            .clamp(1.0 / 240.0, 1.0 / 30.0);
        self.last_tick = Instant::now();
        self.tick_dt(dt)
    }

    /// Advance `t` by an explicit `dt` (tests / paced hosts).
    pub fn tick_dt(&mut self, dt: f32) -> bool {
        if !self.animating() {
            self.t = self.target;
            return false;
        }
        let step = dt.clamp(1.0 / 240.0, 1.0 / 30.0) / HUB_FAN_ANIM_SECS;
        if self.t < self.target {
            self.t = (self.t + step).min(self.target);
        } else {
            self.t = (self.t - step).max(self.target);
        }
        true
    }

    pub fn remember_rest(&mut self, id: &str, x: i32, y: i32) {
        if is_hub_satellite_id(id) {
            self.rest.insert(id.to_string(), (x, y));
        }
    }

    pub fn rest_of(&self, id: &str) -> Option<(i32, i32)> {
        self.rest.get(id).copied()
    }

    pub fn satellite_ids(&self) -> Vec<String> {
        self.rest.keys().cloned().collect()
    }

    pub fn forget_missing(&mut self, keep: &HashSet<String>) {
        self.rest.retain(|id, _| keep.contains(id));
    }
}

/// Collapsed satellite center on the hub face.
pub fn hub_origin(
    hub_x: i32,
    hub_y: i32,
    hub_w: u32,
    hub_h: u32,
    sat_w: u32,
    sat_h: u32,
) -> (i32, i32) {
    (
        hub_x + (hub_w as i32 - sat_w as i32) / 2,
        hub_y + (hub_h as i32 - sat_h as i32) / 2,
    )
}

/// Off-desktop park for a closed fan (no paint, no hit stealing).
///
/// A small hub-relative offset is not enough: a right- or bottom-docked hub
/// leaves `hub - 8×size` still on the same monitor, so collapsed satellites
/// stay visible and can sit under the pointer. Use a coordinate well outside
/// any practical virtual-desktop span (multi-4K walls, negative origins, …).
pub fn hub_park(_hub_x: i32, _hub_y: i32, _sat_w: u32, _sat_h: u32) -> (i32, i32) {
    const OFF: i32 = -100_000;
    (OFF, OFF)
}

/// Visual top-left for a satellite at `fan_t`.
pub fn satellite_pos(
    fan_t: f32,
    rest: (i32, i32),
    origin: (i32, i32),
    park: (i32, i32),
) -> (i32, i32) {
    let eased = hub_fan_ease(fan_t);
    if eased <= 0.001 {
        return park;
    }
    (
        origin.0 + ((rest.0 - origin.0) as f32 * eased).round() as i32,
        origin.1 + ((rest.1 - origin.1) as f32 * eased).round() as i32,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ease_endpoints() {
        assert_eq!(hub_fan_ease(0.0), 0.0);
        assert!((hub_fan_ease(1.0) - 1.0).abs() < f32::EPSILON);
        assert!(hub_fan_ease(0.5) > 0.4 && hub_fan_ease(0.5) < 0.6);
    }

    #[test]
    fn satellite_ids() {
        assert!(!is_hub_satellite_id(QUICK_ACCESS_HUB_ID));
        assert!(is_hub_satellite_id("__sqyre_hub_new_point"));
        assert!(!is_hub_satellite_id(QUICK_ACCESS_HUB_DISMISS));
        assert!(is_hub_chrome_id("__sqyre_hub_new_point"));
        assert!(is_hub_chrome_id(QUICK_ACCESS_HUB_ID));
    }

    #[test]
    fn tick_reaches_target() {
        let mut fan = HubFanState::default();
        fan.set_expanded(true, false);
        let dt = HUB_FAN_TICK_MS as f32 / 1000.0;
        let mut steps = 0;
        while fan.animating() && steps < 100 {
            assert!(fan.tick_dt(dt));
            steps += 1;
        }
        assert!(fan.fully_open(), "t={}", fan.t);
        assert!(!fan.tick_dt(dt));
    }

    #[test]
    fn snap_skips_travel() {
        let mut fan = HubFanState::default();
        fan.set_expanded(true, true);
        assert!(fan.fully_open());
        assert!(!fan.animating());
        fan.set_expanded(false, true);
        assert_eq!(fan.t, 0.0);
        assert!(!fan.open_or_traveling());
    }

    #[test]
    fn pos_parks_when_closed() {
        let origin = (10, 20);
        let rest = (100, 200);
        let park = (-50, -50);
        assert_eq!(satellite_pos(0.0, rest, origin, park), park);
        assert_eq!(satellite_pos(1.0, rest, origin, park), rest);
        let mid = satellite_pos(0.5, rest, origin, park);
        assert!(mid.0 > origin.0 && mid.0 < rest.0);
    }

    #[test]
    fn closed_park_is_off_desktop_for_edge_hub() {
        // Right-edge hub on a 1920×1080 monitor — the old 8×size offset left
        // satellites around x≈1500, still on-screen.
        let hub_x = 1920 - 40;
        let hub_y = 500;
        let (px, py) = hub_park(hub_x, hub_y, 36, 36);
        assert!(px < 0 && py < 0, "park on-desktop: ({px},{py})");
        assert!(
            px < -10_000 && py < -10_000,
            "park must clear multi-monitor desktops: ({px},{py})"
        );
        let origin = hub_origin(hub_x, hub_y, 40, 40, 36, 36);
        let rest = (hub_x - 80, hub_y);
        assert_eq!(satellite_pos(0.0, rest, origin, (px, py)), (px, py));
    }
}
