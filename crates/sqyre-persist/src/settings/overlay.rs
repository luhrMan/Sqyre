//! Desktop overlay button preferences, hub helpers, and defaults.

use serde::{Deserialize, Serialize};
use sqyre_domain::{clamp_match_blur, clamp_match_tolerance, MatchMethod};

use super::UserSettings;

pub const DEFAULT_OVERLAY_PRESS_STOPS_RUN: bool = true;
/// Image Search, Find Pixel, and OCR captures include overlay buttons.
pub const DEFAULT_OVERLAY_BUTTONS_IN_DETECTION: bool = true;
/// Overlay buttons stay hidden while the Sqyre window is in front.
pub const DEFAULT_OVERLAY_BUTTONS_OVER_SQYRE: bool = false;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OverlayVisibilityMode {
    /// No image gate — visibility follows enabled + focus only.
    #[default]
    Off,
    /// Show the button only while a template match is found in the search area.
    ShowWhenFound,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HubEdge {
    Left,
    #[default]
    Right,
    Top,
    Bottom,
}

impl HubEdge {
    /// All selectable values.
    pub const ALL: [Self; 4] = [Self::Left, Self::Right, Self::Top, Self::Bottom];

    /// Short label for settings / diagnostics.
    pub fn label(self) -> &'static str {
        match self {
            Self::Left => "Left",
            Self::Right => "Right",
            Self::Top => "Top",
            Self::Bottom => "Bottom",
        }
    }
}

/// Reserved overlay id and fire token for the quick-access hub button.
pub const QUICK_ACCESS_HUB_ID: &str = "__sqyre_hub";
/// Fire token that only collapses an open hub fan (outside click / dismiss).
pub const QUICK_ACCESS_HUB_DISMISS: &str = "__sqyre_hub_dismiss";
/// List / tooltip label for the seeded hub overlay button.
pub const QUICK_ACCESS_HUB_LABEL: &str = "Quick Access";
/// Default hub face / brand opacity (0–255).
pub const DEFAULT_QUICK_ACCESS_OPACITY: u8 = 220;
/// Brand icon id for the hub face.
pub const QUICK_ACCESS_HUB_ICON: &str = "sqyre-brand";
/// Max fraction of the hub face that may sit outside the monitor when docked.
pub const HUB_MAX_EDGE_OVERHANG: f32 = 0.50;

/// True when `id` is the reserved quick-access hub overlay button.
pub fn is_quick_access_hub_id(id: &str) -> bool {
    id == QUICK_ACCESS_HUB_ID
}

/// Top-left desktop position for a hub face docked flush to `edge` at `offset`.
///
/// `size_px` and the monitor rect share units (physical desktop pixels when
/// the rect comes from capture monitor metrics). Prefer
/// [`snap_hub_xy_to_monitor`] when preserving drag overhang.
pub fn hub_position_on_monitor(
    edge: HubEdge,
    offset: f32,
    mon_x: f32,
    mon_y: f32,
    mon_w: f32,
    mon_h: f32,
    size_px: f32,
) -> (f32, f32) {
    let size = hub_layout_size_px(size_px, mon_w, mon_h);
    let o = if offset.is_finite() {
        offset.clamp(0.0, 1.0)
    } else {
        0.5
    };
    let max_x = (mon_w - size).max(0.0);
    let max_y = (mon_h - size).max(0.0);
    match edge {
        HubEdge::Left => (mon_x, mon_y + o * max_y),
        HubEdge::Right => (mon_x + max_x, mon_y + o * max_y),
        HubEdge::Top => (mon_x + o * max_x, mon_y),
        HubEdge::Bottom => (mon_x + o * max_x, mon_y + max_y),
    }
}

/// On-screen hub edge length in the monitor's coordinate space.
fn hub_layout_size_px(size_px: f32, mon_w: f32, mon_h: f32) -> f32 {
    let fallback = DEFAULT_OVERLAY_BUTTON_SIZE;
    let size = if size_px.is_finite() && size_px > 0.0 {
        size_px
    } else {
        fallback
    };
    size.min(mon_w.max(1.0)).min(mon_h.max(1.0)).max(1.0)
}

