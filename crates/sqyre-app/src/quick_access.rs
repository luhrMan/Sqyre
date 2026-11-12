//! Edge-docked quick-access hub: brand button + inward half-circle actions.

use crate::data_editor::{DataEditorCtx, EditorTab};
use crate::SqyreApp;
use eframe::egui;
use sqyre_persist::QUICK_ACCESS_HUB_ID;
#[cfg(all(feature = "native-runtime", feature = "overlay-buttons"))]
use sqyre_persist::{
    hub_position_on_monitor, snap_hub_to_monitor, HubEdge, OverlayButtonConfig,
    DEFAULT_OVERLAY_ACCENT_HEX, DEFAULT_OVERLAY_BORDER_WIDTH, DEFAULT_OVERLAY_ICON_HEX,
    QUICK_ACCESS_HUB_ICON,
};

/// Fire tokens for radial satellites (also used as overlay button ids).
pub(crate) const HUB_NEW_POINT: &str = "__sqyre_hub_new_point";
pub(crate) const HUB_NEW_AREA: &str = "__sqyre_hub_new_area";
pub(crate) const HUB_SCREEN_CAP: &str = "__sqyre_hub_screen_cap";
pub(crate) const HUB_TOGGLE_WINDOW: &str = "__sqyre_hub_toggle";

/// Actions offered by the hub fan.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum QuickAccessAction {
    NewPoint,
    NewSearchArea,
    ScreenCap,
    ToggleWindow,
}

impl QuickAccessAction {
    #[cfg(all(feature = "native-runtime", feature = "overlay-buttons"))]
    fn icon_id(self) -> &'static str {
        match self {
            Self::NewPoint => "map-pin",
            Self::NewSearchArea => "selection",
            Self::ScreenCap => "camera",
            Self::ToggleWindow => "eye",
        }
    }

    #[cfg(all(feature = "native-runtime", feature = "overlay-buttons"))]
    fn token(self) -> &'static str {
        match self {
            Self::NewPoint => HUB_NEW_POINT,
            Self::NewSearchArea => HUB_NEW_AREA,
            Self::ScreenCap => HUB_SCREEN_CAP,
            Self::ToggleWindow => HUB_TOGGLE_WINDOW,
        }
    }

    fn from_token(name: &str) -> Option<Self> {
        match name.trim() {
            HUB_NEW_POINT => Some(Self::NewPoint),
            HUB_NEW_AREA => Some(Self::NewSearchArea),
            HUB_SCREEN_CAP => Some(Self::ScreenCap),
            HUB_TOGGLE_WINDOW => Some(Self::ToggleWindow),
            _ => None,
        }
    }
}

#[cfg(all(feature = "native-runtime", feature = "overlay-buttons"))]
const ACTIONS: [QuickAccessAction; 4] = [
    QuickAccessAction::NewPoint,
    QuickAccessAction::NewSearchArea,
    QuickAccessAction::ScreenCap,
    QuickAccessAction::ToggleWindow,
];

/// Fan open/close duration (seconds) for satellite travel.
#[cfg(all(feature = "native-runtime", feature = "overlay-buttons"))]
pub(crate) const HUB_FAN_ANIM_SECS: f32 = 0.18;

