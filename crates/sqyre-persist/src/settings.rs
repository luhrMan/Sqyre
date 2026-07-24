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
    ACTION_COLOR_KEY_DEFAULT, ACTION_COLOR_KEY_DETECTION, ACTION_COLOR_KEY_MISCELLANEOUS,
    ACTION_COLOR_KEY_MOUSE_KEYBOARD, ACTION_COLOR_KEY_VARIABLES, ACTION_COLOR_KEY_WAIT,
};

pub const DEFAULT_IMAGE_SEARCH_CLOSE_MATCHES_DISTANCE: i32 = 10;
pub const DEFAULT_DRAG_PREVIEW_DEBOUNCE_MS: i32 = 150;
pub const MIN_DRAG_PREVIEW_DEBOUNCE_MS: i32 = 25;
pub const DEFAULT_HIDE_APP_DURING_RECORDING: bool = true;
pub const DEFAULT_PLAY_FINISH_SOUND: bool = true;
pub const DEFAULT_PLAY_UI_SOUNDS: bool = true;
pub const DEFAULT_SOUND_VOLUME: f32 = 0.25;
pub const DEFAULT_UI_FONT_SIZE: i32 = 14;
pub const DEFAULT_UI_SCALE: f32 = 1.7;
pub const DEFAULT_BACKUP_INTERVAL_HOURS: i32 = 24;
pub const MIN_BACKUP_INTERVAL_HOURS: i32 = 1;
pub const MAX_BACKUP_INTERVAL_HOURS: i32 = 720;
pub const DEFAULT_BACKUP_MAX_KEEP: i32 = 10;
pub const MIN_BACKUP_MAX_KEEP: i32 = 1;
pub const MAX_BACKUP_MAX_KEEP: i32 = 100;
pub const DEFAULT_AUTO_UPDATE_CHECK: bool = true;

/// Absolute path to the settings file (`{sqyre_dir}/settings.yaml`).
pub fn settings_path() -> PathBuf {
    crate::sqyre_dir().join("settings.yaml")
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

/// Always-on-top screen button that starts a named macro.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct OverlayButtonConfig {
    /// Stable id used for the deferred viewport hash.
    pub id: String,
    /// Catalog program this button belongs to (shown when that program is focused).
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub program: String,
    /// Tooltip / optional caption under the icon.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub label: String,
    /// Macro name to start (must match an entry in `db.yaml`).
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub macro_name: String,
    /// Icon catalog id (Phosphor kebab-case, e.g. `play`, `lightning`). Empty = default play.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub icon: String,
    /// Desktop position of the button viewport (top-left, points).
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
}

pub const DEFAULT_OVERLAY_BUTTON_SIZE: f32 = 52.0;
pub const MIN_OVERLAY_BUTTON_SIZE: f32 = 12.0;
pub const MAX_OVERLAY_BUTTON_SIZE: f32 = 128.0;
pub const DEFAULT_OVERLAY_CORNER_RADIUS: f32 = 8.0;
pub const MIN_OVERLAY_CORNER_RADIUS: f32 = 0.0;
pub const MAX_OVERLAY_CORNER_RADIUS: f32 = 64.0;
pub const DEFAULT_OVERLAY_BORDER_WIDTH: f32 = 1.5;
pub const MIN_OVERLAY_BORDER_WIDTH: f32 = 0.0;
pub const MAX_OVERLAY_BORDER_WIDTH: f32 = 8.0;
/// Default border / hover icon when `border_color` / `icon_hover_color` is empty (`#dc9d2e`).
pub const DEFAULT_OVERLAY_ACCENT_HEX: &str = "#dc9d2e";
/// Default idle icon when `icon_color` is empty (`#f5e6c0`).
pub const DEFAULT_OVERLAY_ICON_HEX: &str = "#f5e6c0";