/// Snap a dragged hub top-left to the nearest edge of `monitor`; returns edge + offset.
///
/// Offset is along the flush on-screen span (ignores overhang). Prefer
/// [`snap_hub_xy_to_monitor`] when writing desktop coordinates.
///
/// `size` is the on-screen button edge in the same units as the monitor rect
/// (physical pixels when relocating a native overlay face).
pub fn snap_hub_to_monitor(
    x: f32,
    y: f32,
    size: f32,
    mon_x: f32,
    mon_y: f32,
    mon_w: f32,
    mon_h: f32,
) -> (HubEdge, f32) {
    let size = hub_layout_size_px(size, mon_w, mon_h);
    let cx = x + size * 0.5;
    let cy = y + size * 0.5;
    let dist_left = (cx - mon_x).abs();
    let dist_right = (cx - (mon_x + mon_w)).abs();
    let dist_top = (cy - mon_y).abs();
    let dist_bottom = (cy - (mon_y + mon_h)).abs();
    let max_x = (mon_w - size).max(0.0);
    let max_y = (mon_h - size).max(0.0);

    let edge = if dist_left <= dist_right && dist_left <= dist_top && dist_left <= dist_bottom {
        HubEdge::Left
    } else if dist_right <= dist_top && dist_right <= dist_bottom {
        HubEdge::Right
    } else if dist_top <= dist_bottom {
        HubEdge::Top
    } else {
        HubEdge::Bottom
    };

    let offset = match edge {
        HubEdge::Left | HubEdge::Right => {
            if max_y <= f32::EPSILON {
                0.5
            } else {
                ((y - mon_y) / max_y).clamp(0.0, 1.0)
            }
        }
        HubEdge::Top | HubEdge::Bottom => {
            if max_x <= f32::EPSILON {
                0.5
            } else {
                ((x - mon_x) / max_x).clamp(0.0, 1.0)
            }
        }
    };
    (edge, offset)
}

/// Snap hub top-left to the nearest monitor edge, keeping up to
/// [`HUB_MAX_EDGE_OVERHANG`] of the face outside the monitor.
///
/// Along the docked edge the face may hang outward from flush to 50% off-screen
/// (50% remains visible). Along the edge, the same overhang is allowed at the
/// corners so the icon can sit partially past the monitor bounds.
pub fn snap_hub_xy_to_monitor(
    x: f32,
    y: f32,
    size: f32,
    mon_x: f32,
    mon_y: f32,
    mon_w: f32,
    mon_h: f32,
) -> (HubEdge, f32, f32) {
    let size = hub_layout_size_px(size, mon_w, mon_h);
    let (edge, _) = snap_hub_to_monitor(x, y, size, mon_x, mon_y, mon_w, mon_h);
    let max_out = size * HUB_MAX_EDGE_OVERHANG;
    let min_in = size * (1.0 - HUB_MAX_EDGE_OVERHANG);
    // Along-edge span: flush on-screen through 50% past either end.
    let y_lo = mon_y - max_out;
    let y_hi = mon_y + mon_h - min_in;
    let x_lo = mon_x - max_out;
    let x_hi = mon_x + mon_w - min_in;
    let (nx, ny) = match edge {
        HubEdge::Left => {
            // Flush at mon_x; hang left down to mon_x - max_out.
            (x.clamp(mon_x - max_out, mon_x), y.clamp(y_lo, y_hi))
        }
        HubEdge::Right => {
            let flush = mon_x + mon_w - size;
            // Flush at right; hang right up to flush + max_out.
            (x.clamp(flush, flush + max_out), y.clamp(y_lo, y_hi))
        }
        HubEdge::Top => (x.clamp(x_lo, x_hi), y.clamp(mon_y - max_out, mon_y)),
        HubEdge::Bottom => {
            let flush = mon_y + mon_h - size;
            (x.clamp(x_lo, x_hi), y.clamp(flush, flush + max_out))
        }
    };
    (edge, nx, ny)
}

/// Write edge-docked `x`/`y` onto `btn` after snapping its current top-left.
///
/// `size_px` must match the units used for `btn.x`/`btn.y` and the monitor rect
/// (points when the Data Editor relocates; physical pixels for native faces).
pub fn snap_hub_button_to_monitor(
    btn: &mut OverlayButtonConfig,
    mon_x: f32,
    mon_y: f32,
    mon_w: f32,
    mon_h: f32,
    size_px: f32,
) -> HubEdge {
    let (edge, x, y) = snap_hub_xy_to_monitor(btn.x, btn.y, size_px, mon_x, mon_y, mon_w, mon_h);
    btn.point.clear();
    btn.x = x;
    btn.y = y;
    edge
}