/// Ease fan progress (`0`…`1`) with a smoothstep.
#[cfg(all(feature = "native-runtime", feature = "overlay-buttons"))]
pub(crate) fn hub_fan_ease(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Build hub (+ radial satellites) for the overlay host from the seeded
/// General overlay button.
///
/// `fan_t` is the open amount (`0` closed … `1` fully open). Satellites travel
/// between the hub center and their rested fan slots (and stay in the list when
/// closed so native overlay windows stay warm). `ppp` converts the button size
/// (points) into physical desktop pixels so high-DPI faces do not overlap when
/// layout uses the same space as overlay window positions. Stored `x`/`y` are
/// edge-snapped desktop coords; fan direction comes from the nearest edge.
#[cfg(all(feature = "native-runtime", feature = "overlay-buttons"))]
pub(crate) fn hub_overlay_buttons(
    hub: &OverlayButtonConfig,
    fan_t: f32,
    ppp: f32,
) -> Vec<OverlayButtonConfig> {
    if !hub.enabled {
        return Vec::new();
    }
    let ppp = ppp.max(0.01);
    let size_pt = hub.size.clamp(
        sqyre_persist::MIN_OVERLAY_BUTTON_SIZE,
        sqyre_persist::MAX_OVERLAY_BUTTON_SIZE,
    );
    let size_px = (size_pt * ppp).round().max(1.0);
    let (mon_x, mon_y, mon_w, mon_h) = monitor_for_point(hub.x, hub.y);
    let (edge, offset) = snap_hub_to_monitor(hub.x, hub.y, size_px, mon_x, mon_y, mon_w, mon_h);
    let (hx, hy) = hub_position_on_monitor(edge, offset, mon_x, mon_y, mon_w, mon_h, size_px);
    let opacity = hub.icon_alpha;
    let mut out = vec![hub_face(hub, hx, hy, size_pt)];
    // Always emit satellites while the hub is enabled so the overlay host keeps
    // their windows warm. Creating four face+hit windows on first open stalled
    // XWayland (~1s cursor/screen freeze). Closed satellites park off-desktop
    // (no hit stealing); open only MoveResizes them along the fan.
    let sat_size_pt = (size_pt * 0.92).clamp(
        sqyre_persist::MIN_OVERLAY_BUTTON_SIZE,
        sqyre_persist::MAX_OVERLAY_BUTTON_SIZE,
    );
    let sat_size_px = (sat_size_pt * ppp).round().max(1.0);
    let layout = SatelliteLayout {
        hub_x: hx,
        hub_y: hy,
        hub_size: size_px,
        sat_size: sat_size_px,
        edge,
        radius: size_px * 2.15,
        mon_x,
        mon_y,
        mon_w,
        mon_h,
        count: ACTIONS.len(),
    };
    // Collapsed origin: satellite centered on the hub face.
    let origin_x = hx + (size_px - sat_size_px) * 0.5;
    let origin_y = hy + (size_px - sat_size_px) * 0.5;
    let eased = hub_fan_ease(fan_t);
    // Park far off every monitor so a closed fan cannot sit under the pointer.
    let park_x = mon_x - sat_size_px * 8.0 - 64.0;
    let park_y = mon_y - sat_size_px * 8.0 - 64.0;
    for (i, action) in ACTIONS.iter().enumerate() {
        let (sx, sy) = satellite_top_left(layout, i);
        let (ax, ay) = if eased <= 0.001 {
            (park_x, park_y)
        } else {
            (
                origin_x + (sx - origin_x) * eased,
                origin_y + (sy - origin_y) * eased,
            )
        };
        // Full opacity for the whole travel — fading forced a style repaint
        // (and X shape refresh) on every animation frame.
        out.push(satellite_face(action, ax, ay, sat_size_pt, opacity));
    }
    out
}

#[cfg(all(feature = "native-runtime", feature = "overlay-buttons"))]
#[derive(Clone, Copy)]
struct SatelliteLayout {
    hub_x: f32,
    hub_y: f32,
    hub_size: f32,
    sat_size: f32,
    edge: HubEdge,
    radius: f32,
    mon_x: f32,
    mon_y: f32,
    mon_w: f32,
    mon_h: f32,
    count: usize,
}

#[cfg(all(feature = "native-runtime", feature = "overlay-buttons"))]
fn hub_disk_radius(size_pt: f32) -> f32 {
    (size_pt * 0.5).clamp(
        sqyre_persist::MIN_OVERLAY_CORNER_RADIUS,
        sqyre_persist::MAX_OVERLAY_CORNER_RADIUS,
    )
}

#[cfg(all(feature = "native-runtime", feature = "overlay-buttons"))]
fn hub_face(hub: &OverlayButtonConfig, x: f32, y: f32, size_pt: f32) -> OverlayButtonConfig {
    let mut face = hub.clone();
    face.id = QUICK_ACCESS_HUB_ID.into();
    face.macro_name = QUICK_ACCESS_HUB_ID.into();
    face.icon = QUICK_ACCESS_HUB_ICON.into();
    face.point.clear();
    face.x = x;
    face.y = y;
    face.size = size_pt;
    // Hit disk only — the face paints the full Sqyre logo with no chrome.
    face.corner_radius = hub_disk_radius(size_pt);
    face.border_width = 0.0;
    face.border_alpha = 0;
    face.bg_alpha = 0;
    if face.icon_color.trim().is_empty() {
        face.icon_color = DEFAULT_OVERLAY_ICON_HEX.into();
    }
    if face.icon_hover_color.trim().is_empty() {
        face.icon_hover_color = DEFAULT_OVERLAY_ACCENT_HEX.into();
    }
    face
}

#[cfg(all(feature = "native-runtime", feature = "overlay-buttons"))]
fn satellite_face(
    action: &QuickAccessAction,
    x: f32,
    y: f32,
    size: f32,
    opacity: u8,
) -> OverlayButtonConfig {
    OverlayButtonConfig {
        id: action.token().into(),
        program: String::new(),
        enabled: true,
        // Fire token drained by the app (tip is mapped from id in the overlay host).
        macro_name: action.token().into(),
        icon: action.icon_id().into(),
        point: String::new(),
        x,
        y,
        size,
        corner_radius: hub_disk_radius(size),
        border_width: DEFAULT_OVERLAY_BORDER_WIDTH,
        border_color: DEFAULT_OVERLAY_ACCENT_HEX.into(),
        border_alpha: opacity,
        bg_color: "#000000".into(),
        bg_alpha: opacity.saturating_mul(200) / 255,
        icon_color: DEFAULT_OVERLAY_ICON_HEX.into(),
        icon_alpha: opacity,
        icon_hover_color: DEFAULT_OVERLAY_ACCENT_HEX.into(),
        visibility_gate: Default::default(),
    }
}

/// Top-left of satellite `i` on an inward fan that stays inside the monitor.
#[cfg(all(feature = "native-runtime", feature = "overlay-buttons"))]
fn satellite_top_left(layout: SatelliteLayout, index: usize) -> (f32, f32) {
    let cx = layout.hub_x + layout.hub_size * 0.5;
    let cy = layout.hub_y + layout.hub_size * 0.5;
    let half = layout.sat_size * 0.5;
    let room_left = (cx - layout.mon_x - half).max(0.0);
    let room_right = (layout.mon_x + layout.mon_w - cx - half).max(0.0);
    let room_up = (cy - layout.mon_y - half).max(0.0);
    let room_down = (layout.mon_y + layout.mon_h - cy - half).max(0.0);
    let room_in = match layout.edge {
        HubEdge::Right => room_left,
        HubEdge::Left => room_right,
        HubEdge::Top => room_down,
        HubEdge::Bottom => room_up,
    };
    // Keep the fan radius inside the inward room; tangent room trims the sweep.
    let radius = layout
        .radius
        .min(room_in)
        .max((layout.hub_size * 0.75).min(room_in.max(1.0)));
    let (start, sweep) = inward_fan_range(
        layout.edge,
        radius,
        room_left,
        room_right,
        room_up,
        room_down,
    );
    let t = if layout.count <= 1 {
        0.5
    } else {
        index as f32 / (layout.count as f32 - 1.0)
    };
    let angle = start + sweep * t;
    let sx = cx + radius * angle.cos() - half;
    let sy = cy + radius * angle.sin() - half;
    clamp_rect_to_monitor(
        sx,
        sy,
        layout.sat_size,
        layout.mon_x,
        layout.mon_y,
        layout.mon_w,
        layout.mon_h,
    )
}

/// Inward half-circle, trimmed when the hub sits near a corner.
#[cfg(all(feature = "native-runtime", feature = "overlay-buttons"))]
fn inward_fan_range(
    edge: HubEdge,
    radius: f32,
    room_left: f32,
    room_right: f32,
    room_up: f32,
    room_down: f32,
) -> (f32, f32) {
    let r = radius.max(1.0);
    let max_left = (room_left / r).clamp(0.0, 1.0).asin();
    let max_right = (room_right / r).clamp(0.0, 1.0).asin();
    let max_up = (room_up / r).clamp(0.0, 1.0).asin();
    let max_down = (room_down / r).clamp(0.0, 1.0).asin();
    match edge {
        // Inward π: start = π - max_down → end = π + max_up (full: π/2 → 3π/2).
        HubEdge::Right => (
            std::f32::consts::PI - max_down,
            (max_down + max_up).max(0.01),
        ),
        // Inward 0: start = -max_up → end = max_down (full: -π/2 → π/2).
        HubEdge::Left => (-max_up, (max_up + max_down).max(0.01)),
        // Inward π/2: start = π/2 - max_right → end = π/2 + max_left (full: 0 → π).
        HubEdge::Top => (
            std::f32::consts::FRAC_PI_2 - max_right,
            (max_right + max_left).max(0.01),
        ),
        // Inward 3π/2: start = 3π/2 - max_left → end = 3π/2 + max_right (full: π → 2π).
        HubEdge::Bottom => (
            3.0 * std::f32::consts::FRAC_PI_2 - max_left,
            (max_left + max_right).max(0.01),
        ),
    }
}

#[cfg(all(feature = "native-runtime", feature = "overlay-buttons"))]
fn clamp_rect_to_monitor(
    x: f32,
    y: f32,
    size: f32,
    mon_x: f32,
    mon_y: f32,
    mon_w: f32,
    mon_h: f32,
) -> (f32, f32) {
    let max_x = (mon_x + mon_w - size).max(mon_x);
    let max_y = (mon_y + mon_h - size).max(mon_y);
    (x.clamp(mon_x, max_x), y.clamp(mon_y, max_y))
}

#[cfg(all(feature = "native-runtime", feature = "overlay-buttons"))]
fn primary_monitor_points() -> (f32, f32, f32, f32) {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let rects = sqyre_capture::preferred_monitor_rects();
        if let Some(r) = rects.first() {
            return (r.x as f32, r.y as f32, r.w as f32, r.h as f32);
        }
    }
    (
        0.0,
        0.0,
        sqyre_persist::DEFAULT_OVERLAY_FALLBACK_SCREEN_W,
        sqyre_persist::DEFAULT_OVERLAY_FALLBACK_SCREEN_H,
    )
}

