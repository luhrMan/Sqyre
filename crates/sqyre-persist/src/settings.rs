//! User settings persisted inside the Sqyre data directory.
//!
//! Lives at `~/.sqyre/settings.yaml` (or under a relocated data dir). A small
//! pointer file at `~/.config/sqyre/data_dir` records a non-default data location
//! so the next launch can find settings after a relocate.

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

use crate::{PersistError, Result};
use sqyre_domain::{
    clamp_match_blur, clamp_match_tolerance, rename_coordinate_entity, rename_coordinate_program,
    CatalogItemSort, CoordinateRef, Macro, MatchMethod, ACTION_COLOR_KEY_CONTROL_FLOW,
    ACTION_COLOR_KEY_DEFAULT, ACTION_COLOR_KEY_DETECTION, ACTION_COLOR_KEY_MISCELLANEOUS,
    ACTION_COLOR_KEY_MOUSE_KEYBOARD, ACTION_COLOR_KEY_VARIABLES, ACTION_COLOR_KEY_WAIT,
};

pub const DEFAULT_IMAGE_SEARCH_CLOSE_MATCHES_DISTANCE: i32 = 10;
/// Skip later icon variants once one hits on the same search/placement.
pub const DEFAULT_IMAGE_SEARCH_VARIANT_EXIT_EARLY: bool = true;
pub const DEFAULT_HIDE_APP_DURING_RECORDING: bool = true;
pub const DEFAULT_PLAY_FINISH_SOUND: bool = true;
pub const DEFAULT_SOUND_VOLUME: f32 = 0.25;
pub const DEFAULT_UI_FONT_SIZE: i32 = 12;
/// Multiplier on the display's own scale (DPI / density), not absolute pixels per point.
pub const DEFAULT_UI_SCALE: f32 = 1.0;
pub const MIN_UI_SCALE: f32 = 0.5;
pub const MAX_UI_SCALE: f32 = 2.5;
pub const DEFAULT_COMPACT_PROGRAM_HEADERS: bool = true;
pub const DEFAULT_WINDOW_BORDER: bool = true;
/// OS title bar / resize chrome around the main window (Settings → Appearance).
pub const DEFAULT_WINDOW_DECORATIONS: bool = true;
pub const DEFAULT_RUN_BUTTON_GLOW: bool = true;
pub const DEFAULT_BACKUP_INTERVAL_HOURS: i32 = 24;
pub const MIN_BACKUP_INTERVAL_HOURS: i32 = 1;
pub const MAX_BACKUP_INTERVAL_HOURS: i32 = 720;
pub const DEFAULT_BACKUP_MAX_KEEP: i32 = 10;
pub const MIN_BACKUP_MAX_KEEP: i32 = 1;
pub const MAX_BACKUP_MAX_KEEP: i32 = 100;
pub const DEFAULT_AUTO_UPDATE_CHECK: bool = true;
pub const DEFAULT_RELEASE_HELD_INPUTS_ON_END: bool = true;
/// Pressing an overlay button while a macro is running requests stop.
pub const DEFAULT_OVERLAY_PRESS_STOPS_RUN: bool = true;
/// Image Search, Find Pixel, and OCR captures include overlay buttons.
pub const DEFAULT_OVERLAY_BUTTONS_IN_DETECTION: bool = true;
/// Overlay buttons stay hidden while the Sqyre window is in front.
pub const DEFAULT_OVERLAY_BUTTONS_OVER_SQYRE: bool = false;
/// Data Editor left-list width as a fraction of the editor body.
pub const DEFAULT_DATA_EDITOR_LEFT_FRAC: f32 = 0.30;
pub const MIN_DATA_EDITOR_LEFT_FRAC: f32 = 0.15;
pub const MAX_DATA_EDITOR_LEFT_FRAC: f32 = 0.75;
/// Macro list side panel width as a fraction of the window.
pub const DEFAULT_MACRO_LIST_FRAC: f32 = 0.30;
pub const MIN_MACRO_LIST_FRAC: f32 = 0.15;
pub const MAX_MACRO_LIST_FRAC: f32 = 0.75;
/// Narrow viewports auto-switch to Compact spacing unless the user opts out.
pub const DEFAULT_AUTO_COMPACT_DENSITY: bool = true;

/// Key that stops a running macro (rdev key name; empty disables panic stop).
pub const DEFAULT_PANIC_KEY: &str = "esc";
/// Length of the desktop click-marker flash.
pub const DEFAULT_CLICK_FLASH_MS: i32 = 360;
pub const MIN_CLICK_FLASH_MS: i32 = 50;
pub const MAX_CLICK_FLASH_MS: i32 = 5_000;
/// Keyboard delay seeded into newly created macros.
pub const DEFAULT_MACRO_KEYBOARD_DELAY_MS: i32 = 25;
/// Mouse delay seeded into newly created macros.
pub const DEFAULT_MACRO_MOUSE_DELAY_MS: i32 = 25;
/// Idle gap (ms) that becomes a Wait action while recording.
pub const DEFAULT_RECORD_WAIT_THRESHOLD_MS: i64 = 40;
pub const MIN_RECORD_WAIT_THRESHOLD_MS: i64 = 5;
pub const MAX_RECORD_WAIT_THRESHOLD_MS: i64 = 2_000;
/// Minimum mouse travel (px) recorded as a point while recording.
pub const DEFAULT_RECORD_MOVE_MIN_DISTANCE: i32 = 8;
pub const MIN_RECORD_MOVE_MIN_DISTANCE: i32 = 1;
pub const MAX_RECORD_MOVE_MIN_DISTANCE: i32 = 100;
/// Press duration (ms) at or below which a recording emits a Tap.
pub const DEFAULT_RECORD_TAP_MAX_MS: i64 = 200;
pub const MIN_RECORD_TAP_MAX_MS: i64 = 50;
pub const MAX_RECORD_TAP_MAX_MS: i64 = 2_000;
/// Snap radius (px) to existing catalog points while recording.
pub const DEFAULT_RECORD_SNAP_POINT_PX: i32 = 24;
pub const MIN_RECORD_SNAP_POINT_PX: i32 = 0;
pub const MAX_RECORD_SNAP_POINT_PX: i32 = 200;
/// Hours between update checks; `0` checks on every launch.
pub const DEFAULT_UPDATE_CHECK_INTERVAL_HOURS: i32 = 0;
pub const MIN_UPDATE_CHECK_INTERVAL_HOURS: i32 = 0;
pub const MAX_UPDATE_CHECK_INTERVAL_HOURS: i32 = 720;

/// Milliseconds between mem.log memory samples.
pub const DEFAULT_MEMORY_SAMPLE_INTERVAL_MS: i32 = 5_000;
pub const MIN_MEMORY_SAMPLE_INTERVAL_MS: i32 = 250;
pub const MAX_MEMORY_SAMPLE_INTERVAL_MS: i32 = 300_000;

/// Long-edge cap (px) for images stored in the action log.
pub const DEFAULT_LOG_IMAGE_MAX_EDGE: i32 = 640;
pub const MIN_LOG_IMAGE_MAX_EDGE: i32 = 64;
pub const MAX_LOG_IMAGE_MAX_EDGE: i32 = 8192;

/// Default While budget when a macro sets `max_iterations` ≤ 0.
pub const DEFAULT_WHILE_MAX_ITERATIONS: i32 = 100_000;
pub const MIN_WHILE_MAX_ITERATIONS: i32 = 1;
pub const MAX_WHILE_MAX_ITERATIONS: i32 = 10_000_000;
/// Default max nested RunMacro depth (including the top-level macro).
pub const DEFAULT_RUN_MACRO_MAX_DEPTH: i32 = 32;
pub const MIN_RUN_MACRO_MAX_DEPTH: i32 = 1;
pub const MAX_RUN_MACRO_MAX_DEPTH: i32 = 256;

/// Log entries kept per action in the Logs window (oldest dropped).
pub const DEFAULT_ACTION_LOG_MAX_ENTRIES: i32 = 5_000;
pub const MIN_ACTION_LOG_MAX_ENTRIES: i32 = 50;
pub const MAX_ACTION_LOG_MAX_ENTRIES: i32 = 50_000;

/// Soft cap for Rayon / search worker threads (used when the machine has more cores).
pub const DEFAULT_WORKER_THREADS_CAP: i32 = 16;
pub const MIN_WORKER_THREADS: i32 = 1;

/// Logical CPU count available to this process (`1` if unknown).
pub fn available_worker_threads() -> i32 {
    std::thread::available_parallelism()
        .map(|n| n.get() as i32)
        .unwrap_or(1)
        .max(MIN_WORKER_THREADS)
}

/// Default worker-thread count: `min(16, available CPUs)`.
pub fn default_worker_threads() -> i32 {
    available_worker_threads().min(DEFAULT_WORKER_THREADS_CAP)
}

/// Absolute path to the settings file (`{sqyre_dir}/settings.yaml`).
pub fn settings_path() -> PathBuf {
    crate::sqyre_dir().join(crate::SETTINGS_FILE)
}

/// XDG pointer that records a relocated data directory (one path per line).
#[cfg(not(target_arch = "wasm32"))]
fn data_dir_pointer_path() -> PathBuf {
    crate::sqyre_config_dir().join("data_dir")
}

/// Apply a relocated data-dir pointer before loading settings from `.sqyre`.
#[cfg(not(target_arch = "wasm32"))]
fn apply_data_dir_pointer() {
    let path = data_dir_pointer_path();
    let Ok(text) = fs::read_to_string(&path) else {
        return;
    };
    let dir = text.lines().next().unwrap_or("").trim();
    if dir.is_empty() {
        let _ = fs::remove_file(&path);
        crate::set_sqyre_dir_override(None);
        return;
    }
    crate::set_sqyre_dir_override(Some(PathBuf::from(dir)));
}

#[cfg(not(target_arch = "wasm32"))]
fn write_data_dir_pointer(sqyre_dir: &str) -> Result<()> {
    let path = data_dir_pointer_path();
    let dir = sqyre_dir.trim();
    if dir.is_empty() {
        if path.exists() {
            fs::remove_file(&path)?;
        }
        return Ok(());
    }
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    crate::atomic_write(&path, format!("{dir}\n"))?;
    Ok(())
}