/// Seeded General overlay button for the edge-docked quick-access hub.
pub fn default_quick_access_hub_button() -> OverlayButtonConfig {
    let size = DEFAULT_OVERLAY_BUTTON_SIZE;
    let (x, y) = hub_position_on_monitor(
        HubEdge::Right,
        0.5,
        0.0,
        0.0,
        DEFAULT_OVERLAY_FALLBACK_SCREEN_W,
        DEFAULT_OVERLAY_FALLBACK_SCREEN_H,
        size,
    );
    OverlayButtonConfig {
        id: QUICK_ACCESS_HUB_ID.into(),
        program: sqyre_domain::GENERAL_PROGRAM.into(),
        enabled: true,
        macro_name: QUICK_ACCESS_HUB_ID.into(),
        icon: QUICK_ACCESS_HUB_ICON.into(),
        point: String::new(),
        x,
        y,
        size,
        // Hit disk only — face paints the brand logo with no chrome.
        corner_radius: (size * 0.5).clamp(MIN_OVERLAY_CORNER_RADIUS, MAX_OVERLAY_CORNER_RADIUS),
        border_width: 0.0,
        border_color: String::new(),
        border_alpha: 0,
        bg_color: String::new(),
        bg_alpha: 0,
        icon_color: DEFAULT_OVERLAY_ICON_HEX.into(),
        icon_alpha: DEFAULT_QUICK_ACCESS_OPACITY,
        icon_hover_color: DEFAULT_OVERLAY_ACCENT_HEX.into(),
        visibility_gate: OverlayVisibilityGate::default(),
    }
}