fn default_overlay_button_size() -> f32 {
    DEFAULT_OVERLAY_BUTTON_SIZE
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
        Self {
            id: id.into(),
            program: program.into(),
            label: String::new(),
            macro_name: String::new(),
            icon: String::new(),
            x: 48.0,
            y: 48.0,
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
        }
    }

    /// Display name for lists (label, else macro, else id).
    pub fn display_name(&self) -> &str {
        let label = self.label.trim();
        if !label.is_empty() {
            return label;
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

/// User-tunable Sqyre preferences.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct UserSettings {
    #[serde(default)]
    pub save_meta_images: bool,
    #[serde(default)]
    pub highlight_active_action: bool,
    #[serde(default = "default_hide_recording")]
    pub hide_app_during_recording: bool,
    /// Play an audible cue when a top-level macro run finishes successfully.
    #[serde(default = "default_play_finish_sound")]
    pub play_finish_sound: bool,
    /// Play audible cues when the user adds or deletes macros, actions, or catalog entities.
    #[serde(default = "default_play_ui_sounds")]
    pub play_ui_sounds: bool,
    /// Playback volume for app cue sounds (`0.0`–`1.0`).
    #[serde(default = "default_sound_volume")]
    pub sound_volume: f32,
    #[serde(default = "default_close_matches")]
    pub image_search_close_matches_distance: i32,
    #[serde(default = "default_drag_debounce")]
    pub drag_preview_debounce_ms: i32,
    /// Absolute path override for the `.sqyre` data directory (empty = `~/.sqyre`).
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub sqyre_dir: String,
    #[serde(default = "default_font_size")]
    pub ui_font_size: i32,
    #[serde(default = "default_ui_scale")]
    pub ui_scale: f32,
    #[serde(default)]
    pub action_colors: ActionColorPrefs,
    /// Per-action-type blank templates for the Add Action picker (YAML action maps).
    #[serde(default, skip_serializing_if = "std::collections::BTreeMap::is_empty")]
    pub action_defaults: std::collections::BTreeMap<String, serde_yaml::Mapping>,
    /// Show floating always-on-top buttons that start macros.
    #[serde(default)]
    pub overlay_enabled: bool,
    /// User-configured overlay buttons (per-program; shown when that program is focused).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub overlay_buttons: Vec<OverlayButtonConfig>,
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
}

fn default_hide_recording() -> bool {
    DEFAULT_HIDE_APP_DURING_RECORDING
}
fn default_play_finish_sound() -> bool {
    DEFAULT_PLAY_FINISH_SOUND
}
fn default_play_ui_sounds() -> bool {
    DEFAULT_PLAY_UI_SOUNDS
}
fn default_sound_volume() -> f32 {
    DEFAULT_SOUND_VOLUME
}
fn default_close_matches() -> i32 {
    DEFAULT_IMAGE_SEARCH_CLOSE_MATCHES_DISTANCE
}
fn default_drag_debounce() -> i32 {
    DEFAULT_DRAG_PREVIEW_DEBOUNCE_MS
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
fn is_zero_i64(v: &i64) -> bool {
    *v == 0
}

impl Default for UserSettings {
    fn default() -> Self {
        Self {
            save_meta_images: false,
            highlight_active_action: false,
            hide_app_during_recording: DEFAULT_HIDE_APP_DURING_RECORDING,
            play_finish_sound: DEFAULT_PLAY_FINISH_SOUND,
            play_ui_sounds: DEFAULT_PLAY_UI_SOUNDS,
            sound_volume: DEFAULT_SOUND_VOLUME,
            image_search_close_matches_distance: DEFAULT_IMAGE_SEARCH_CLOSE_MATCHES_DISTANCE,
            drag_preview_debounce_ms: DEFAULT_DRAG_PREVIEW_DEBOUNCE_MS,
            sqyre_dir: String::new(),
            ui_font_size: DEFAULT_UI_FONT_SIZE,
            ui_scale: DEFAULT_UI_SCALE,
            action_colors: ActionColorPrefs::default(),
            action_defaults: std::collections::BTreeMap::new(),
            overlay_enabled: false,
            overlay_buttons: Vec::new(),
            backup_enabled: false,
            backup_interval_hours: DEFAULT_BACKUP_INTERVAL_HOURS,
            backup_max_keep: DEFAULT_BACKUP_MAX_KEEP,
            last_backup_unix: 0,
            auto_update_check: DEFAULT_AUTO_UPDATE_CHECK,
            last_update_check_unix: 0,
        }
    }
}

impl UserSettings {
    pub fn load_default() -> Result<Self> {
        #[cfg(target_arch = "wasm32")]
        {
            return Ok(Self::default());
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
        let mut s: Self = serde_yaml::from_str(&text)?;
        s.clamp();
        Ok(s)
    }

    pub fn save_default(&self) -> Result<()> {
        #[cfg(target_arch = "wasm32")]
        {
            let _ = self;
            return Ok(());
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            // Keep runtime override + XDG pointer in sync before resolving settings_path().
            self.apply_sqyre_dir_override();
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

    /// Clamp numeric ranges to settings UI bounds.
    pub fn clamp(&mut self) {
        self.image_search_close_matches_distance =
            self.image_search_close_matches_distance.clamp(0, 100);
        if self.drag_preview_debounce_ms < MIN_DRAG_PREVIEW_DEBOUNCE_MS {
            self.drag_preview_debounce_ms = DEFAULT_DRAG_PREVIEW_DEBOUNCE_MS;
        }
        self.drag_preview_debounce_ms = self
            .drag_preview_debounce_ms
            .clamp(MIN_DRAG_PREVIEW_DEBOUNCE_MS, 1000);
        self.ui_font_size = self.ui_font_size.clamp(10, 28);
        if self.ui_scale <= 0.0 {
            self.ui_scale = DEFAULT_UI_SCALE;
        }
        self.ui_scale = ((self.ui_scale * 10.0).round() / 10.0).clamp(0.5, 2.5);
        if !self.sound_volume.is_finite() {
            self.sound_volume = DEFAULT_SOUND_VOLUME;
        }
        self.sound_volume = self.sound_volume.clamp(0.0, 1.0);
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
        let path = dir.path().join("settings.yaml");
        let mut s = UserSettings {
            save_meta_images: true,
            highlight_active_action: true,
            image_search_close_matches_distance: 25,
            ui_scale: 1.2,
            overlay_enabled: true,
            ..Default::default()
        };
        s.action_colors.detection = "#aabbcc".into();
        s.overlay_buttons.push(OverlayButtonConfig {
            id: "btn-1".into(),
            program: "Demo Game".into(),
            label: "Go".into(),
            macro_name: "demo".into(),
            icon: "bolt".into(),
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
        });
        s.save_to_path(&path).unwrap();

        let loaded = UserSettings::load_from_path(&path).unwrap();
        assert!(loaded.save_meta_images);
        assert!(loaded.highlight_active_action);
        assert_eq!(loaded.image_search_close_matches_distance, 25);
        assert!((loaded.ui_scale - 1.2).abs() < f32::EPSILON);
        assert_eq!(loaded.action_colors.detection, "#aabbcc");
        assert!(loaded.overlay_enabled);
        assert_eq!(loaded.overlay_buttons.len(), 1);
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
    }

    #[test]
    fn overlay_style_defaults_when_omitted() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("settings.yaml");
        std::fs::write(
            &path,
            r#"
overlay_enabled: true
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
            assert_eq!(settings_path(), dir.path().join("settings.yaml"));
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
        fs::write(src.join("db.yaml"), "macros: {}\n").unwrap();
        move_dir(&src, &dst).unwrap();
        assert!(!src.exists());
        assert!(dst.join("db.yaml").exists());
        assert!(dst.join("images").is_dir());
    }
}