/// Monitor containing `(x, y)`, or primary.
#[cfg(all(feature = "native-runtime", feature = "overlay-buttons"))]
pub(crate) fn monitor_for_point(x: f32, y: f32) -> (f32, f32, f32, f32) {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let rects = sqyre_capture::preferred_monitor_rects();
        for r in &rects {
            let rx = r.x as f32;
            let ry = r.y as f32;
            let rw = r.w as f32;
            let rh = r.h as f32;
            if x >= rx && x < rx + rw && y >= ry && y < ry + rh {
                return (rx, ry, rw, rh);
            }
        }
        if let Some(r) = rects.first() {
            return (r.x as f32, r.y as f32, r.w as f32, r.h as f32);
        }
    }
    let _ = (x, y);
    primary_monitor_points()
}

impl SqyreApp {
    /// True when the pending fire token is the hub face (toggle expand).
    fn is_quick_access_hub_token(name: &str) -> bool {
        name.trim() == QUICK_ACCESS_HUB_ID
    }

    pub(crate) fn toggle_quick_access_fan(&mut self, ctx: &egui::Context) {
        self.quick_access_expanded = !self.quick_access_expanded;
        ctx.request_repaint();
    }

    pub(crate) fn close_quick_access_fan(&mut self) {
        self.quick_access_expanded = false;
    }