/// Insert the hub button when missing; repair locked identity/chrome fields when present.
///
/// Returns `true` when the settings list changed.
pub fn ensure_quick_access_hub_button(settings: &mut UserSettings) -> bool {
    if let Some(btn) = settings
        .overlay
        .overlay_buttons
        .iter_mut()
        .find(|b| is_quick_access_hub_id(&b.id))
    {
        let mut dirty = false;
        if btn.program != sqyre_domain::GENERAL_PROGRAM {
            btn.program = sqyre_domain::GENERAL_PROGRAM.into();
            dirty = true;
        }
        if btn.macro_name != QUICK_ACCESS_HUB_ID {
            btn.macro_name = QUICK_ACCESS_HUB_ID.into();
            dirty = true;
        }
        if btn.icon != QUICK_ACCESS_HUB_ICON {
            btn.icon = QUICK_ACCESS_HUB_ICON.into();
            dirty = true;
        }
        if !btn.point.is_empty() {
            btn.point.clear();
            dirty = true;
        }
        if btn.border_width != 0.0 {
            btn.border_width = 0.0;
            dirty = true;
        }
        if btn.border_alpha != 0 {
            btn.border_alpha = 0;
            dirty = true;
        }
        if btn.bg_alpha != 0 {
            btn.bg_alpha = 0;
            dirty = true;
        }
        return dirty;
    }
    settings
        .overlay
        .overlay_buttons
        .insert(0, default_quick_access_hub_button());
    true
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct OverlayVisibilityGate {
    #[serde(default)]
    pub mode: OverlayVisibilityMode,
    /// Catalog item refs (`program~item`), same as Image Search targets.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub targets: Vec<String>,
    /// Catalog search area (`program~area`); resolved with live monitor slots.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub search_area: String,
    #[serde(default = "default_overlay_gate_tolerance")]
    pub tolerance: f64,
    #[serde(default = "default_overlay_gate_blur")]
    pub blur: i32,
    #[serde(default)]
    pub match_method: MatchMethod,
    /// Milliseconds between capture+match polls.
    #[serde(default = "default_overlay_gate_interval_ms")]
    pub interval_ms: u64,
}

impl Default for OverlayVisibilityGate {
    fn default() -> Self {
        Self {
            mode: OverlayVisibilityMode::Off,
            targets: Vec::new(),
            search_area: String::new(),
            tolerance: DEFAULT_OVERLAY_GATE_TOLERANCE,
            blur: DEFAULT_OVERLAY_GATE_BLUR,
            match_method: MatchMethod::CcoeffNormed,
            interval_ms: DEFAULT_OVERLAY_GATE_INTERVAL_MS,
        }
    }
}

impl OverlayVisibilityGate {
    pub fn is_active(&self) -> bool {
        self.mode == OverlayVisibilityMode::ShowWhenFound
    }

    pub fn is_off_default(&self) -> bool {
        *self == Self::default()
    }

    pub fn clamp(&mut self) {
        self.blur = clamp_match_blur(self.blur);
        self.tolerance = clamp_match_tolerance(self.tolerance, self.match_method);
        if self.interval_ms == 0 {
            self.interval_ms = DEFAULT_OVERLAY_GATE_INTERVAL_MS;
        }
        self.interval_ms = self
            .interval_ms
            .clamp(MIN_OVERLAY_GATE_INTERVAL_MS, MAX_OVERLAY_GATE_INTERVAL_MS);
    }
}

/// Always-on-top screen button that starts a named macro.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct OverlayButtonConfig {
    /// Stable id used for the deferred viewport hash.
    pub id: String,
    /// Catalog program this button belongs to (shown when that program is focused).
    /// [`sqyre_domain::GENERAL_PROGRAM`] buttons ignore which other app is focused.
    /// Every button hides while Sqyre is in front unless
    /// [`OverlaySettings::overlay_buttons_over_sqyre`] is on.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub program: String,
    /// When false the button is not drawn (except live Data Editor preview).
    #[serde(default = "default_overlay_button_enabled")]
    pub enabled: bool,
    /// Macro name to start (must match an entry in `db.yaml`).
    /// Also used as the button's display name / tooltip.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub macro_name: String,
    /// Icon catalog id (Phosphor kebab-case, e.g. `play`, `lightning`). Empty = default play.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub icon: String,
    /// Optional catalog point (`program~name`). When set, desktop position is resolved from
    /// that point each frame; [`Self::x`] / [`Self::y`] are fallbacks if resolve fails.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub point: String,
    /// Desktop position of the button viewport (top-left, points).
    /// Used when [`Self::point`] is empty, or as fallback when the point cannot be resolved.
    #[serde(default)]
    pub x: f32,
    #[serde(default)]
    pub y: f32,
    /// Button glyph/image size in points (viewport is slightly larger for padding).
    #[serde(default = "default_overlay_button_size")]
    pub size: f32,
    /// Corner rounding in points (0 = square).
    #[serde(default = "default_overlay_corner_radius")]
    pub corner_radius: f32,
    /// Border stroke width in points (0 = no border).
    #[serde(default = "default_overlay_border_width")]
    pub border_width: f32,
    /// Border color as `#rrggbb` (empty = Sqyre gold).
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub border_color: String,
    /// Border opacity 0–255.
    #[serde(default = "default_overlay_full_alpha")]
    pub border_alpha: u8,
    /// Background fill color as `#rrggbb` (empty = black when alpha > 0).
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub bg_color: String,
    /// Background opacity 0–255 (0 = no fill).
    #[serde(default)]
    pub bg_alpha: u8,
    /// Icon/glyph color as `#rrggbb` (empty = cream).
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub icon_color: String,
    /// Icon opacity 0–255.
    #[serde(default = "default_overlay_full_alpha")]
    pub icon_alpha: u8,
    /// Hover icon color as `#rrggbb` (empty = Sqyre gold).
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub icon_hover_color: String,
    /// Optional image-search visibility gate (show when template found).
    #[serde(default, skip_serializing_if = "OverlayVisibilityGate::is_off_default")]
    pub visibility_gate: OverlayVisibilityGate,
}

pub const DEFAULT_OVERLAY_BUTTON_SIZE: f32 = 26.0;
pub const MIN_OVERLAY_BUTTON_SIZE: f32 = 12.0;
pub const MAX_OVERLAY_BUTTON_SIZE: f32 = 128.0;
pub const DEFAULT_OVERLAY_CORNER_RADIUS: f32 = 8.0;
pub const MIN_OVERLAY_CORNER_RADIUS: f32 = 0.0;
pub const MAX_OVERLAY_CORNER_RADIUS: f32 = 64.0;
pub const DEFAULT_OVERLAY_BORDER_WIDTH: f32 = 1.5;
pub const MIN_OVERLAY_BORDER_WIDTH: f32 = 0.0;
pub const MAX_OVERLAY_BORDER_WIDTH: f32 = 8.0;
/// Fallback primary-monitor size when live display metrics are unavailable.
pub const DEFAULT_OVERLAY_FALLBACK_SCREEN_W: f32 = 1920.0;
pub const DEFAULT_OVERLAY_FALLBACK_SCREEN_H: f32 = 1080.0;
/// Default border / hover icon when `border_color` / `icon_hover_color` is empty (`#dc9d2e`).
pub const DEFAULT_OVERLAY_ACCENT_HEX: &str = "#dc9d2e";
/// Default idle icon when `icon_color` is empty (`#f5e6c0`).
pub const DEFAULT_OVERLAY_ICON_HEX: &str = "#f5e6c0";