/// Action-color hex overrides (`#rrggbb`). Empty string = built-in pastel.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct ActionColorPrefs {
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub mouse_keyboard: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub detection: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub variables: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub control_flow: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub miscellaneous: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub wait: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub default: String,
}

impl ActionColorPrefs {
    pub fn get(&self, key: &str) -> &str {
        match key {
            ACTION_COLOR_KEY_MOUSE_KEYBOARD => &self.mouse_keyboard,
            ACTION_COLOR_KEY_DETECTION => &self.detection,
            ACTION_COLOR_KEY_VARIABLES => &self.variables,
            ACTION_COLOR_KEY_CONTROL_FLOW => &self.control_flow,
            ACTION_COLOR_KEY_MISCELLANEOUS => &self.miscellaneous,
            ACTION_COLOR_KEY_WAIT => &self.wait,
            ACTION_COLOR_KEY_DEFAULT => &self.default,
            _ => "",
        }
    }

    pub fn set(&mut self, key: &str, hex: String) {
        match key {
            ACTION_COLOR_KEY_MOUSE_KEYBOARD => self.mouse_keyboard = hex,
            ACTION_COLOR_KEY_DETECTION => self.detection = hex,
            ACTION_COLOR_KEY_VARIABLES => self.variables = hex,
            ACTION_COLOR_KEY_CONTROL_FLOW => self.control_flow = hex,
            ACTION_COLOR_KEY_MISCELLANEOUS => self.miscellaneous = hex,
            ACTION_COLOR_KEY_WAIT => self.wait = hex,
            ACTION_COLOR_KEY_DEFAULT => self.default = hex,
            _ => {}
        }
    }

    pub fn clear_all(&mut self) {
        *self = Self::default();
    }
}

/// How an overlay button reacts to periodic image search.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OverlayVisibilityMode {
    /// No image gate — visibility follows enabled + focus only.
    #[default]
    Off,
    /// Show the button only while a template match is found in the search area.
    ShowWhenFound,
}

/// UI density: how much vertical space separates sections and elements.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UiDensity {
    /// 4 pt between sections, 2 pt between elements inside a section.
    Compact,
    /// 8 pt between sections, 4 pt between elements inside a section.
    #[default]
    Normal,
    /// 12 pt between sections, 8 pt between elements inside a section.
    Spaced,
}

impl UiDensity {
    /// All selectable values, in dropdown order.
    pub const ALL: [Self; 3] = [Self::Compact, Self::Normal, Self::Spaced];

    /// Gap between sections (framed cards, separator breaks, subsection heads).
    pub fn section_gap(self) -> f32 {
        match self {
            Self::Compact => 4.0,
            Self::Normal => 8.0,
            Self::Spaced => 12.0,
        }
    }

    /// Gap between element groups inside a section (field rows, buttons, items).
    pub fn element_gap(self) -> f32 {
        match self {
            Self::Compact => 2.0,
            Self::Normal => 4.0,
            Self::Spaced => 8.0,
        }
    }

    /// Dropdown display name.
    pub fn label(self) -> &'static str {
        match self {
            Self::Compact => "Compact",
            Self::Normal => "Normal",
            Self::Spaced => "Spaced",
        }
    }
}

/// What pressing a running macro's hotkey does.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HotkeyWhileRunning {
    /// Extra presses are dropped while the macro runs.
    #[default]
    Ignore,
    /// Pressing the hotkey again stops the run.
    Stop,
    /// Pressing the hotkey again restarts the macro.
    Restart,
}

impl HotkeyWhileRunning {
    /// All selectable values, in dropdown order.
    pub const ALL: [Self; 3] = [Self::Ignore, Self::Stop, Self::Restart];

    /// Dropdown display name.
    pub fn label(self) -> &'static str {
        match self {
            Self::Ignore => "Ignore",
            Self::Stop => "Stop the run",
            Self::Restart => "Restart",
        }
    }
}

/// Which screen edge the quick-access hub docks to.
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

/// True when `id` is the reserved quick-access hub overlay button.
pub fn is_quick_access_hub_id(id: &str) -> bool {
    id == QUICK_ACCESS_HUB_ID
}

/// Top-left desktop position for a hub face docked to `edge` at `offset`.
///
/// `size_px` and the monitor rect share units (physical desktop pixels when
/// the rect comes from capture monitor metrics).
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
    let (edge, offset) = snap_hub_to_monitor(btn.x, btn.y, size_px, mon_x, mon_y, mon_w, mon_h);
    let (x, y) = hub_position_on_monitor(edge, offset, mon_x, mon_y, mon_w, mon_h, size_px);
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
        .overlay_buttons
        .insert(0, default_quick_access_hub_button());
    true
}

/// Proportional font family used for UI text.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UiFontFamily {
    /// Ubuntu (egui default UI font).
    #[default]
    Standard,
    /// Hack monospace for all UI text.
    Monospace,
}

impl UiFontFamily {
    /// All selectable values, in dropdown order.
    pub const ALL: [Self; 2] = [Self::Standard, Self::Monospace];

    /// Dropdown display name.
    pub fn label(self) -> &'static str {
        match self {
            Self::Standard => "Ubuntu",
            Self::Monospace => "Monospace",
        }
    }
}