    /// Snap the fan shut without playing the collapse motion (mode switches).
    #[cfg(all(feature = "native-runtime", feature = "overlay-buttons"))]
    pub(crate) fn snap_quick_access_fan_closed(&mut self) {
        self.quick_access_expanded = false;
        self.quick_access_fan_t = 0.0;
    }

    /// Advance satellite travel toward the expanded/collapsed target.
    #[cfg(all(feature = "native-runtime", feature = "overlay-buttons"))]
    pub(crate) fn tick_quick_access_fan(&mut self, ctx: &egui::Context) {
        let target = if self.quick_access_expanded { 1.0 } else { 0.0 };
        if (self.quick_access_fan_t - target).abs() < 0.001 {
            self.quick_access_fan_t = target;
            return;
        }
        let dt = ctx.input(|i| i.stable_dt).clamp(1.0 / 240.0, 1.0 / 30.0);
        let step = dt / HUB_FAN_ANIM_SECS;
        if self.quick_access_fan_t < target {
            self.quick_access_fan_t = (self.quick_access_fan_t + step).min(target);
        } else {
            self.quick_access_fan_t = (self.quick_access_fan_t - step).max(target);
        }
        ctx.request_repaint();
    }

    /// Esc collapses an open fan.
    pub(crate) fn poll_quick_access_escape(&mut self, ctx: &egui::Context) {
        if !self.quick_access_expanded && self.quick_access_fan_t <= 0.001 {
            return;
        }
        sqyre_hotkeys::claim_popup_escape(true);
        if sqyre_hotkeys::take_popup_escape() {
            self.close_quick_access_fan();
            ctx.request_repaint();
        }
    }

