//! Edge-docked quick-access hub: brand button + inward half-circle actions.

use crate::data_editor::{DataEditorCtx, EditorTab};
use crate::SqyreApp;
use eframe::egui;
use sqyre_persist::QUICK_ACCESS_HUB_ID;
#[cfg(all(feature = "native-runtime", feature = "overlay-buttons"))]
use sqyre_persist::{
    HubEdge, OverlayButtonConfig, QuickAccessHubSettings, DEFAULT_OVERLAY_ACCENT_HEX,
    DEFAULT_OVERLAY_BORDER_WIDTH, DEFAULT_OVERLAY_CORNER_RADIUS, DEFAULT_OVERLAY_ICON_HEX,
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

/// Build hub (+ optional radial satellites) for the overlay host.
#[cfg(all(feature = "native-runtime", feature = "overlay-buttons"))]
pub(crate) fn hub_overlay_buttons(
    hub: &QuickAccessHubSettings,
    expanded: bool,
) -> Vec<OverlayButtonConfig> {
    if !hub.enabled {
        return Vec::new();
    }
    let (mon_x, mon_y, mon_w, mon_h) = primary_monitor_points();
    let (hx, hy) = hub.position_on_monitor(mon_x, mon_y, mon_w, mon_h);
    let opacity = hub.opacity;
    let mut out = vec![hub_face(hub, hx, hy, opacity)];
    if expanded {
        let sat_size = (hub.size * 0.92).clamp(
            sqyre_persist::MIN_OVERLAY_BUTTON_SIZE,
            sqyre_persist::MAX_OVERLAY_BUTTON_SIZE,
        );
        let radius = hub.size * 2.15;
        for (i, action) in ACTIONS.iter().enumerate() {
            let (sx, sy) = satellite_top_left(SatelliteLayout {
                hub_x: hx,
                hub_y: hy,
                hub_size: hub.size,
                sat_size,
                edge: hub.edge,
                radius,
                index: i,
                count: ACTIONS.len(),
            });
            out.push(satellite_face(action, sx, sy, sat_size, opacity));
        }
    }
    out
}

#[cfg(all(feature = "native-runtime", feature = "overlay-buttons"))]
struct SatelliteLayout {
    hub_x: f32,
    hub_y: f32,
    hub_size: f32,
    sat_size: f32,
    edge: HubEdge,
    radius: f32,
    index: usize,
    count: usize,
}

#[cfg(all(feature = "native-runtime", feature = "overlay-buttons"))]
fn hub_face(hub: &QuickAccessHubSettings, x: f32, y: f32, opacity: u8) -> OverlayButtonConfig {
    OverlayButtonConfig {
        id: QUICK_ACCESS_HUB_ID.into(),
        program: String::new(),
        enabled: true,
        macro_name: QUICK_ACCESS_HUB_ID.into(),
        icon: "sqyre-brand".into(),
        point: String::new(),
        x,
        y,
        size: hub.size,
        corner_radius: DEFAULT_OVERLAY_CORNER_RADIUS,
        border_width: DEFAULT_OVERLAY_BORDER_WIDTH,
        border_color: DEFAULT_OVERLAY_ACCENT_HEX.into(),
        border_alpha: opacity,
        bg_color: "#000000".into(),
        bg_alpha: opacity.saturating_mul(180) / 255,
        icon_color: DEFAULT_OVERLAY_ICON_HEX.into(),
        icon_alpha: opacity,
        icon_hover_color: DEFAULT_OVERLAY_ACCENT_HEX.into(),
        visibility_gate: Default::default(),
    }
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
        corner_radius: DEFAULT_OVERLAY_CORNER_RADIUS,
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

/// Top-left of satellite `i` on an inward half-circle from the hub.
#[cfg(all(feature = "native-runtime", feature = "overlay-buttons"))]
fn satellite_top_left(layout: SatelliteLayout) -> (f32, f32) {
    let cx = layout.hub_x + layout.hub_size * 0.5;
    let cy = layout.hub_y + layout.hub_size * 0.5;
    let (start, sweep) = match layout.edge {
        // Inward = toward screen center.
        HubEdge::Right => (std::f32::consts::FRAC_PI_2, std::f32::consts::PI), // up → left → down
        HubEdge::Left => (-std::f32::consts::FRAC_PI_2, std::f32::consts::PI), // down → right → up
        HubEdge::Top => (0.0, std::f32::consts::PI), // right → down → left
        HubEdge::Bottom => (std::f32::consts::PI, std::f32::consts::PI), // left → up → right
    };
    let t = if layout.count <= 1 {
        0.5
    } else {
        layout.index as f32 / (layout.count as f32 - 1.0)
    };
    let angle = start + sweep * t;
    let sx = cx + layout.radius * angle.cos() - layout.sat_size * 0.5;
    let sy = cy + layout.radius * angle.sin() - layout.sat_size * 0.5;
    (sx, sy)
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

    /// Esc collapses an open fan.
    pub(crate) fn poll_quick_access_escape(&mut self, ctx: &egui::Context) {
        if !self.quick_access_expanded {
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
        let Some(action) = QuickAccessAction::from_token(token) else {
            return false;
        };
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
        let hub_x = 1894.0;
        let hub_y = 500.0;
        let hub_size = 26.0;
        let sat = 24.0;
        let layout = |index| SatelliteLayout {
            hub_x,
            hub_y,
            hub_size,
            sat_size: sat,
            edge: HubEdge::Right,
            radius: 56.0,
            index,
            count: 4,
        };
        let (_x0, y0) = satellite_top_left(layout(0));
        let (x1, _y1) = satellite_top_left(layout(1));
        let (_x2, y2) = satellite_top_left(layout(3));
        // Screen Y grows downward: start angle π/2 is below, end 3π/2 is above.
        assert!(y0 > hub_y + hub_size * 0.5);
        assert!(y2 < hub_y + hub_size * 0.5);
        assert!(x1 < hub_x);
    }
}