/// Default Image Search tolerance for overlay visibility gates.
pub const DEFAULT_OVERLAY_GATE_TOLERANCE: f64 = 0.95;
/// Default Image Search blur for overlay visibility gates.
pub const DEFAULT_OVERLAY_GATE_BLUR: i32 = 5;
/// Default poll interval for overlay visibility gates.
pub const DEFAULT_OVERLAY_GATE_INTERVAL_MS: u64 = 250;
pub const MIN_OVERLAY_GATE_INTERVAL_MS: u64 = 100;
pub const MAX_OVERLAY_GATE_INTERVAL_MS: u64 = 60_000;

fn default_overlay_button_size() -> f32 {
    DEFAULT_OVERLAY_BUTTON_SIZE
}

fn default_overlay_gate_tolerance() -> f64 {
    DEFAULT_OVERLAY_GATE_TOLERANCE
}

fn default_overlay_gate_blur() -> i32 {
    DEFAULT_OVERLAY_GATE_BLUR
}

fn default_overlay_gate_interval_ms() -> u64 {
    DEFAULT_OVERLAY_GATE_INTERVAL_MS
}

/// Top-left for an overlay button centered on a desktop rect.
///
/// `index` offsets subsequent buttons horizontally so new ones do not stack.
pub fn default_overlay_position(
    screen_x: f32,
    screen_y: f32,
    screen_w: f32,
    screen_h: f32,
    size: f32,
    index: usize,
) -> (f32, f32) {
    let gap = size + 8.0;
    let x = screen_x + (screen_w - size) * 0.5 + (index as f32) * gap;
    let y = screen_y + (screen_h - size) * 0.5;
    (x.max(screen_x), y.max(screen_y))
}

fn default_overlay_button_enabled() -> bool {
    true
}

fn default_overlay_corner_radius() -> f32 {
    DEFAULT_OVERLAY_CORNER_RADIUS
}

fn default_overlay_border_width() -> f32 {
    DEFAULT_OVERLAY_BORDER_WIDTH
}

fn default_overlay_full_alpha() -> u8 {
    255
}

impl OverlayButtonConfig {
    pub fn new(id: impl Into<String>, program: impl Into<String>) -> Self {
        let (x, y) = default_overlay_position(
            0.0,
            0.0,
            DEFAULT_OVERLAY_FALLBACK_SCREEN_W,
            DEFAULT_OVERLAY_FALLBACK_SCREEN_H,
            DEFAULT_OVERLAY_BUTTON_SIZE,
            0,
        );
        Self {
            id: id.into(),
            program: program.into(),
            enabled: true,
            macro_name: String::new(),
            icon: String::new(),
            point: String::new(),
            x,
            y,
            size: DEFAULT_OVERLAY_BUTTON_SIZE,
            corner_radius: DEFAULT_OVERLAY_CORNER_RADIUS,
            border_width: DEFAULT_OVERLAY_BORDER_WIDTH,
            border_color: String::new(),
            border_alpha: 255,
            bg_color: String::new(),
            bg_alpha: 0,
            icon_color: String::new(),
            icon_alpha: 255,
            icon_hover_color: String::new(),
            visibility_gate: OverlayVisibilityGate::default(),
        }
    }

    /// Display name for lists (macro name, else id).
    pub fn display_name(&self) -> &str {
        if is_quick_access_hub_id(&self.id) {
            return QUICK_ACCESS_HUB_LABEL;
        }
        let macro_name = self.macro_name.trim();
        if !macro_name.is_empty() {
            return macro_name;
        }
        self.id.as_str()
    }