    pub(crate) fn dispatch_quick_access_token(&mut self, token: &str, ctx: &egui::Context) -> bool {
        if Self::is_quick_access_hub_token(token) {
            self.toggle_quick_access_fan(ctx);
            return true;
        }
        if token.trim() == sqyre_persist::QUICK_ACCESS_HUB_DISMISS {
            self.close_quick_access_fan();
            ctx.request_repaint();
            return true;
        }
        let Some(action) = QuickAccessAction::from_token(token) else {
            return false;
        };
        // Satellites fire only when the fan is fully open (host also empties hits).
        if (self.quick_access_fan_t - 1.0).abs() > 0.001 || !self.quick_access_expanded {
            return true;
        }
        self.close_quick_access_fan();
        self.dispatch_quick_access(action, ctx);
        true
    }

    pub(crate) fn dispatch_quick_access(&mut self, action: QuickAccessAction, ctx: &egui::Context) {
        match action {
            QuickAccessAction::NewPoint => self.quick_access_new_entity(EditorTab::Points, ctx),
            QuickAccessAction::NewSearchArea => {
                self.quick_access_new_entity(EditorTab::SearchAreas, ctx)
            }
            QuickAccessAction::ScreenCap => self.quick_access_screen_cap(ctx),
            QuickAccessAction::ToggleWindow => self.quick_access_toggle_window(ctx),
        }
    }

    fn quick_access_new_entity(&mut self, tab: EditorTab, ctx: &egui::Context) {
        self.refresh_focused_program_hint();
        let pending = self.pending_viewport_scale;
        self.data_editor.open_new(
            tab,
            &mut DataEditorCtx {
                ctx,
                db: &mut self.workspace.db,
                macros: &mut self.workspace.macros,
                catalog: &mut self.workspace.catalog,
                icons: &mut self.icon_cache,
                screen_click: &self.screen_click,
                settings: self.settings_ui.settings_mut(),
                hotkey_tag_filters: &mut self.workspace.hotkey_tag_filters,
                pending_scale: pending.as_ref(),
            },
        );
    }

    fn quick_access_screen_cap(&mut self, ctx: &egui::Context) {
        self.refresh_focused_program_hint();
        self.data_editor.open_tab(
            ctx,
            EditorTab::ScreenCap,
            &self.workspace.catalog,
            self.settings_ui.settings(),
        );
        self.data_editor
            .prefer_focused_program_for_new(&self.workspace.catalog, self.settings_ui.settings());
        if !self.screen_click.is_armed() {
            self.screen_click.arm_search_area();
            self.data_editor
                .set_ok("Screen Cap: click two corners to capture.");
        }
    }