/// Per-button image-search gate that can hide an overlay until a template is found.
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
    /// [`UserSettings::overlay_buttons_over_sqyre`] is on.
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

    /// Desktop top-left for the button viewport.
    ///
    /// When [`Self::point`] is set, resolves that catalog point (literal scalars /
    /// arithmetic without macro variables). Falls back to [`Self::x`] / [`Self::y`]
    /// when the point is empty or cannot be resolved.
    pub fn resolved_position(&self, catalog: &crate::ProgramCatalog) -> (f32, f32) {
        let point = self.point.trim();
        if !point.is_empty() {
            let m = Macro::new("", 0, vec![]);
            if let Ok((x, y)) = catalog.resolve_point(&CoordinateRef(point.to_string()), &m) {
                return (x as f32, y as f32);
            }
        }
        (self.x, self.y)
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

/// User-tunable Sqyre preferences.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct UserSettings {
    #[serde(default)]
    pub save_meta_images: bool,
    /// Log entries kept per action in the Logs window (oldest dropped).
    #[serde(default = "default_action_log_max_entries")]
    pub action_log_max_entries: i32,
    #[serde(default)]
    pub highlight_active_action: bool,
    /// Flash a marker on the desktop where each automated mouse press lands.
    #[serde(default)]
    pub click_effect: bool,
    /// Gold box on the desktop at the location shown by a hovered/embedded
    /// image preview tooltip (points and search areas). Off by default.
    #[serde(default)]
    pub preview_desktop_outline: bool,
    /// In catalog lists, programs with a process icon omit the name (icon + child count only).
    #[serde(default = "default_compact_program_headers")]
    pub compact_program_headers: bool,
    /// Data Editor lists keep program headers that have no entries for the current tab.
    #[serde(default)]
    pub show_empty_program_headers: bool,
    /// Gold border just inside the main window edge.
    #[serde(default = "default_window_border")]
    pub window_border: bool,
    /// OS title bar and system resize borders on the main window.
    /// When false the app draws its own edge grips and top-bar drag.
    #[serde(default = "default_window_decorations")]
    pub window_decorations: bool,
    /// Pulsing green halo behind the Run button while it is enabled.
    #[serde(default = "default_run_button_glow")]
    pub run_button_glow: bool,
    #[serde(default = "default_hide_recording")]
    pub hide_app_during_recording: bool,
    /// Release keys/buttons still held from Down/hold actions when a macro ends.
    #[serde(default = "default_release_held_inputs_on_end")]
    pub release_held_inputs_on_end: bool,
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
    /// Safety budget for While actions with `max_iterations` ≤ 0.
    #[serde(default = "default_while_max_iterations")]
    pub while_max_iterations: i32,
    /// Max nested RunMacro depth (including the top-level macro).
    #[serde(default = "default_run_macro_max_depth")]
    pub run_macro_max_depth: i32,
    /// Play an audible cue when a top-level macro run finishes successfully.
    #[serde(default = "default_play_finish_sound")]
    pub play_finish_sound: bool,
    /// Play an audible cue when the user adds macros, actions, or catalog entities.
    #[serde(default = "default_play_add_sound")]
    pub play_add_sound: bool,
    /// Play an audible cue when the user deletes macros, actions, or catalog entities.
    #[serde(default = "default_play_delete_sound")]
    pub play_delete_sound: bool,
    /// Playback volume for app cue sounds (`0.0`–`1.0`).
    #[serde(default = "default_sound_volume")]
    pub sound_volume: f32,
    #[serde(default = "default_close_matches")]
    pub image_search_close_matches_distance: i32,
    /// When true, stop trying later icon variants after one hits on the same search.
    #[serde(default = "default_variant_exit_early")]
    pub image_search_variant_exit_early: bool,
    /// Absolute path override for the `.sqyre` data directory (empty = `~/.sqyre`).
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub sqyre_dir: String,
    #[serde(default = "default_font_size")]
    pub ui_font_size: i32,
    #[serde(default = "default_ui_scale")]
    pub ui_scale: f32,
    /// Section/element spacing preset (Settings → Appearance → Spacing).
    #[serde(default)]
    pub ui_density: UiDensity,
    /// Switch to Compact spacing automatically on narrow viewports (phones,
    /// small windows). Disable to keep the chosen density at all sizes.
    #[serde(default = "default_auto_compact_density")]
    pub auto_compact_density: bool,
    #[serde(default)]
    pub action_colors: ActionColorPrefs,
    /// Per-action-type blank templates for the Add Action picker (YAML action maps).
    #[serde(default, skip_serializing_if = "std::collections::BTreeMap::is_empty")]
    pub action_defaults: std::collections::BTreeMap<String, serde_yaml::Mapping>,
    /// User-configured overlay buttons (per-program; enabled buttons for
    /// [`sqyre_domain::GENERAL_PROGRAM`] ignore other apps, others show when that
    /// program is focused). All of them hide while Sqyre is in front unless
    /// [`Self::overlay_buttons_over_sqyre`] is on.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub overlay_buttons: Vec<OverlayButtonConfig>,
    /// Edge-docked quick-access hub (New Point / Search Area / Screen Cap / Show-Hide).
    /// Automatically zip the data directory on a schedule.
    #[serde(default)]
    pub backup_enabled: bool,
    /// Hours between automatic backups.
    #[serde(default = "default_backup_interval")]
    pub backup_interval_hours: i32,
    /// How many managed `sqyre-backup-*.zip` files to keep.
    #[serde(default = "default_backup_max_keep")]
    pub backup_max_keep: i32,
    /// Unix seconds of the last successful backup (0 = never).
    #[serde(default, skip_serializing_if = "is_zero_i64")]
    pub last_backup_unix: i64,
    /// Check GitHub Releases for a newer Sqyre build on startup.
    #[serde(default = "default_auto_update_check")]
    pub auto_update_check: bool,
    /// Unix seconds of the last successful update check (0 = never).
    #[serde(default, skip_serializing_if = "is_zero_i64")]
    pub last_update_check_unix: i64,
    /// Active macro-list hotkey tag filters (empty = no hotkeys; `""` entry = untagged).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub hotkey_tag_filters: Vec<String>,
    /// When true, focusing a Program sets `hotkey_tag_filters` to that program's macro tags,
    /// which always include its name (cleared when no Program owns focus). When false, filters
    /// are manual only.
    #[serde(default)]
    pub hotkey_tags_while_focused: bool,
    /// Data Editor left list width as a fraction of the editor body (`0.15`–`0.75`).
    #[serde(default = "default_data_editor_left_frac")]
    pub data_editor_left_split: f32,
    /// Data Editor Items / Pixel Check left-list sort.
    #[serde(default, skip_serializing_if = "is_default_catalog_item_sort")]
    pub data_editor_items_sort: CatalogItemSort,
    /// Tag priority when [`Self::data_editor_items_sort`] is [`CatalogItemSort::Tags`].
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub data_editor_items_tag_priority: Vec<String>,
    /// Macro list side panel width as a fraction of the window (`0.15`–`0.75`).
    #[serde(default = "default_macro_list_frac")]
    pub macro_list_split: f32,
    /// Name of the macro selected when Sqyre last ran; reselected on startup.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub last_selected_macro: String,
    /// Rayon worker threads for image search, find-pixel, and capture conversion.
    /// Applied at process start (`build_global`); changing it needs a restart.
    #[serde(default = "default_worker_threads")]
    pub worker_threads: i32,
    /// First-start permissions popup was confirmed; later launches skip it.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub permissions_intro_done: bool,
    /// Write diag.log diagnostics to the data directory.
    #[serde(default)]
    pub diagnostic_logging: bool,
    /// Sleep Wayland screen sharing after the desktop sits still.
    /// The next capture shows a loading icon while the stream wakes.
    #[serde(default = "default_suspend_screen_capture")]
    pub suspend_screen_capture_when_idle: bool,
    /// Write mem.log memory samples to the data directory.
    #[serde(default)]
    pub memory_logging: bool,
    /// Milliseconds between mem.log samples when `memory_logging` is on.
    #[serde(default = "default_memory_sample_interval")]
    pub memory_logging_interval_ms: i32,
    /// Reselect the macro that was selected on exit.
    #[serde(default = "default_restore_last_macro")]
    pub restore_last_macro: bool,
    /// Hide to the system tray instead of quitting on window close.
    #[serde(default)]
    pub close_to_tray: bool,
    /// Start Sqyre automatically on login (Linux XDG autostart).
    #[serde(default)]
    pub launch_on_login: bool,
    /// Launch with the main window hidden in the system tray.
    #[serde(default)]
    pub start_minimized: bool,
    /// Move the cursor back to its pre-run position when a macro ends.
    #[serde(default)]
    pub restore_cursor_on_end: bool,
    /// Click-marker flash length in milliseconds.
    #[serde(default = "default_click_flash_ms")]
    pub click_flash_ms: i32,
    /// Long-edge cap (px) for images stored in the action log.
    #[serde(default = "default_log_image_max_edge")]
    pub log_image_max_edge: i32,
    /// Keyboard delay seeded into newly created macros.
    #[serde(default = "default_macro_keyboard_delay")]
    pub default_keyboard_delay_ms: i32,
    /// Mouse delay seeded into newly created macros.
    #[serde(default = "default_macro_mouse_delay")]
    pub default_mouse_delay_ms: i32,
    /// Single key that stops a running macro (rdev name like `esc`, `f12`;
    /// empty disables panic stop).
    #[serde(default = "default_panic_key_string")]
    pub panic_key: String,
    /// What a running macro's hotkey does on repeat presses.
    #[serde(default)]
    pub hotkey_while_running: HotkeyWhileRunning,
    /// `press`/`release` trigger seeded into newly created macros.
    #[serde(default = "default_hotkey_trigger")]
    pub default_hotkey_trigger: String,
    /// Idle gap (ms) that becomes a Wait action while recording.
    #[serde(default = "default_record_wait_threshold")]
    pub record_wait_threshold_ms: i64,
    /// Minimum mouse travel (px) recorded as a point while recording.
    #[serde(default = "default_record_move_min_distance")]
    pub record_move_min_distance: i32,
    /// Press duration (ms) at or below which a recording emits a Tap.
    #[serde(default = "default_record_tap_max")]
    pub record_tap_max_ms: i64,
    /// Snap radius (px) to existing catalog points while recording.
    #[serde(default = "default_record_snap_point")]
    pub record_snap_point_px: i32,
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
    /// Custom macro-finished cue file (empty = built-in).
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub finish_sound_file: String,
    /// Custom add-item cue file (empty = built-in).
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub add_sound_file: String,
    /// Custom delete-item cue file (empty = built-in).
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub delete_sound_file: String,
    /// Custom backups folder (empty = `{sqyre_dir}/backups`).
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub backup_dir: String,
    /// Hours between update checks (0 = every launch).
    #[serde(default)]
    pub update_check_interval_hours: i32,
    /// Offer prerelease builds in update checks.
    #[serde(default)]
    pub update_include_prerelease: bool,
    /// UI accent color `#rrggbb` (empty = Sqyre gold).
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub accent_color: String,
    /// Monospace font size (0 = same as UI font).
    #[serde(default)]
    pub mono_font_size: i32,
    /// UI text font family.
    #[serde(default)]
    pub ui_font_family: UiFontFamily,
    /// Last main-window inner size in points (0 = startup default).
    #[serde(default, skip_serializing_if = "is_zero_f32")]
    pub window_width: f32,
    /// Last main-window inner size in points (0 = startup default).
    #[serde(default, skip_serializing_if = "is_zero_f32")]
    pub window_height: f32,
    /// Last main-window outer position in points (None = platform default).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub window_pos_x: Option<f32>,
    /// Last main-window outer position in points (None = platform default).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub window_pos_y: Option<f32>,
    /// Restore the window maximized on startup.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub window_maximized: bool,
}

