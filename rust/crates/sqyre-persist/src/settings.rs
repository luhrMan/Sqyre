//! User settings persisted outside the data directory (Go Fyne Preferences).
//!
//! Lives under the XDG config dir (`~/.config/sqyre/settings.yaml`) so the
//! `sqyre_dir` override can relocate `~/.sqyre` without losing preferences.

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

use crate::{PersistError, Result};

pub const DEFAULT_IMAGE_SEARCH_CLOSE_MATCHES_DISTANCE: i32 = 10;
pub const DEFAULT_DRAG_PREVIEW_DEBOUNCE_MS: i32 = 150;
pub const MIN_DRAG_PREVIEW_DEBOUNCE_MS: i32 = 25;
pub const DEFAULT_HIDE_APP_DURING_RECORDING: bool = true;
pub const DEFAULT_UI_FONT_SIZE: i32 = 14;
pub const DEFAULT_UI_SCALE: f32 = 1.0;

pub const ACTION_COLOR_MOUSE_KEYBOARD: &str = "mouse_keyboard";
pub const ACTION_COLOR_DETECTION: &str = "detection";
pub const ACTION_COLOR_VARIABLES: &str = "variables";
pub const ACTION_COLOR_MISCELLANEOUS: &str = "miscellaneous";
pub const ACTION_COLOR_WAIT: &str = "wait";
pub const ACTION_COLOR_DEFAULT: &str = "default";

/// Absolute path to the settings file (`~/.config/sqyre/settings.yaml`).
pub fn settings_path() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| {
            dirs::home_dir()
                .unwrap_or_else(std::env::temp_dir)
                .join(".config")
        })
        .join("sqyre")
        .join("settings.yaml")
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
            ACTION_COLOR_MOUSE_KEYBOARD => &self.mouse_keyboard,
            ACTION_COLOR_DETECTION => &self.detection,
            ACTION_COLOR_VARIABLES => &self.variables,
            ACTION_COLOR_MISCELLANEOUS => &self.miscellaneous,
            ACTION_COLOR_WAIT => &self.wait,
            ACTION_COLOR_DEFAULT => &self.default,
            _ => "",
        }
    }

    pub fn set(&mut self, key: &str, hex: String) {
        match key {
            ACTION_COLOR_MOUSE_KEYBOARD => self.mouse_keyboard = hex,
            ACTION_COLOR_DETECTION => self.detection = hex,
            ACTION_COLOR_VARIABLES => self.variables = hex,
            ACTION_COLOR_MISCELLANEOUS => self.miscellaneous = hex,
            ACTION_COLOR_WAIT => self.wait = hex,
            ACTION_COLOR_DEFAULT => self.default = hex,
            _ => {}
        }
    }

    pub fn clear_all(&mut self) {
        *self = Self::default();
    }
}

/// User-tunable Sqyre preferences (parity with Go User Settings).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct UserSettings {
    #[serde(default)]
    pub save_meta_images: bool,
    #[serde(default)]
    pub highlight_active_action: bool,
    #[serde(default = "default_hide_recording")]
    pub hide_app_during_recording: bool,
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
}

fn default_hide_recording() -> bool {
    DEFAULT_HIDE_APP_DURING_RECORDING
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

impl Default for UserSettings {
    fn default() -> Self {
        Self {
            save_meta_images: false,
            highlight_active_action: false,
            hide_app_during_recording: DEFAULT_HIDE_APP_DURING_RECORDING,
            image_search_close_matches_distance: DEFAULT_IMAGE_SEARCH_CLOSE_MATCHES_DISTANCE,
            drag_preview_debounce_ms: DEFAULT_DRAG_PREVIEW_DEBOUNCE_MS,
            sqyre_dir: String::new(),
            ui_font_size: DEFAULT_UI_FONT_SIZE,
            ui_scale: DEFAULT_UI_SCALE,
            action_colors: ActionColorPrefs::default(),
        }
    }
}

impl UserSettings {
    pub fn load_default() -> Result<Self> {
        Self::load_from_path(settings_path())
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
        self.save_to_path(settings_path())
    }

    pub fn save_to_path(&self, path: impl AsRef<Path>) -> Result<()> {
        let path = path.as_ref();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let mut clamped = self.clone();
        clamped.clamp();
        fs::write(path, serde_yaml::to_string(&clamped)?)?;
        Ok(())
    }

    /// Clamp numeric ranges to the same bounds as the Go settings UI.
    pub fn clamp(&mut self) {
        self.image_search_close_matches_distance =
            self.image_search_close_matches_distance.clamp(0, 100);
        if self.drag_preview_debounce_ms < MIN_DRAG_PREVIEW_DEBOUNCE_MS {
            self.drag_preview_debounce_ms = DEFAULT_DRAG_PREVIEW_DEBOUNCE_MS;
        }
        self.drag_preview_debounce_ms = self.drag_preview_debounce_ms.clamp(MIN_DRAG_PREVIEW_DEBOUNCE_MS, 1000);
        self.ui_font_size = self.ui_font_size.clamp(10, 28);
        if self.ui_scale <= 0.0 {
            self.ui_scale = DEFAULT_UI_SCALE;
        }
        self.ui_scale = ((self.ui_scale * 10.0).round() / 10.0).clamp(0.5, 2.5);
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
        let mut s = UserSettings::default();
        s.save_meta_images = true;
        s.highlight_active_action = true;
        s.image_search_close_matches_distance = 25;
        s.ui_scale = 1.2;
        s.action_colors.detection = "#aabbcc".into();
        s.save_to_path(&path).unwrap();

        let loaded = UserSettings::load_from_path(&path).unwrap();
        assert!(loaded.save_meta_images);
        assert!(loaded.highlight_active_action);
        assert_eq!(loaded.image_search_close_matches_distance, 25);
        assert!((loaded.ui_scale - 1.2).abs() < f32::EPSILON);
        assert_eq!(loaded.action_colors.detection, "#aabbcc");
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