    /// Snapshot the focused catalog program for New / Screen Cap targeting.
    pub(crate) fn refresh_focused_program_hint(&mut self) {
        #[cfg(all(feature = "native-runtime", not(target_arch = "wasm32")))]
        {
            let hint = self.macro_overlay.last_foreign_focus();
            let program = crate::hotkey_focus_tags::focused_editor_program(
                &self.workspace.catalog,
                hint.as_ref(),
            );
            self.data_editor.set_focused_program_hint(program);
        }
        #[cfg(any(not(feature = "native-runtime"), target_arch = "wasm32"))]
        {
            self.data_editor.set_focused_program_hint(None);
        }
    }

    fn quick_access_toggle_window(&mut self, ctx: &egui::Context) {
        #[cfg(not(target_arch = "wasm32"))]
        {
            self.tray.toggle_visible(ctx);
        }
        #[cfg(target_arch = "wasm32")]
        {
            let _ = ctx;
        }
    }
}

#[cfg(all(feature = "native-runtime", feature = "overlay-buttons"))]
#[cfg(test)]
mod tests {
    use super::*;
    use sqyre_persist::HubEdge;

    #[test]
    fn satellites_fan_inward_from_right_edge() {
        let layout = SatelliteLayout {
            hub_x: 1894.0,
            hub_y: 500.0,
            hub_size: 26.0,
            sat_size: 24.0,
            edge: HubEdge::Right,
            radius: 56.0,
            mon_x: 0.0,
            mon_y: 0.0,
            mon_w: 1920.0,
            mon_h: 1080.0,
            count: 4,
        };
        let (_x0, y0) = satellite_top_left(layout, 0);
        let (x1, _y1) = satellite_top_left(layout, 1);
        let (_x2, y2) = satellite_top_left(layout, 3);
        // Screen Y grows downward: start is below, end is above.
        assert!(y0 > layout.hub_y + layout.hub_size * 0.5);
        assert!(y2 < layout.hub_y + layout.hub_size * 0.5);
        assert!(x1 < layout.hub_x);
    }

    #[test]
    fn satellites_stay_on_monitor_near_corner() {
        let layout = SatelliteLayout {
            hub_x: 1894.0,
            hub_y: 0.0,
            hub_size: 26.0,
            sat_size: 24.0,
            edge: HubEdge::Right,
            radius: 56.0,
            mon_x: 0.0,
            mon_y: 0.0,
            mon_w: 1920.0,
            mon_h: 1080.0,
            count: 4,
        };
        for i in 0..4 {
            let (x, y) = satellite_top_left(layout, i);
            assert!(x >= layout.mon_x, "sat {i} x={x}");
            assert!(y >= layout.mon_y, "sat {i} y={y}");
            assert!(x + layout.sat_size <= layout.mon_x + layout.mon_w + 0.5);
            assert!(y + layout.sat_size <= layout.mon_y + layout.mon_h + 0.5);
        }
    }

    fn test_hub_button() -> OverlayButtonConfig {
        let mut hub = sqyre_persist::default_quick_access_hub_button();
        let (x, y) = hub_position_on_monitor(HubEdge::Right, 0.5, 0.0, 0.0, 1920.0, 1080.0, 26.0);
        hub.x = x;
        hub.y = y;
        hub.size = 26.0;
        hub.icon_alpha = 220;
        hub
    }

    #[test]
    fn hub_buttons_are_disks() {
        let hub = test_hub_button();
        let btns = hub_overlay_buttons(&hub, 1.0, 1.0);
        assert_eq!(btns.len(), 5);
        for btn in &btns {
            assert!(
                (btn.corner_radius - btn.size * 0.5).abs() < f32::EPSILON,
                "id={} r={} size={}",
                btn.id,
                btn.corner_radius,
                btn.size
            );
        }
        let face = &btns[0];
        assert_eq!(face.id, QUICK_ACCESS_HUB_ID);
        assert_eq!(face.border_width, 0.0);
        assert_eq!(face.bg_alpha, 0);
        assert_eq!(face.border_alpha, 0);
        assert_eq!(face.icon, QUICK_ACCESS_HUB_ICON);
    }