fn default_hide_recording() -> bool {
    DEFAULT_HIDE_APP_DURING_RECORDING
}
fn default_release_held_inputs_on_end() -> bool {
    DEFAULT_RELEASE_HELD_INPUTS_ON_END
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
fn default_action_log_max_entries() -> i32 {
    DEFAULT_ACTION_LOG_MAX_ENTRIES
}
fn default_while_max_iterations() -> i32 {
    DEFAULT_WHILE_MAX_ITERATIONS
}
fn default_run_macro_max_depth() -> i32 {
    DEFAULT_RUN_MACRO_MAX_DEPTH
}
fn default_play_finish_sound() -> bool {
    DEFAULT_PLAY_FINISH_SOUND
}
fn default_play_add_sound() -> bool {
    true
}
fn default_play_delete_sound() -> bool {
    true
}
fn default_sound_volume() -> f32 {
    DEFAULT_SOUND_VOLUME
}
fn default_close_matches() -> i32 {
    DEFAULT_IMAGE_SEARCH_CLOSE_MATCHES_DISTANCE
}
fn default_variant_exit_early() -> bool {
    DEFAULT_IMAGE_SEARCH_VARIANT_EXIT_EARLY
}
fn default_restore_last_macro() -> bool {
    true
}
fn default_memory_sample_interval() -> i32 {
    DEFAULT_MEMORY_SAMPLE_INTERVAL_MS
}
fn default_log_image_max_edge() -> i32 {
    DEFAULT_LOG_IMAGE_MAX_EDGE
}
fn default_click_flash_ms() -> i32 {
    DEFAULT_CLICK_FLASH_MS
}
fn default_macro_keyboard_delay() -> i32 {
    DEFAULT_MACRO_KEYBOARD_DELAY_MS
}
fn default_macro_mouse_delay() -> i32 {
    DEFAULT_MACRO_MOUSE_DELAY_MS
}
fn default_panic_key_string() -> String {
    DEFAULT_PANIC_KEY.to_string()
}
fn default_hotkey_trigger() -> String {
    sqyre_domain::HOTKEY_TRIGGER_PRESS.to_string()
}
fn default_record_wait_threshold() -> i64 {
    DEFAULT_RECORD_WAIT_THRESHOLD_MS
}
fn default_record_move_min_distance() -> i32 {
    DEFAULT_RECORD_MOVE_MIN_DISTANCE
}
fn default_record_tap_max() -> i64 {
    DEFAULT_RECORD_TAP_MAX_MS
}
fn default_record_snap_point() -> i32 {
    DEFAULT_RECORD_SNAP_POINT_PX
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
fn default_compact_program_headers() -> bool {
    DEFAULT_COMPACT_PROGRAM_HEADERS
}
fn default_window_border() -> bool {
    DEFAULT_WINDOW_BORDER
}
fn default_window_decorations() -> bool {
    DEFAULT_WINDOW_DECORATIONS
}
fn default_suspend_screen_capture() -> bool {
    true
}
fn default_run_button_glow() -> bool {
    DEFAULT_RUN_BUTTON_GLOW
}
fn default_font_size() -> i32 {
    DEFAULT_UI_FONT_SIZE
}
fn default_ui_scale() -> f32 {
    DEFAULT_UI_SCALE
}
fn default_backup_interval() -> i32 {
    DEFAULT_BACKUP_INTERVAL_HOURS
}
fn default_backup_max_keep() -> i32 {
    DEFAULT_BACKUP_MAX_KEEP
}
fn default_auto_update_check() -> bool {
    DEFAULT_AUTO_UPDATE_CHECK
}
fn default_data_editor_left_frac() -> f32 {
    DEFAULT_DATA_EDITOR_LEFT_FRAC
}
fn default_macro_list_frac() -> f32 {
    DEFAULT_MACRO_LIST_FRAC
}
fn default_auto_compact_density() -> bool {
    DEFAULT_AUTO_COMPACT_DENSITY
}
fn is_default_catalog_item_sort(v: &CatalogItemSort) -> bool {
    *v == CatalogItemSort::default()
}

fn is_zero_i64(v: &i64) -> bool {
    *v == 0
}

fn is_zero_f32(v: &f32) -> bool {
    *v == 0.0
}

impl Default for UserSettings {
    fn default() -> Self {
        Self {
            save_meta_images: false,
            action_log_max_entries: DEFAULT_ACTION_LOG_MAX_ENTRIES,
            highlight_active_action: false,
            click_effect: false,
            preview_desktop_outline: false,
            compact_program_headers: DEFAULT_COMPACT_PROGRAM_HEADERS,
            show_empty_program_headers: false,
            window_border: DEFAULT_WINDOW_BORDER,
            window_decorations: DEFAULT_WINDOW_DECORATIONS,
            run_button_glow: DEFAULT_RUN_BUTTON_GLOW,
            hide_app_during_recording: DEFAULT_HIDE_APP_DURING_RECORDING,
            release_held_inputs_on_end: DEFAULT_RELEASE_HELD_INPUTS_ON_END,
            overlay_press_stops_run: DEFAULT_OVERLAY_PRESS_STOPS_RUN,
            overlay_buttons_in_detection: DEFAULT_OVERLAY_BUTTONS_IN_DETECTION,
            overlay_buttons_over_sqyre: DEFAULT_OVERLAY_BUTTONS_OVER_SQYRE,
            while_max_iterations: DEFAULT_WHILE_MAX_ITERATIONS,
            run_macro_max_depth: DEFAULT_RUN_MACRO_MAX_DEPTH,
            play_finish_sound: DEFAULT_PLAY_FINISH_SOUND,
            play_add_sound: true,
            play_delete_sound: true,
            sound_volume: DEFAULT_SOUND_VOLUME,
            image_search_close_matches_distance: DEFAULT_IMAGE_SEARCH_CLOSE_MATCHES_DISTANCE,
            image_search_variant_exit_early: DEFAULT_IMAGE_SEARCH_VARIANT_EXIT_EARLY,
            sqyre_dir: String::new(),
            ui_font_size: DEFAULT_UI_FONT_SIZE,
            ui_scale: DEFAULT_UI_SCALE,
            ui_density: UiDensity::default(),
            auto_compact_density: DEFAULT_AUTO_COMPACT_DENSITY,
            action_colors: ActionColorPrefs::default(),
            action_defaults: std::collections::BTreeMap::new(),
            overlay_buttons: Vec::new(),
            backup_enabled: false,
            backup_interval_hours: DEFAULT_BACKUP_INTERVAL_HOURS,
            backup_max_keep: DEFAULT_BACKUP_MAX_KEEP,
            last_backup_unix: 0,
            auto_update_check: DEFAULT_AUTO_UPDATE_CHECK,
            last_update_check_unix: 0,
            hotkey_tag_filters: Vec::new(),
            hotkey_tags_while_focused: false,
            data_editor_left_split: DEFAULT_DATA_EDITOR_LEFT_FRAC,
            data_editor_items_sort: CatalogItemSort::default(),
            data_editor_items_tag_priority: Vec::new(),
            macro_list_split: DEFAULT_MACRO_LIST_FRAC,
            last_selected_macro: String::new(),
            worker_threads: default_worker_threads(),
            permissions_intro_done: false,
            diagnostic_logging: false,
            suspend_screen_capture_when_idle: true,
            memory_logging: false,
            memory_logging_interval_ms: DEFAULT_MEMORY_SAMPLE_INTERVAL_MS,
            restore_last_macro: true,
            close_to_tray: false,
            launch_on_login: false,
            start_minimized: false,
            restore_cursor_on_end: false,
            click_flash_ms: DEFAULT_CLICK_FLASH_MS,
            log_image_max_edge: DEFAULT_LOG_IMAGE_MAX_EDGE,
            default_keyboard_delay_ms: DEFAULT_MACRO_KEYBOARD_DELAY_MS,
            default_mouse_delay_ms: DEFAULT_MACRO_MOUSE_DELAY_MS,
            panic_key: DEFAULT_PANIC_KEY.to_string(),
            hotkey_while_running: HotkeyWhileRunning::default(),
            default_hotkey_trigger: sqyre_domain::HOTKEY_TRIGGER_PRESS.to_string(),
            record_wait_threshold_ms: DEFAULT_RECORD_WAIT_THRESHOLD_MS,
            record_move_min_distance: DEFAULT_RECORD_MOVE_MIN_DISTANCE,
            record_tap_max_ms: DEFAULT_RECORD_TAP_MAX_MS,
            record_snap_point_px: DEFAULT_RECORD_SNAP_POINT_PX,
            overlay_default_size: DEFAULT_OVERLAY_BUTTON_SIZE,
            overlay_default_corner_radius: DEFAULT_OVERLAY_CORNER_RADIUS,
            overlay_default_border_width: DEFAULT_OVERLAY_BORDER_WIDTH,
            overlay_default_accent_color: String::new(),
            overlay_default_icon_color: String::new(),
            overlay_gate_default_interval_ms: DEFAULT_OVERLAY_GATE_INTERVAL_MS,
            finish_sound_file: String::new(),
            add_sound_file: String::new(),
            delete_sound_file: String::new(),
            backup_dir: String::new(),
            update_check_interval_hours: DEFAULT_UPDATE_CHECK_INTERVAL_HOURS,
            update_include_prerelease: false,
            accent_color: String::new(),
            mono_font_size: 0,
            ui_font_family: UiFontFamily::default(),
            window_width: 0.0,
            window_height: 0.0,
            window_pos_x: None,
            window_pos_y: None,
            window_maximized: false,
        }
    }
}

impl UserSettings {
    pub fn load_default() -> Result<Self> {
        #[cfg(target_arch = "wasm32")]
        {
            Ok(Self::default())
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            apply_data_dir_pointer();
            Self::load_from_path(settings_path())
        }
    }

    pub fn load_from_path(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        if !path.exists() {
            return Ok(Self::default());
        }
        let text = fs::read_to_string(path)?;
        const MAX_SETTINGS_YAML_BYTES: usize = 1024 * 1024;
        if text.len() > MAX_SETTINGS_YAML_BYTES {
            return Err(PersistError::Message(format!(
                "settings.yaml too large ({} bytes; max {MAX_SETTINGS_YAML_BYTES})",
                text.len()
            )));
        }
        sqyre_serialize::check_yaml_safety(&text)?;
        let mut s: Self = serde_yaml::from_str(&text)?;
        s.clamp();
        Ok(s)
    }

    pub fn save_default(&self) -> Result<()> {
        #[cfg(target_arch = "wasm32")]
        {
            let _ = self;
            Ok(())
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            // The runtime override is set by whoever relocated the data dir
            // (settings UI, pointer file at load, tests); an empty `sqyre_dir`
            // must not clear it or saves would leak to the real ~/.sqyre.
            write_data_dir_pointer(&self.sqyre_dir)?;
            self.save_to_path(settings_path())
        }
    }

    pub fn save_to_path(&self, path: impl AsRef<Path>) -> Result<()> {
        let path = path.as_ref();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let mut clamped = self.clone();
        clamped.clamp();
        crate::atomic_write(path, serde_yaml::to_string(&clamped)?)?;
        Ok(())
    }

    /// Propagate a macro rename into overlay button macro refs.
    pub fn rename_overlay_macro(&mut self, old_name: &str, new_name: &str) -> bool {
        let old_name = old_name.trim();
        let new_name = new_name.trim();
        if old_name == new_name || old_name.is_empty() || new_name.is_empty() {
            return false;
        }
        let mut changed = false;
        for btn in &mut self.overlay_buttons {
            if btn.macro_name == old_name {
                btn.macro_name = new_name.to_string();
                changed = true;
            }
        }
        changed
    }

    /// Propagate a catalog point rename into overlay button point refs.
    pub fn rename_overlay_point_entity(
        &mut self,
        program: &str,
        old_name: &str,
        new_name: &str,
    ) -> bool {
        let mut changed = false;
        for btn in &mut self.overlay_buttons {
            let cur = CoordinateRef(btn.point.clone());
            let next = rename_coordinate_entity(&cur, program, old_name, new_name);
            if next != cur {
                btn.point = next.0;
                changed = true;
            }
        }
        changed
    }

    /// Propagate a catalog search-area rename into overlay visibility-gate refs.
    pub fn rename_overlay_search_area_entity(
        &mut self,
        program: &str,
        old_name: &str,
        new_name: &str,
    ) -> bool {
        let mut changed = false;
        for btn in &mut self.overlay_buttons {
            let cur = CoordinateRef(btn.visibility_gate.search_area.clone());
            let next = rename_coordinate_entity(&cur, program, old_name, new_name);
            if next != cur {
                btn.visibility_gate.search_area = next.0;
                changed = true;
            }
        }
        changed
    }

    /// Move hotkey tag filters that are a renamed program's name tag (or a path under it).
    pub fn rename_hotkey_tag_program(&mut self, old_program: &str, new_program: &str) -> bool {
        sqyre_domain::rename_tags_for_program(
            &mut self.hotkey_tag_filters,
            old_program,
            new_program,
        )
    }

    /// Propagate a program rename into overlay button point / gate search-area refs.
    pub fn rename_overlay_point_program(&mut self, old_program: &str, new_program: &str) -> bool {
        let mut changed = false;
        for btn in &mut self.overlay_buttons {
            let cur = CoordinateRef(btn.point.clone());
            let next = rename_coordinate_program(&cur, old_program, new_program);
            if next != cur {
                btn.point = next.0;
                changed = true;
            }
            let cur = CoordinateRef(btn.visibility_gate.search_area.clone());
            let next = rename_coordinate_program(&cur, old_program, new_program);
            if next != cur {
                btn.visibility_gate.search_area = next.0;
                changed = true;
            }
        }
        changed
    }

    /// Remove overlay buttons bound to a deleted program.
    ///
    /// The seeded Quick Access hub is kept even if its program name matches.
    pub fn remove_overlay_buttons_for_program(&mut self, program: &str) -> bool {
        let before = self.overlay_buttons.len();
        self.overlay_buttons
            .retain(|btn| btn.program != program || is_quick_access_hub_id(&btn.id));
        self.overlay_buttons.len() != before
    }

    /// Clear overlay button point refs that target a deleted catalog point.
    pub fn clear_overlay_point_refs(&mut self, program: &str, name: &str) -> bool {
        let mut changed = false;
        for btn in &mut self.overlay_buttons {
            let cur = CoordinateRef(btn.point.clone());
            if cur.is_empty() || cur.name() != name {
                continue;
            }
            let matches = match cur.program() {
                Some(p) => p == program,
                None => true,
            };
            if matches {
                btn.point.clear();
                changed = true;
            }
        }
        changed
    }

    /// Clear overlay visibility-gate search-area refs that target a deleted area.
    pub fn clear_overlay_search_area_refs(&mut self, program: &str, name: &str) -> bool {
        let mut changed = false;
        for btn in &mut self.overlay_buttons {
            let cur = CoordinateRef(btn.visibility_gate.search_area.clone());
            if cur.is_empty() || cur.name() != name {
                continue;
            }
            let matches = match cur.program() {
                Some(p) => p == program,
                None => true,
            };
            if matches {
                btn.visibility_gate.search_area.clear();
                changed = true;
            }
        }
        changed
    }

    /// Clamp numeric ranges to settings UI bounds.
    pub fn clamp(&mut self) {
        self.image_search_close_matches_distance =
            self.image_search_close_matches_distance.clamp(0, 100);
        self.ui_font_size = self.ui_font_size.clamp(10, 28);
        if self.ui_scale <= 0.0 {
            self.ui_scale = DEFAULT_UI_SCALE;
        }
        self.ui_scale = ((self.ui_scale * 10.0).round() / 10.0).clamp(MIN_UI_SCALE, MAX_UI_SCALE);
        if !self.sound_volume.is_finite() {
            self.sound_volume = DEFAULT_SOUND_VOLUME;
        }
        self.sound_volume = self.sound_volume.clamp(0.0, 1.0);
        if self.while_max_iterations <= 0 {
            self.while_max_iterations = DEFAULT_WHILE_MAX_ITERATIONS;
        }
        self.while_max_iterations = self
            .while_max_iterations
            .clamp(MIN_WHILE_MAX_ITERATIONS, MAX_WHILE_MAX_ITERATIONS);
        if self.run_macro_max_depth <= 0 {
            self.run_macro_max_depth = DEFAULT_RUN_MACRO_MAX_DEPTH;
        }
        self.run_macro_max_depth = self
            .run_macro_max_depth
            .clamp(MIN_RUN_MACRO_MAX_DEPTH, MAX_RUN_MACRO_MAX_DEPTH);
        if self.action_log_max_entries <= 0 {
            self.action_log_max_entries = DEFAULT_ACTION_LOG_MAX_ENTRIES;
        }
        self.action_log_max_entries = self
            .action_log_max_entries
            .clamp(MIN_ACTION_LOG_MAX_ENTRIES, MAX_ACTION_LOG_MAX_ENTRIES);
        if self.worker_threads <= 0 {
            self.worker_threads = default_worker_threads();
        }
        self.worker_threads = self
            .worker_threads
            .clamp(MIN_WORKER_THREADS, available_worker_threads());
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
        if self.backup_interval_hours < MIN_BACKUP_INTERVAL_HOURS {
            self.backup_interval_hours = DEFAULT_BACKUP_INTERVAL_HOURS;
        }
        self.backup_interval_hours = self
            .backup_interval_hours
            .clamp(MIN_BACKUP_INTERVAL_HOURS, MAX_BACKUP_INTERVAL_HOURS);
        if self.backup_max_keep < MIN_BACKUP_MAX_KEEP {
            self.backup_max_keep = DEFAULT_BACKUP_MAX_KEEP;
        }
        self.backup_max_keep = self
            .backup_max_keep
            .clamp(MIN_BACKUP_MAX_KEEP, MAX_BACKUP_MAX_KEEP);
        if self.last_backup_unix < 0 {
            self.last_backup_unix = 0;
        }
        if self.last_update_check_unix < 0 {
            self.last_update_check_unix = 0;
        }
        if !self.data_editor_left_split.is_finite() {
            self.data_editor_left_split = DEFAULT_DATA_EDITOR_LEFT_FRAC;
        }
        self.data_editor_left_split = self
            .data_editor_left_split
            .clamp(MIN_DATA_EDITOR_LEFT_FRAC, MAX_DATA_EDITOR_LEFT_FRAC);
        if !self.macro_list_split.is_finite() || self.macro_list_split <= 0.0 {
            self.macro_list_split = DEFAULT_MACRO_LIST_FRAC;
        }
        self.macro_list_split = self
            .macro_list_split
            .clamp(MIN_MACRO_LIST_FRAC, MAX_MACRO_LIST_FRAC);
        if self.click_flash_ms <= 0 {
            self.click_flash_ms = DEFAULT_CLICK_FLASH_MS;
        }
        self.click_flash_ms = self
            .click_flash_ms
            .clamp(MIN_CLICK_FLASH_MS, MAX_CLICK_FLASH_MS);
        self.log_image_max_edge = self
            .log_image_max_edge
            .clamp(MIN_LOG_IMAGE_MAX_EDGE, MAX_LOG_IMAGE_MAX_EDGE);
        self.default_keyboard_delay_ms = self.default_keyboard_delay_ms.clamp(0, 60_000);
        self.default_mouse_delay_ms = self.default_mouse_delay_ms.clamp(0, 60_000);
        self.panic_key = self.panic_key.trim().to_ascii_lowercase();
        self.default_hotkey_trigger =
            sqyre_domain::Macro::parse_hotkey_trigger(&self.default_hotkey_trigger);
        if self.record_wait_threshold_ms < MIN_RECORD_WAIT_THRESHOLD_MS {
            self.record_wait_threshold_ms = DEFAULT_RECORD_WAIT_THRESHOLD_MS;
        }
        self.record_wait_threshold_ms = self
            .record_wait_threshold_ms
            .clamp(MIN_RECORD_WAIT_THRESHOLD_MS, MAX_RECORD_WAIT_THRESHOLD_MS);
        if self.record_move_min_distance < MIN_RECORD_MOVE_MIN_DISTANCE {
            self.record_move_min_distance = DEFAULT_RECORD_MOVE_MIN_DISTANCE;
        }
        self.record_move_min_distance = self
            .record_move_min_distance
            .clamp(MIN_RECORD_MOVE_MIN_DISTANCE, MAX_RECORD_MOVE_MIN_DISTANCE);
        if self.record_tap_max_ms < MIN_RECORD_TAP_MAX_MS {
            self.record_tap_max_ms = DEFAULT_RECORD_TAP_MAX_MS;
        }
        self.record_tap_max_ms = self
            .record_tap_max_ms
            .clamp(MIN_RECORD_TAP_MAX_MS, MAX_RECORD_TAP_MAX_MS);
        self.record_snap_point_px = self
            .record_snap_point_px
            .clamp(MIN_RECORD_SNAP_POINT_PX, MAX_RECORD_SNAP_POINT_PX);
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
        if !self.accent_color.trim().is_empty()
            && sqyre_domain::parse_hex_color(self.accent_color.trim()).is_none()
        {
            self.accent_color.clear();
        }
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
        self.update_check_interval_hours = self.update_check_interval_hours.clamp(
            MIN_UPDATE_CHECK_INTERVAL_HOURS,
            MAX_UPDATE_CHECK_INTERVAL_HOURS,
        );
        self.mono_font_size = self.mono_font_size.clamp(0, 40);
        self.memory_logging_interval_ms = self
            .memory_logging_interval_ms
            .clamp(MIN_MEMORY_SAMPLE_INTERVAL_MS, MAX_MEMORY_SAMPLE_INTERVAL_MS);
        if !self.window_width.is_finite() || self.window_width < 0.0 {
            self.window_width = 0.0;
        }
        if !self.window_height.is_finite() || self.window_height < 0.0 {
            self.window_height = 0.0;
        }
        if self.window_pos_x.is_some_and(|v| !v.is_finite()) {
            self.window_pos_x = None;
        }
        if self.window_pos_y.is_some_and(|v| !v.is_finite()) {
            self.window_pos_y = None;
        }
        let _ = ensure_quick_access_hub_button(self);
    }

    /// Apply `sqyre_dir` override to the process-wide data directory.
    pub fn apply_sqyre_dir_override(&self) {
        let path = self.sqyre_dir.trim();
        if path.is_empty() {
            crate::set_sqyre_dir_override(None);
        } else {
            crate::set_sqyre_dir_override(Some(PathBuf::from(path)));
        }
    }

    /// Apply `backup_dir` override to the process-wide backups directory.
    pub fn apply_backup_dir_override(&self) {
        let path = self.backup_dir.trim();
        if path.is_empty() {
            crate::set_backup_dir_override(None);
        } else {
            crate::set_backup_dir_override(Some(PathBuf::from(path)));
        }
    }
}

/// Open `path` in the platform file manager.
pub fn open_path_in_file_manager(path: impl AsRef<Path>) -> Result<()> {
    let path = path.as_ref();
    if path.as_os_str().is_empty() {
        return Err(PersistError::Message("open folder: empty path".into()));
    }
    if !path.exists() {
        return Err(PersistError::Message(format!(
            "open folder {}: path does not exist",
            path.display()
        )));
    }
    let status = {
        #[cfg(target_os = "windows")]
        {
            std::process::Command::new("explorer").arg(path).status()
        }
        #[cfg(target_os = "macos")]
        {
            std::process::Command::new("open").arg(path).status()
        }
        #[cfg(not(any(target_os = "windows", target_os = "macos")))]
        {
            std::process::Command::new("xdg-open").arg(path).status()
        }
    };
    match status {
        Ok(s) if s.success() => Ok(()),
        Ok(s) => Err(PersistError::Message(format!(
            "open folder {}: exited {}",
            path.display(),
            s
        ))),
        Err(e) => Err(PersistError::Io(e)),
    }
}

/// Ensure the Sqyre data directory exists and open it in the file manager.
pub fn open_sqyre_dir() -> Result<()> {
    crate::initialize_directories()?;
    open_path_in_file_manager(crate::sqyre_dir())
}

/// Move `src` to `dst`. Falls back to recursive copy on cross-device rename.
/// `dst` must not already exist.
pub fn move_dir(src: impl AsRef<Path>, dst: impl AsRef<Path>) -> Result<()> {
    let src = src.as_ref();
    let dst = dst.as_ref();
    if src == dst {
        return Ok(());
    }
    if dst.exists() {
        return Err(PersistError::Message(format!(
            "move {} to {}: destination already exists",
            src.display(),
            dst.display()
        )));
    }
    if let Some(parent) = dst.parent() {
        fs::create_dir_all(parent)?;
    }
    match fs::rename(src, dst) {
        Ok(()) => Ok(()),
        Err(e) if is_cross_device(&e) => {
            copy_tree(src, dst)?;
            fs::remove_dir_all(src)?;
            Ok(())
        }
        Err(e) => Err(PersistError::Io(e)),
    }
}

fn is_cross_device(err: &std::io::Error) -> bool {
    // Linux EXDEV = 18; Windows ERROR_NOT_SAME_DEVICE = 17.
    matches!(err.raw_os_error(), Some(17) | Some(18))
}

fn copy_tree(src: &Path, dst: &Path) -> Result<()> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let from = entry.path();
        let to = dst.join(entry.file_name());
        let ft = entry.file_type()?;
        if ft.is_dir() {
            copy_tree(&from, &to)?;
        } else {
            fs::copy(&from, &to)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn roundtrip_settings() {
        let dir = tempdir().unwrap();
        let path = dir.path().join(crate::SETTINGS_FILE);
        let mut s = UserSettings {
            save_meta_images: true,
            highlight_active_action: true,
            show_empty_program_headers: true,
            image_search_close_matches_distance: 25,
            image_search_variant_exit_early: false,
            ui_scale: 1.2,
            ui_density: UiDensity::Spaced,
            hotkey_tag_filters: vec!["combat".into()],
            last_selected_macro: "farm loop".into(),
            permissions_intro_done: true,
            ..Default::default()
        };
        s.action_colors.detection = "#aabbcc".into();
        s.overlay_buttons.push(OverlayButtonConfig {
            id: "btn-1".into(),
            program: "Demo Game".into(),
            enabled: true,
            macro_name: "demo".into(),
            icon: "bolt".into(),
            point: String::new(),
            x: 100.0,
            y: 200.0,
            size: 64.0,
            corner_radius: 12.0,
            border_width: 2.0,
            border_color: "#ff0000".into(),
            border_alpha: 200,
            bg_color: "#112233".into(),
            bg_alpha: 128,
            icon_color: "#abcdef".into(),
            icon_alpha: 240,
            icon_hover_color: "#fedcba".into(),
            visibility_gate: OverlayVisibilityGate::default(),
        });
        s.save_to_path(&path).unwrap();

        let loaded = UserSettings::load_from_path(&path).unwrap();
        assert!(loaded.save_meta_images);
        assert!(loaded.highlight_active_action);
        assert_eq!(loaded.image_search_close_matches_distance, 25);
        assert!(!loaded.image_search_variant_exit_early);
        assert!((loaded.ui_scale - 1.2).abs() < f32::EPSILON);
        assert_eq!(loaded.action_colors.detection, "#aabbcc");
        assert_eq!(loaded.overlay_buttons.len(), 1);
        assert!(loaded.overlay_buttons[0].enabled);
        assert_eq!(loaded.overlay_buttons[0].program, "Demo Game");
        assert_eq!(loaded.overlay_buttons[0].macro_name, "demo");
        assert_eq!(loaded.overlay_buttons[0].icon, "bolt");
        assert!((loaded.overlay_buttons[0].size - 64.0).abs() < f32::EPSILON);
        assert!((loaded.overlay_buttons[0].corner_radius - 12.0).abs() < f32::EPSILON);
        assert_eq!(loaded.overlay_buttons[0].border_color, "#ff0000");
        assert_eq!(loaded.overlay_buttons[0].border_alpha, 200);
        assert_eq!(loaded.overlay_buttons[0].bg_color, "#112233");
        assert_eq!(loaded.overlay_buttons[0].bg_alpha, 128);
        assert_eq!(loaded.overlay_buttons[0].icon_color, "#abcdef");
        assert_eq!(loaded.overlay_buttons[0].icon_hover_color, "#fedcba");
        assert_eq!(loaded.hotkey_tag_filters, vec!["combat".to_string()]);
        assert_eq!(loaded.ui_density, UiDensity::Spaced);
        assert_eq!(loaded.last_selected_macro, "farm loop");
        assert!(loaded.permissions_intro_done);
        assert!(loaded.overlay_press_stops_run);
        assert!(loaded.overlay_buttons_in_detection);
        assert!(!loaded.overlay_buttons_over_sqyre);
        assert!(!loaded.hotkey_tags_while_focused);
        assert!(
            (loaded.data_editor_left_split - DEFAULT_DATA_EDITOR_LEFT_FRAC).abs() < f32::EPSILON
        );
        assert!(
            loaded.overlay_buttons.is_empty(),
            "defaults do not seed the hub until sanitize"
        );
    }

    #[test]
    fn ensure_quick_access_hub_button_seeds_and_repairs() {
        let mut s = UserSettings::default();
        assert!(ensure_quick_access_hub_button(&mut s));
        assert_eq!(s.overlay_buttons.len(), 1);
        let hub = &s.overlay_buttons[0];
        assert_eq!(hub.id, QUICK_ACCESS_HUB_ID);
        assert_eq!(hub.program, sqyre_domain::GENERAL_PROGRAM);
        assert_eq!(hub.macro_name, QUICK_ACCESS_HUB_ID);
        assert_eq!(hub.icon, QUICK_ACCESS_HUB_ICON);
        assert_eq!(hub.display_name(), QUICK_ACCESS_HUB_LABEL);
        assert_eq!(hub.border_width, 0.0);
        assert_eq!(hub.bg_alpha, 0);
        assert_eq!(hub.icon_alpha, DEFAULT_QUICK_ACCESS_OPACITY);
        assert!(!ensure_quick_access_hub_button(&mut s));

        s.overlay_buttons[0].program = "Other".into();
        s.overlay_buttons[0].macro_name = "wrong".into();
        s.overlay_buttons[0].icon = "bolt".into();
        s.overlay_buttons[0].point = "General~Monitor 1 Center".into();
        s.overlay_buttons[0].border_width = 2.0;
        s.overlay_buttons[0].border_alpha = 200;
        s.overlay_buttons[0].bg_alpha = 128;
        assert!(ensure_quick_access_hub_button(&mut s));
        let hub = &s.overlay_buttons[0];
        assert_eq!(hub.program, sqyre_domain::GENERAL_PROGRAM);
        assert_eq!(hub.macro_name, QUICK_ACCESS_HUB_ID);
        assert_eq!(hub.icon, QUICK_ACCESS_HUB_ICON);
        assert!(hub.point.is_empty());
        assert_eq!(hub.border_width, 0.0);
        assert_eq!(hub.border_alpha, 0);
        assert_eq!(hub.bg_alpha, 0);
    }

    #[test]
    fn clamp_seeds_quick_access_hub_button() {
        let mut s = UserSettings::default();
        s.clamp();
        assert!(s
            .overlay_buttons
            .iter()
            .any(|b| is_quick_access_hub_id(&b.id)));
    }

    #[test]
    fn snap_hub_prefers_nearest_edge() {
        let (edge, offset) = snap_hub_to_monitor(0.0, 200.0, 26.0, 0.0, 0.0, 1920.0, 1080.0);
        assert_eq!(edge, HubEdge::Left);
        assert!((offset - 200.0 / (1080.0 - 26.0)).abs() < 0.01);

        let (edge, offset) = snap_hub_to_monitor(1894.0, 540.0, 26.0, 0.0, 0.0, 1920.0, 1080.0);
        assert_eq!(edge, HubEdge::Right);
        assert!((offset - 540.0 / (1080.0 - 26.0)).abs() < 0.01);

        let (x, y) = hub_position_on_monitor(HubEdge::Right, 0.5, 0.0, 0.0, 1920.0, 1080.0, 26.0);
        assert!((x - (1920.0 - 26.0)).abs() < f32::EPSILON);
        assert!((y - 0.5 * (1080.0 - 26.0)).abs() < f32::EPSILON);

        // Physical face size at 2× must not be clamped to the points max (128).
        let (x2, _) = hub_position_on_monitor(HubEdge::Right, 0.5, 0.0, 0.0, 3840.0, 2160.0, 52.0);
        assert!((x2 - (3840.0 - 52.0)).abs() < f32::EPSILON);

        let mut btn = default_quick_access_hub_button();
        btn.x = 100.0;
        btn.y = 400.0;
        let edge = snap_hub_button_to_monitor(&mut btn, 0.0, 0.0, 1920.0, 1080.0, 26.0);
        assert_eq!(edge, HubEdge::Left);
        assert!((btn.x - 0.0).abs() < f32::EPSILON);
        assert!((btn.y - 400.0).abs() < 1.0);
    }

    #[test]
    fn data_editor_items_sort_and_tag_priority_roundtrip() {
        let s = UserSettings {
            data_editor_items_sort: CatalogItemSort::Tags,
            data_editor_items_tag_priority: vec!["rare".into(), "heal".into()],
            ..Default::default()
        };
        let yaml = serde_yaml::to_string(&s).unwrap();
        assert!(yaml.contains("data_editor_items_sort: tags"));
        assert!(yaml.contains("rare"));
        let back: UserSettings = serde_yaml::from_str(&yaml).unwrap();
        assert_eq!(back.data_editor_items_sort, CatalogItemSort::Tags);
        assert_eq!(
            back.data_editor_items_tag_priority,
            vec!["rare".to_string(), "heal".to_string()]
        );

        let defaults = UserSettings::default();
        let default_yaml = serde_yaml::to_string(&defaults).unwrap();
        assert!(
            !default_yaml.contains("data_editor_items_sort"),
            "default NameAsc sort should omit from yaml"
        );
        assert!(
            !default_yaml.contains("data_editor_items_tag_priority"),
            "empty tag priority should omit from yaml"
        );
    }

    #[test]
    fn new_customization_fields_roundtrip() {
        let s = UserSettings {
            diagnostic_logging: true,
            memory_logging: true,
            memory_logging_interval_ms: 2_000,
            restore_last_macro: false,
            close_to_tray: true,
            start_minimized: true,
            restore_cursor_on_end: true,
            click_flash_ms: 500,
            default_keyboard_delay_ms: 40,
            default_mouse_delay_ms: 60,
            panic_key: "f12".into(),
            hotkey_while_running: HotkeyWhileRunning::Restart,
            default_hotkey_trigger: "release".into(),
            record_wait_threshold_ms: 100,
            record_move_min_distance: 16,
            record_tap_max_ms: 300,
            record_snap_point_px: 40,
            overlay_default_size: 40.0,
            overlay_gate_default_interval_ms: 400,
            finish_sound_file: "/tmp/finish.mp3".into(),
            backup_dir: "/tmp/backups".into(),
            update_check_interval_hours: 12,
            update_include_prerelease: true,
            accent_color: "#336699".into(),
            mono_font_size: 15,
            ui_font_family: UiFontFamily::Monospace,
            window_decorations: false,
            window_width: 1100.0,
            window_height: 750.0,
            window_pos_x: Some(10.0),
            window_pos_y: Some(20.0),
            window_maximized: true,
            ..Default::default()
        };
        let back: UserSettings = serde_yaml::from_str(&serde_yaml::to_string(&s).unwrap()).unwrap();
        assert!(back.diagnostic_logging);
        assert!(!back.window_decorations);
        assert!(back.suspend_screen_capture_when_idle);
        assert!(back.memory_logging);
        assert_eq!(back.memory_logging_interval_ms, 2_000);
        assert!(!back.restore_last_macro);
        assert!(back.close_to_tray);
        assert!(back.start_minimized);
        assert!(back.restore_cursor_on_end);
        assert_eq!(back.click_flash_ms, 500);
        assert_eq!(back.default_keyboard_delay_ms, 40);
        assert_eq!(back.default_mouse_delay_ms, 60);
        assert_eq!(back.panic_key, "f12");
        assert_eq!(back.hotkey_while_running, HotkeyWhileRunning::Restart);
        assert_eq!(back.default_hotkey_trigger, "release");
        assert_eq!(back.record_wait_threshold_ms, 100);
        assert_eq!(back.record_move_min_distance, 16);
        assert_eq!(back.record_tap_max_ms, 300);
        assert_eq!(back.record_snap_point_px, 40);
        assert!((back.overlay_default_size - 40.0).abs() < f32::EPSILON);
        assert_eq!(back.overlay_gate_default_interval_ms, 400);
        assert_eq!(back.finish_sound_file, "/tmp/finish.mp3");
        assert_eq!(back.backup_dir, "/tmp/backups");
        assert_eq!(back.update_check_interval_hours, 12);
        assert!(back.update_include_prerelease);
        assert_eq!(back.accent_color, "#336699");
        assert_eq!(back.mono_font_size, 15);
        assert_eq!(back.ui_font_family, UiFontFamily::Monospace);
        assert!((back.window_width - 1100.0).abs() < f32::EPSILON);
        assert_eq!(back.window_pos_x, Some(10.0));
        assert!(back.window_maximized);
    }

    #[test]
    fn window_decorations_defaults_on_and_roundtrips_off() {
        let missing: UserSettings = serde_yaml::from_str("{}\n").unwrap();
        assert!(missing.window_decorations);
        let off = UserSettings {
            window_decorations: false,
            ..Default::default()
        };
        let back: UserSettings =
            serde_yaml::from_str(&serde_yaml::to_string(&off).unwrap()).unwrap();
        assert!(!back.window_decorations);
    }

    #[test]
    fn suspend_screen_capture_defaults_on_and_roundtrips_off() {
        let missing: UserSettings = serde_yaml::from_str("{}\n").unwrap();
        assert!(missing.suspend_screen_capture_when_idle);
        let off = UserSettings {
            suspend_screen_capture_when_idle: false,
            ..Default::default()
        };
        let back: UserSettings =
            serde_yaml::from_str(&serde_yaml::to_string(&off).unwrap()).unwrap();
        assert!(!back.suspend_screen_capture_when_idle);
    }

    #[test]
    fn clamp_normalizes_new_fields() {
        let mut s = UserSettings {
            panic_key: "  F12  ".into(),
            default_hotkey_trigger: "bogus".into(),
            accent_color: "not-a-color".into(),
            record_wait_threshold_ms: 1,
            update_check_interval_hours: -5,
            ..Default::default()
        };
        s.clamp();
        assert_eq!(s.panic_key, "f12");
        assert_eq!(s.default_hotkey_trigger, "press");
        assert!(s.accent_color.is_empty());
        assert_eq!(s.record_wait_threshold_ms, DEFAULT_RECORD_WAIT_THRESHOLD_MS);
        assert_eq!(s.update_check_interval_hours, 0);
    }

    #[test]
    fn hotkey_while_running_and_font_family_have_labels() {
        assert_eq!(HotkeyWhileRunning::default(), HotkeyWhileRunning::Ignore);
        for p in HotkeyWhileRunning::ALL {
            assert!(!p.label().is_empty());
        }
        assert_eq!(UiFontFamily::default(), UiFontFamily::Standard);
        for f in UiFontFamily::ALL {
            assert!(!f.label().is_empty());
        }
    }

    #[test]
    fn ui_density_defaults_normal_and_maps_to_gap_tiers() {
        assert_eq!(UiDensity::default(), UiDensity::Normal);
        let mut value = serde_yaml::to_value(UserSettings::default()).unwrap();
        value
            .as_mapping_mut()
            .unwrap()
            .remove(serde_yaml::Value::String("ui_density".into()));
        let settings: UserSettings = serde_yaml::from_value(value).unwrap();
        assert_eq!(settings.ui_density, UiDensity::Normal);
        // Section gaps scale 4/8/12; element gaps 2/4/8.
        assert_eq!(UiDensity::Compact.section_gap(), 4.0);
        assert_eq!(UiDensity::Normal.section_gap(), 8.0);
        assert_eq!(UiDensity::Spaced.section_gap(), 12.0);
        assert_eq!(UiDensity::Compact.element_gap(), 2.0);
        assert_eq!(UiDensity::Normal.element_gap(), 4.0);
        assert_eq!(UiDensity::Spaced.element_gap(), 8.0);
        assert_eq!(
            UiDensity::ALL.map(|d| d.label()),
            ["Compact", "Normal", "Spaced"]
        );
    }

    #[test]
    fn overlay_press_stops_run_defaults_on_when_absent() {
        assert!(UserSettings::default().overlay_press_stops_run);
        let mut value = serde_yaml::to_value(UserSettings::default()).unwrap();
        value
            .as_mapping_mut()
            .unwrap()
            .remove(serde_yaml::Value::String("overlay_press_stops_run".into()));
        let settings: UserSettings = serde_yaml::from_value(value).unwrap();
        assert!(settings.overlay_press_stops_run);

        let off = UserSettings {
            overlay_press_stops_run: false,
            ..Default::default()
        };
        let back: UserSettings =
            serde_yaml::from_str(&serde_yaml::to_string(&off).unwrap()).unwrap();
        assert!(!back.overlay_press_stops_run);
    }

    #[test]
    fn overlay_buttons_in_detection_defaults_on_when_absent() {
        assert!(UserSettings::default().overlay_buttons_in_detection);
        let mut value = serde_yaml::to_value(UserSettings::default()).unwrap();
        value
            .as_mapping_mut()
            .unwrap()
            .remove(serde_yaml::Value::String(
                "overlay_buttons_in_detection".into(),
            ));
        let settings: UserSettings = serde_yaml::from_value(value).unwrap();
        assert!(settings.overlay_buttons_in_detection);

        let off = UserSettings {
            overlay_buttons_in_detection: false,
            ..Default::default()
        };
        let back: UserSettings =
            serde_yaml::from_str(&serde_yaml::to_string(&off).unwrap()).unwrap();
        assert!(!back.overlay_buttons_in_detection);
    }

    #[test]
    fn overlay_buttons_over_sqyre_defaults_off_when_absent() {
        assert!(!UserSettings::default().overlay_buttons_over_sqyre);
        let mut value = serde_yaml::to_value(UserSettings::default()).unwrap();
        value
            .as_mapping_mut()
            .unwrap()
            .remove(serde_yaml::Value::String(
                "overlay_buttons_over_sqyre".into(),
            ));
        let settings: UserSettings = serde_yaml::from_value(value).unwrap();
        assert!(!settings.overlay_buttons_over_sqyre);

        let on = UserSettings {
            overlay_buttons_over_sqyre: true,
            ..Default::default()
        };
        let back: UserSettings =
            serde_yaml::from_str(&serde_yaml::to_string(&on).unwrap()).unwrap();
        assert!(back.overlay_buttons_over_sqyre);
    }

    #[test]
    fn click_effect_defaults_off_when_absent() {
        assert!(!UserSettings::default().click_effect);
        let mut value = serde_yaml::to_value(UserSettings::default()).unwrap();
        value
            .as_mapping_mut()
            .unwrap()
            .remove(serde_yaml::Value::String("click_effect".into()));
        let settings: UserSettings = serde_yaml::from_value(value).unwrap();
        assert!(!settings.click_effect);

        let on = UserSettings {
            click_effect: true,
            ..Default::default()
        };
        let back: UserSettings =
            serde_yaml::from_str(&serde_yaml::to_string(&on).unwrap()).unwrap();
        assert!(back.click_effect);
    }

    #[test]
    fn clamp_action_log_max_entries() {
        let mut s = UserSettings {
            action_log_max_entries: 0,
            ..Default::default()
        };
        s.clamp();
        assert_eq!(s.action_log_max_entries, DEFAULT_ACTION_LOG_MAX_ENTRIES);
        s.action_log_max_entries = 5;
        s.clamp();
        assert_eq!(s.action_log_max_entries, MIN_ACTION_LOG_MAX_ENTRIES);
        s.action_log_max_entries = 999_999;
        s.clamp();
        assert_eq!(s.action_log_max_entries, MAX_ACTION_LOG_MAX_ENTRIES);
        assert_eq!(
            usize::try_from(MAX_ACTION_LOG_MAX_ENTRIES).unwrap(),
            sqyre_ports::MAX_ENTRIES_PER_ACTION
        );
    }

    #[test]
    fn action_log_max_entries_defaults_when_absent() {
        let mut value = serde_yaml::to_value(UserSettings::default()).unwrap();
        value
            .as_mapping_mut()
            .unwrap()
            .remove(serde_yaml::Value::String("action_log_max_entries".into()));
        let settings: UserSettings = serde_yaml::from_value(value).unwrap();
        assert_eq!(
            settings.action_log_max_entries,
            DEFAULT_ACTION_LOG_MAX_ENTRIES
        );
    }

    #[test]
    fn clamp_data_editor_left_frac() {
        let mut s = UserSettings {
            data_editor_left_split: 0.05,
            ..Default::default()
        };
        s.clamp();
        assert!((s.data_editor_left_split - MIN_DATA_EDITOR_LEFT_FRAC).abs() < f32::EPSILON);
        s.data_editor_left_split = 0.9;
        s.clamp();
        assert!((s.data_editor_left_split - MAX_DATA_EDITOR_LEFT_FRAC).abs() < f32::EPSILON);
    }

    #[test]
    fn clamp_macro_list_frac() {
        let mut s = UserSettings {
            macro_list_split: 0.05,
            ..Default::default()
        };
        s.clamp();
        assert!((s.macro_list_split - MIN_MACRO_LIST_FRAC).abs() < f32::EPSILON);
        s.macro_list_split = 0.9;
        s.clamp();
        assert!((s.macro_list_split - MAX_MACRO_LIST_FRAC).abs() < f32::EPSILON);
    }

    #[test]
    fn default_worker_threads_caps_at_sixteen() {
        let d = default_worker_threads();
        assert!(d >= MIN_WORKER_THREADS);
        assert!(d <= DEFAULT_WORKER_THREADS_CAP);
        assert!(d <= available_worker_threads());
        let mut s = UserSettings::default();
        assert_eq!(s.worker_threads, d);
        s.worker_threads = 0;
        s.clamp();
        assert_eq!(s.worker_threads, d);
        s.worker_threads = available_worker_threads().saturating_add(100);
        s.clamp();
        assert_eq!(s.worker_threads, available_worker_threads());
    }

    #[test]
    fn rename_overlay_macro_updates_matching_buttons() {
        let mut s = UserSettings::default();
        let mut a = OverlayButtonConfig::new("a", "P");
        a.macro_name = "old".into();
        let mut b = OverlayButtonConfig::new("b", "P");
        b.macro_name = "other".into();
        s.overlay_buttons.push(a);
        s.overlay_buttons.push(b);
        assert!(s.rename_overlay_macro("old", "new"));
        assert_eq!(s.overlay_buttons[0].macro_name, "new");
        assert_eq!(s.overlay_buttons[1].macro_name, "other");
        assert!(!s.rename_overlay_macro("old", "new"));
        assert!(!s.rename_overlay_macro("", "x"));
        assert!(!s.rename_overlay_macro("new", "new"));
    }

    #[test]
    fn remove_overlay_buttons_for_program_drops_matching() {
        let mut s = UserSettings::default();
        s.overlay_buttons
            .push(OverlayButtonConfig::new("keep", "Other"));
        s.overlay_buttons
            .push(OverlayButtonConfig::new("drop-a", "Gone"));
        s.overlay_buttons
            .push(OverlayButtonConfig::new("drop-b", "Gone"));
        s.overlay_buttons.push(default_quick_access_hub_button());
        assert!(s.remove_overlay_buttons_for_program("Gone"));
        assert_eq!(s.overlay_buttons.len(), 2);
        assert!(s.overlay_buttons.iter().any(|b| b.id == "keep"));
        assert!(s
            .overlay_buttons
            .iter()
            .any(|b| is_quick_access_hub_id(&b.id)));
        assert!(!s.remove_overlay_buttons_for_program("Gone"));
        assert!(!s.remove_overlay_buttons_for_program("Missing"));
        // Hub stays even when clearing General program buttons.
        assert!(!s.remove_overlay_buttons_for_program(sqyre_domain::GENERAL_PROGRAM));
        assert!(s
            .overlay_buttons
            .iter()
            .any(|b| is_quick_access_hub_id(&b.id)));

        let dir = tempdir().unwrap();
        let path = dir.path().join(crate::SETTINGS_FILE);
        s.save_to_path(&path).unwrap();
        let loaded = UserSettings::load_from_path(&path).unwrap();
        assert!(loaded.overlay_buttons.iter().any(|b| b.id == "keep"));
        assert!(loaded
            .overlay_buttons
            .iter()
            .any(|b| is_quick_access_hub_id(&b.id)));
        assert_eq!(
            loaded
                .overlay_buttons
                .iter()
                .find(|b| b.id == "keep")
                .map(|b| b.program.as_str()),
            Some("Other")
        );
    }

    #[test]
    fn rename_hotkey_tag_program_follows_name_and_nested_paths() {
        let mut s = UserSettings {
            hotkey_tag_filters: vec!["Old".into(), "Old/raid".into(), "keep".into()],
            ..UserSettings::default()
        };
        assert!(s.rename_hotkey_tag_program("Old", "New"));
        assert_eq!(
            s.hotkey_tag_filters,
            vec![
                "New".to_string(),
                "New/raid".to_string(),
                "keep".to_string()
            ]
        );
        assert!(!s.rename_hotkey_tag_program("New", "New"));
    }

    #[test]
    fn hotkey_tag_filters_omitted_defaults_empty() {
        let dir = tempdir().unwrap();
        let path = dir.path().join(crate::SETTINGS_FILE);
        std::fs::write(&path, "save_meta_images: true\n").unwrap();
        let loaded = UserSettings::load_from_path(&path).unwrap();
        assert!(loaded.hotkey_tag_filters.is_empty());
        assert!(!loaded.hotkey_tags_while_focused);
        assert!(loaded.image_search_variant_exit_early);
    }

    #[test]
    fn overlay_style_defaults_when_omitted() {
        let dir = tempdir().unwrap();
        let path = dir.path().join(crate::SETTINGS_FILE);
        std::fs::write(
            &path,
            r#"
overlay_buttons:
  - id: btn-legacy
    program: P
    macro_name: m
    x: 1.0
    y: 2.0
    size: 40.0
"#,
        )
        .unwrap();
        let loaded = UserSettings::load_from_path(&path).unwrap();
        let btn = &loaded.overlay_buttons[0];
        assert!(btn.enabled);
        assert!((btn.corner_radius - DEFAULT_OVERLAY_CORNER_RADIUS).abs() < f32::EPSILON);
        assert!((btn.border_width - DEFAULT_OVERLAY_BORDER_WIDTH).abs() < f32::EPSILON);
        assert!(btn.border_color.is_empty());
        assert_eq!(btn.border_alpha, 255);
        assert_eq!(btn.bg_alpha, 0);
        assert_eq!(btn.icon_alpha, 255);
        assert_eq!(
            btn.border_rgba(),
            OverlayButtonConfig::resolve_rgba("", 255, DEFAULT_OVERLAY_ACCENT_HEX)
        );
    }

    #[test]
    fn settings_path_is_under_sqyre_dir() {
        let dir = tempdir().unwrap();
        crate::with_sqyre_dir_override(dir.path().to_path_buf(), || {
            assert_eq!(settings_path(), dir.path().join(crate::SETTINGS_FILE));
        });
    }

    #[test]
    fn missing_file_uses_defaults() {
        let dir = tempdir().unwrap();
        let s = UserSettings::load_from_path(dir.path().join("nope.yaml")).unwrap();
        assert_eq!(s, UserSettings::default());
    }

    #[test]
    fn move_dir_relocates_tree() {
        let root = tempdir().unwrap();
        let src = root.path().join("old");
        let dst = root.path().join("new");
        fs::create_dir_all(src.join("images")).unwrap();
        fs::write(src.join(crate::DB_FILE), "macros: {}\n").unwrap();
        move_dir(&src, &dst).unwrap();
        assert!(!src.exists());
        assert!(dst.join(crate::DB_FILE).exists());
        assert!(dst.join("images").is_dir());
    }
}
