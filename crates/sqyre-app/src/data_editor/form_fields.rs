//! Per-tab form buffers grouped off [`super::DataEditor`].

use crate::data_editor::helpers::{overlay_hex_or_empty, rgba_color};
use crate::overlay_icons;
use eframe::egui;
use sqyre_persist::{
    default_overlay_position, OverlayButtonConfig, OverlayVisibilityGate, OverlayVisibilityMode,
    DEFAULT_OVERLAY_BORDER_WIDTH, DEFAULT_OVERLAY_BUTTON_SIZE, DEFAULT_OVERLAY_CORNER_RADIUS,
    DEFAULT_OVERLAY_FALLBACK_SCREEN_H, DEFAULT_OVERLAY_FALLBACK_SCREEN_W,
    DEFAULT_OVERLAY_GATE_BLUR, DEFAULT_OVERLAY_GATE_INTERVAL_MS, DEFAULT_OVERLAY_GATE_TOLERANCE,
};

/// Overlay visibility-gate draft fields.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct OverlayGateFields {
    pub enabled: bool,
    pub targets: Vec<String>,
    pub search_area: String,
    pub tolerance: f64,
    pub blur: i32,
    pub match_method: sqyre_domain::MatchMethod,
    pub interval_ms: u64,
}

impl Default for OverlayGateFields {
    fn default() -> Self {
        Self {
            enabled: false,
            targets: Vec::new(),
            search_area: String::new(),
            tolerance: DEFAULT_OVERLAY_GATE_TOLERANCE,
            blur: DEFAULT_OVERLAY_GATE_BLUR,
            match_method: sqyre_domain::MatchMethod::CcoeffNormed,
            interval_ms: DEFAULT_OVERLAY_GATE_INTERVAL_MS,
        }
    }
}

impl OverlayGateFields {
    pub(crate) fn from_config(gate: &OverlayVisibilityGate) -> Self {
        Self {
            enabled: gate.is_active(),
            targets: gate.targets.clone(),
            search_area: gate.search_area.clone(),
            tolerance: gate.tolerance,
            blur: gate.blur,
            match_method: gate.match_method,
            interval_ms: gate.interval_ms,
        }
    }

    pub(crate) fn to_config(&self) -> OverlayVisibilityGate {
        let mut gate = OverlayVisibilityGate {
            mode: if self.enabled {
                OverlayVisibilityMode::ShowWhenFound
            } else {
                OverlayVisibilityMode::Off
            },
            targets: self.targets.clone(),
            search_area: self.search_area.trim().to_string(),
            tolerance: self.tolerance,
            blur: self.blur,
            match_method: self.match_method,
            interval_ms: self.interval_ms,
        };
        gate.clamp();
        gate
    }

    pub(crate) fn is_dirty_vs(&self, gate: &OverlayVisibilityGate) -> bool {
        self.enabled != gate.is_active()
            || self.targets != gate.targets
            || self.search_area.trim() != gate.search_area.trim()
            || sqyre_domain::f64_changed(self.tolerance, gate.tolerance)
            || self.blur != gate.blur
            || self.match_method != gate.match_method
            || self.interval_ms != gate.interval_ms
    }
}

/// Overlay button form draft (style + placement + visibility gate).
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct OverlayFields {
    pub macro_name: String,
    pub enabled: bool,
    pub icon: String,
    pub point: String,
    pub x: f32,
    pub y: f32,
    pub size: f32,
    pub corner_radius: f32,
    pub border_width: f32,
    pub border: egui::Color32,
    pub bg: egui::Color32,
    pub icon_color: egui::Color32,
    pub icon_hover: egui::Color32,
    pub gate: OverlayGateFields,
}

impl Default for OverlayFields {
    fn default() -> Self {
        let (x, y) = default_overlay_position(
            0.0,
            0.0,
            DEFAULT_OVERLAY_FALLBACK_SCREEN_W,
            DEFAULT_OVERLAY_FALLBACK_SCREEN_H,
            DEFAULT_OVERLAY_BUTTON_SIZE,
            0,
        );
        Self {
            macro_name: String::new(),
            enabled: true,
            icon: overlay_icons::DEFAULT_ICON_ID.into(),
            point: String::new(),
            x,
            y,
            size: DEFAULT_OVERLAY_BUTTON_SIZE,
            corner_radius: DEFAULT_OVERLAY_CORNER_RADIUS,
            border_width: DEFAULT_OVERLAY_BORDER_WIDTH,
            border: rgba_color([0xdc, 0x9d, 0x2e, 255]),
            bg: rgba_color([0, 0, 0, 0]),
            icon_color: rgba_color([0xf5, 0xe6, 0xc0, 255]),
            icon_hover: rgba_color([0xdc, 0x9d, 0x2e, 255]),
            gate: OverlayGateFields::default(),
        }
    }
}

impl OverlayFields {
    pub(crate) fn apply_style_to_config(&self, btn: &mut OverlayButtonConfig) {
        btn.corner_radius = self.corner_radius;
        btn.border_width = self.border_width;
        btn.border_color =
            overlay_hex_or_empty(self.border, sqyre_persist::DEFAULT_OVERLAY_ACCENT_HEX);
        btn.border_alpha = self.border.a();
        btn.bg_color = if self.bg.a() == 0 {
            String::new()
        } else {
            overlay_hex_or_empty(self.bg, "#000000")
        };
        btn.bg_alpha = self.bg.a();
        btn.icon_color =
            overlay_hex_or_empty(self.icon_color, sqyre_persist::DEFAULT_OVERLAY_ICON_HEX);
        btn.icon_alpha = self.icon_color.a();
        btn.icon_hover_color =
            overlay_hex_or_empty(self.icon_hover, sqyre_persist::DEFAULT_OVERLAY_ACCENT_HEX);
    }