    #[test]
    fn disabled_hub_emits_nothing() {
        let mut hub = test_hub_button();
        hub.enabled = false;
        assert!(hub_overlay_buttons(&hub, 1.0, 1.0).is_empty());
    }

    #[test]
    fn high_dpi_satellites_do_not_overlap() {
        let hub = test_hub_button();
        let ppp = 2.0;
        let btns = hub_overlay_buttons(&hub, 1.0, ppp);
        assert_eq!(btns.len(), 5);
        let sats: Vec<_> = btns
            .iter()
            .filter(|b| b.id != QUICK_ACCESS_HUB_ID)
            .collect();
        let phys = |b: &OverlayButtonConfig| (b.size * ppp).round();
        for (i, a) in sats.iter().enumerate() {
            for b in sats.iter().skip(i + 1) {
                let ax = a.x + phys(a) * 0.5;
                let ay = a.y + phys(a) * 0.5;
                let bx = b.x + phys(b) * 0.5;
                let by = b.y + phys(b) * 0.5;
                let dist = ((ax - bx).hypot(ay - by)).max(0.0);
                let min_sep = (phys(a) + phys(b)) * 0.5;
                assert!(
                    dist + 0.5 >= min_sep,
                    "satellites overlap at {ppp}×: dist={dist} min={min_sep}"
                );
            }
        }
    }

    #[test]
    fn fan_animation_travels_out_from_hub() {
        let hub = test_hub_button();
        let closed = hub_overlay_buttons(&hub, 0.0, 1.0);
        assert_eq!(closed.len(), 5, "closed fan keeps warm satellites");
        for sat in closed.iter().skip(1) {
            assert!(
                sat.x < 0.0 && sat.y < 0.0,
                "closed satellite parks off-desktop: {} @ {},{}",
                sat.id,
                sat.x,
                sat.y
            );
            assert_eq!(
                sat.border_alpha, hub.icon_alpha,
                "no opacity fade while traveling"
            );
        }
        let mid = hub_overlay_buttons(&hub, 0.5, 1.0);
        let open = hub_overlay_buttons(&hub, 1.0, 1.0);
        assert_eq!(mid.len(), 5);
        assert_eq!(open.len(), 5);
        let hub_btn = &open[0];
        let hub_cx = hub_btn.x + hub_btn.size * 0.5;
        let hub_cy = hub_btn.y + hub_btn.size * 0.5;
        for i in 1..5 {
            let m = &mid[i];
            let o = &open[i];
            let m_cx = m.x + m.size * 0.5;
            let m_cy = m.y + m.size * 0.5;
            let o_cx = o.x + o.size * 0.5;
            let o_cy = o.y + o.size * 0.5;
            let mid_dist = (m_cx - hub_cx).hypot(m_cy - hub_cy);
            let open_dist = (o_cx - hub_cx).hypot(o_cy - hub_cy);
            assert!(
                mid_dist + 0.5 < open_dist,
                "sat {i} should travel outward: mid={mid_dist} open={open_dist}"
            );
        }
        assert!(hub_fan_ease(0.0) == 0.0);
        assert!((hub_fan_ease(1.0) - 1.0).abs() < f32::EPSILON);
        assert!(hub_fan_ease(0.5) > 0.4 && hub_fan_ease(0.5) < 0.6);
    }

    #[test]
    fn hub_dismiss_token_is_reserved() {
        assert_eq!(
            sqyre_persist::QUICK_ACCESS_HUB_DISMISS,
            "__sqyre_hub_dismiss"
        );
        assert!(QuickAccessAction::from_token(sqyre_persist::QUICK_ACCESS_HUB_DISMISS).is_none());
    }
}