    /// Resolve `#rrggbb` (or empty → `fallback`) plus alpha → RGBA.
    pub fn resolve_rgba(hex: &str, alpha: u8, fallback: &str) -> [u8; 4] {
        let src = if hex.trim().is_empty() {
            fallback
        } else {
            hex.trim()
        };
        let rgb = sqyre_domain::parse_hex_color(src).unwrap_or_else(|| {
            sqyre_domain::parse_hex_color(fallback).unwrap_or([0xdc, 0x9d, 0x2e, 255])
        });
        [rgb[0], rgb[1], rgb[2], alpha]
    }

    pub fn border_rgba(&self) -> [u8; 4] {
        Self::resolve_rgba(
            &self.border_color,
            self.border_alpha,
            DEFAULT_OVERLAY_ACCENT_HEX,
        )
    }

    pub fn bg_rgba(&self) -> [u8; 4] {
        Self::resolve_rgba(&self.bg_color, self.bg_alpha, "#000000")
    }

    pub fn icon_rgba(&self) -> [u8; 4] {
        Self::resolve_rgba(&self.icon_color, self.icon_alpha, DEFAULT_OVERLAY_ICON_HEX)
    }

    pub fn icon_hover_rgba(&self) -> [u8; 4] {
        Self::resolve_rgba(
            &self.icon_hover_color,
            self.icon_alpha,
            DEFAULT_OVERLAY_ACCENT_HEX,
        )
    }