    pub(crate) fn load_style_from_config(&mut self, btn: &OverlayButtonConfig) {
        self.corner_radius = btn.corner_radius;
        self.border_width = btn.border_width;
        self.border = rgba_color(btn.border_rgba());
        self.bg = rgba_color(btn.bg_rgba());
        self.icon_color = rgba_color(btn.icon_rgba());
        self.icon_hover = rgba_color(btn.icon_hover_rgba());
    }

    pub(crate) fn reset_style(&mut self) {
        let defaults = OverlayButtonConfig::new("", "");
        self.load_style_from_config(&defaults);
    }

    pub(crate) fn style_is_dirty_vs(&self, btn: &OverlayButtonConfig) -> bool {
        sqyre_domain::f32_changed(self.corner_radius, btn.corner_radius)
            || sqyre_domain::f32_changed(self.border_width, btn.border_width)
            || self.border != rgba_color(btn.border_rgba())
            || self.bg != rgba_color(btn.bg_rgba())
            || self.icon_color != rgba_color(btn.icon_rgba())
            || self.icon_hover != rgba_color(btn.icon_hover_rgba())
    }

    pub(crate) fn placement_is_dirty_vs(&self, btn: &OverlayButtonConfig) -> bool {
        self.macro_name.trim() != btn.macro_name.trim()
            || self.enabled != btn.enabled
            || self.icon != btn.icon
            || self.point.trim() != btn.point.trim()
            || sqyre_domain::f32_changed(self.x, btn.x)
            || sqyre_domain::f32_changed(self.y, btn.y)
            || sqyre_domain::f32_changed(self.size, btn.size)
    }
}

/// Mask geometry draft fields.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct MaskFields {
    pub shape: String,
    pub center_x: String,
    pub center_y: String,
    pub base: String,
    pub height: String,
    pub radius: String,
    pub inverse: bool,
}

impl Default for MaskFields {
    fn default() -> Self {
        Self {
            shape: "rectangle".into(),
            center_x: "50".into(),
            center_y: "50".into(),
            base: String::new(),
            height: String::new(),
            radius: String::new(),
            inverse: false,
        }
    }
}

/// Shared left/top/right/bottom draft (Search Areas, Screen Cap, Match Check).
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct RectFields {
    pub left: String,
    pub top: String,
    pub right: String,
    pub bottom: String,
}

impl RectFields {
    pub(crate) fn load_displays(
        &mut self,
        left: impl Into<String>,
        top: impl Into<String>,
        right: impl Into<String>,
        bottom: impl Into<String>,
    ) {
        self.left = left.into();
        self.top = top.into();
        self.right = right.into();
        self.bottom = bottom.into();
    }
}

/// Program binding draft fields.
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct ProgramFields {
    pub process_path: String,
    pub window_title: String,
}

/// Item-specific draft fields (mask name + stack max).
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct ItemFields {
    pub mask: String,
    pub stack_max: String,
}

impl Default for ItemFields {
    fn default() -> Self {
        Self {
            mask: String::new(),
            stack_max: "0".into(),
        }
    }
}

/// Atlas member-list draft fields.
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct AtlasFields {
    pub members: Vec<String>,
    pub add: String,
}

/// Point coordinate draft fields.
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct PointFields {
    pub x: String,
    pub y: String,
}

/// Editable form buffers for the currently selected entity.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct FormDrafts {
    pub name: String,
    /// 1-based monitor slot for Points / Search Areas (relative coords in point/rect).
    pub monitor: u32,
    pub point: PointFields,
    pub rect: RectFields,
    pub cols: String,
    pub rows: String,
    pub item: ItemFields,
    pub tags: Vec<String>,
    pub tag_draft: String,
    pub mask: MaskFields,
    pub search_area: String,
    pub atlas: AtlasFields,
    /// Overlay button form draft (placement, style, visibility gate).
    pub overlay: OverlayFields,
    pub program: ProgramFields,
}

impl Default for FormDrafts {
    fn default() -> Self {
        Self {
            name: String::new(),
            monitor: 1,
            point: PointFields::default(),
            rect: RectFields::default(),
            cols: "1".into(),
            rows: "1".into(),
            item: ItemFields::default(),
            tags: Vec::new(),
            tag_draft: String::new(),
            mask: MaskFields::default(),
            search_area: String::new(),
            atlas: AtlasFields::default(),
            overlay: OverlayFields::default(),
            program: ProgramFields::default(),
        }
    }
}

#[cfg(test)]
mod drafts_tests {
    use super::*;

    #[test]
    fn default_drafts_match_prior_editor_defaults() {
        let d = FormDrafts::default();
        assert!(d.name.is_empty());
        assert_eq!(d.monitor, 1);
        assert_eq!(d.cols, "1");
        assert_eq!(d.rows, "1");
        assert_eq!(d.item.stack_max, "0");
        assert!(d.tags.is_empty());
        assert!(d.tag_draft.is_empty());
        assert!(d.search_area.is_empty());
    }
}
