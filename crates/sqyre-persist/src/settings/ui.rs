//! UI layout, chrome, fonts, and appearance preferences.

use serde::{Deserialize, Serialize};
use sqyre_domain::{
    CatalogItemSort, ACTION_COLOR_KEY_CONTROL_FLOW, ACTION_COLOR_KEY_DEFAULT,
    ACTION_COLOR_KEY_DETECTION, ACTION_COLOR_KEY_MISCELLANEOUS, ACTION_COLOR_KEY_MOUSE_KEYBOARD,
    ACTION_COLOR_KEY_VARIABLES, ACTION_COLOR_KEY_WAIT,
};

pub const DEFAULT_HIDE_APP_DURING_RECORDING: bool = true;
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct UiLayout {
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

    /// Data Editor left list width as a fraction of the editor body (`0.15`–`0.75`).
    #[serde(default = "default_data_editor_left_frac")]
    pub data_editor_left_split: f32,

    /// Data Editor Items / Pixel Check left-list sort.
    #[serde(default, skip_serializing_if = "is_default_catalog_item_sort")]
    pub data_editor_items_sort: CatalogItemSort,

    /// Tag priority when [`UiLayout::data_editor_items_sort`] is [`CatalogItemSort::Tags`].
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub data_editor_items_tag_priority: Vec<String>,

    /// Macro list side panel width as a fraction of the window (`0.15`–`0.75`).
    #[serde(default = "default_macro_list_frac")]
    pub macro_list_split: f32,

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
fn default_compact_program_headers() -> bool {
    DEFAULT_COMPACT_PROGRAM_HEADERS
}
fn default_window_border() -> bool {
    DEFAULT_WINDOW_BORDER
}
fn default_window_decorations() -> bool {
    DEFAULT_WINDOW_DECORATIONS
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

fn is_zero_f32(v: &f32) -> bool {
    *v == 0.0
}

impl Default for UiLayout {
    fn default() -> Self {
        Self {
            preview_desktop_outline: false,
            compact_program_headers: DEFAULT_COMPACT_PROGRAM_HEADERS,
            show_empty_program_headers: false,
            window_border: DEFAULT_WINDOW_BORDER,
            window_decorations: DEFAULT_WINDOW_DECORATIONS,
            run_button_glow: DEFAULT_RUN_BUTTON_GLOW,
            hide_app_during_recording: DEFAULT_HIDE_APP_DURING_RECORDING,
            ui_font_size: DEFAULT_UI_FONT_SIZE,
            ui_scale: DEFAULT_UI_SCALE,
            ui_density: UiDensity::default(),
            auto_compact_density: DEFAULT_AUTO_COMPACT_DENSITY,
            action_colors: ActionColorPrefs::default(),
            data_editor_left_split: DEFAULT_DATA_EDITOR_LEFT_FRAC,
            data_editor_items_sort: CatalogItemSort::default(),
            data_editor_items_tag_priority: Vec::new(),
            macro_list_split: DEFAULT_MACRO_LIST_FRAC,
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

impl UiLayout {
    /// Clamp numeric ranges to settings UI bounds.
    pub fn clamp(&mut self) {
        self.ui_font_size = self.ui_font_size.clamp(10, 28);
        if self.ui_scale <= 0.0 {
            self.ui_scale = DEFAULT_UI_SCALE;
        }
        self.ui_scale = ((self.ui_scale * 10.0).round() / 10.0).clamp(MIN_UI_SCALE, MAX_UI_SCALE);
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
        if !self.accent_color.trim().is_empty()
            && sqyre_domain::parse_hex_color(self.accent_color.trim()).is_none()
        {
            self.accent_color.clear();
        }
        self.mono_font_size = self.mono_font_size.clamp(0, 40);
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
    }
}