    /// Reset appearance fields to built-in defaults (keeps size/position/binding).
    pub fn reset_appearance(&mut self) {
        self.corner_radius = DEFAULT_OVERLAY_CORNER_RADIUS;
        self.border_width = DEFAULT_OVERLAY_BORDER_WIDTH;
        self.border_color.clear();
        self.border_alpha = 255;
        self.bg_color.clear();
        self.bg_alpha = 0;
        self.icon_color.clear();
        self.icon_alpha = 255;
        self.icon_hover_color.clear();
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct OverlaySettings {
    /// Pressing an overlay button while a macro is running requests stop.
    #[serde(default = "default_overlay_press_stops_run")]
    pub overlay_press_stops_run: bool,

    /// When true, Image Search, Find Pixel, and OCR captures include overlay buttons.
    #[serde(default = "default_overlay_buttons_in_detection")]
    pub overlay_buttons_in_detection: bool,

    /// When true, overlay buttons stay on screen while the Sqyre window is in front.
    /// When false, they hide until another app is focused. Placing a button in the
    /// Overlay editor still shows that button.
    #[serde(default = "default_overlay_buttons_over_sqyre")]
    pub overlay_buttons_over_sqyre: bool,

    /// User-configured overlay buttons (per-program; enabled buttons for
    /// [`sqyre_domain::GENERAL_PROGRAM`] ignore other apps, others show when that
    /// program is focused). All of them hide while Sqyre is in front unless
    /// [`OverlaySettings::overlay_buttons_over_sqyre`] is on.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub overlay_buttons: Vec<OverlayButtonConfig>,

    /// Size (pt) seeded into newly created overlay buttons.
    #[serde(default = "default_overlay_button_size_v")]
    pub overlay_default_size: f32,

    /// Corner radius (pt) seeded into newly created overlay buttons.
    #[serde(default = "default_overlay_corner_radius_v")]
    pub overlay_default_corner_radius: f32,

    /// Border width (pt) seeded into newly created overlay buttons.
    #[serde(default = "default_overlay_border_width_v")]
    pub overlay_default_border_width: f32,

    /// Border / hover color seeded into new overlay buttons (`#rrggbb`, empty = gold).
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub overlay_default_accent_color: String,

    /// Icon color seeded into new overlay buttons (`#rrggbb`, empty = cream).
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub overlay_default_icon_color: String,

    /// Poll interval (ms) seeded into new overlay visibility gates.
    #[serde(default = "default_overlay_gate_interval_v")]
    pub overlay_gate_default_interval_ms: u64,
}

fn default_overlay_press_stops_run() -> bool {
    DEFAULT_OVERLAY_PRESS_STOPS_RUN
}
fn default_overlay_buttons_in_detection() -> bool {
    DEFAULT_OVERLAY_BUTTONS_IN_DETECTION
}
fn default_overlay_buttons_over_sqyre() -> bool {
    DEFAULT_OVERLAY_BUTTONS_OVER_SQYRE
}
fn default_overlay_button_size_v() -> f32 {
    DEFAULT_OVERLAY_BUTTON_SIZE
}
fn default_overlay_corner_radius_v() -> f32 {
    DEFAULT_OVERLAY_CORNER_RADIUS
}
fn default_overlay_border_width_v() -> f32 {
    DEFAULT_OVERLAY_BORDER_WIDTH
}
fn default_overlay_gate_interval_v() -> u64 {
    DEFAULT_OVERLAY_GATE_INTERVAL_MS
}

impl Default for OverlaySettings {
    fn default() -> Self {
        Self {
            overlay_press_stops_run: DEFAULT_OVERLAY_PRESS_STOPS_RUN,
            overlay_buttons_in_detection: DEFAULT_OVERLAY_BUTTONS_IN_DETECTION,
            overlay_buttons_over_sqyre: DEFAULT_OVERLAY_BUTTONS_OVER_SQYRE,
            overlay_buttons: Vec::new(),
            overlay_default_size: DEFAULT_OVERLAY_BUTTON_SIZE,
            overlay_default_corner_radius: DEFAULT_OVERLAY_CORNER_RADIUS,
            overlay_default_border_width: DEFAULT_OVERLAY_BORDER_WIDTH,
            overlay_default_accent_color: String::new(),
            overlay_default_icon_color: String::new(),
            overlay_gate_default_interval_ms: DEFAULT_OVERLAY_GATE_INTERVAL_MS,
        }
    }
}

impl OverlaySettings {
    /// Clamp numeric ranges to settings UI bounds.
    pub fn clamp(&mut self) {
        for btn in &mut self.overlay_buttons {
            if btn.size <= 0.0 {
                btn.size = DEFAULT_OVERLAY_BUTTON_SIZE;
            }
            btn.size = btn
                .size
                .clamp(MIN_OVERLAY_BUTTON_SIZE, MAX_OVERLAY_BUTTON_SIZE);
            if btn.corner_radius < 0.0 {
                btn.corner_radius = DEFAULT_OVERLAY_CORNER_RADIUS;
            }
            btn.corner_radius = btn
                .corner_radius
                .clamp(MIN_OVERLAY_CORNER_RADIUS, MAX_OVERLAY_CORNER_RADIUS);
            if btn.border_width < 0.0 {
                btn.border_width = DEFAULT_OVERLAY_BORDER_WIDTH;
            }
            btn.border_width = btn
                .border_width
                .clamp(MIN_OVERLAY_BORDER_WIDTH, MAX_OVERLAY_BORDER_WIDTH);
            btn.visibility_gate.clamp();
        }
        if self.overlay_default_size <= 0.0 {
            self.overlay_default_size = DEFAULT_OVERLAY_BUTTON_SIZE;
        }
        self.overlay_default_size = self
            .overlay_default_size
            .clamp(MIN_OVERLAY_BUTTON_SIZE, MAX_OVERLAY_BUTTON_SIZE);
        if self.overlay_default_corner_radius < 0.0 {
            self.overlay_default_corner_radius = DEFAULT_OVERLAY_CORNER_RADIUS;
        }
        self.overlay_default_corner_radius = self
            .overlay_default_corner_radius
            .clamp(MIN_OVERLAY_CORNER_RADIUS, MAX_OVERLAY_CORNER_RADIUS);
        if self.overlay_default_border_width < 0.0 {
            self.overlay_default_border_width = DEFAULT_OVERLAY_BORDER_WIDTH;
        }
        self.overlay_default_border_width = self
            .overlay_default_border_width
            .clamp(MIN_OVERLAY_BORDER_WIDTH, MAX_OVERLAY_BORDER_WIDTH);
        self.overlay_gate_default_interval_ms = self
            .overlay_gate_default_interval_ms
            .clamp(MIN_OVERLAY_GATE_INTERVAL_MS, MAX_OVERLAY_GATE_INTERVAL_MS);
        if !self.overlay_default_accent_color.trim().is_empty()
            && sqyre_domain::parse_hex_color(self.overlay_default_accent_color.trim()).is_none()
        {
            self.overlay_default_accent_color.clear();
        }
        if !self.overlay_default_icon_color.trim().is_empty()
            && sqyre_domain::parse_hex_color(self.overlay_default_icon_color.trim()).is_none()
        {
            self.overlay_default_icon_color.clear();
        }
    }
}
