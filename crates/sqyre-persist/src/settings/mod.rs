//! User settings persisted inside the Sqyre data directory.
//!
//! Lives at `~/.sqyre/settings.yaml` (or under a relocated data dir). A small
//! pointer file at `~/.config/sqyre/data_dir` records a non-default data location
//! so the next launch can find settings after a relocate.

mod data;
mod overlay;
mod paths;
mod runtime;
mod sound;
mod ui;

pub use data::{
    DataLifecycle, DEFAULT_AUTO_UPDATE_CHECK, DEFAULT_BACKUP_INTERVAL_HOURS,
    DEFAULT_BACKUP_MAX_KEEP, DEFAULT_UPDATE_CHECK_INTERVAL_HOURS, MAX_BACKUP_INTERVAL_HOURS,
    MAX_BACKUP_MAX_KEEP, MAX_UPDATE_CHECK_INTERVAL_HOURS, MIN_BACKUP_INTERVAL_HOURS,
    MIN_BACKUP_MAX_KEEP, MIN_UPDATE_CHECK_INTERVAL_HOURS,
};
pub use overlay::{
    default_overlay_position, default_quick_access_hub_button, ensure_quick_access_hub_button,
    hub_position_on_monitor, is_quick_access_hub_id, snap_hub_button_to_monitor,
    snap_hub_to_monitor, snap_hub_xy_to_monitor, HubEdge, OverlayButtonConfig, OverlaySettings,
    OverlayVisibilityGate, OverlayVisibilityMode, DEFAULT_OVERLAY_ACCENT_HEX,
    DEFAULT_OVERLAY_BORDER_WIDTH, DEFAULT_OVERLAY_BUTTON_SIZE, DEFAULT_OVERLAY_CORNER_RADIUS,
    DEFAULT_OVERLAY_FALLBACK_SCREEN_H, DEFAULT_OVERLAY_FALLBACK_SCREEN_W,
    DEFAULT_OVERLAY_GATE_BLUR, DEFAULT_OVERLAY_GATE_INTERVAL_MS, DEFAULT_OVERLAY_GATE_TOLERANCE,
    DEFAULT_OVERLAY_ICON_HEX, DEFAULT_QUICK_ACCESS_OPACITY, HUB_MAX_EDGE_OVERHANG,
    MAX_OVERLAY_BORDER_WIDTH, MAX_OVERLAY_BUTTON_SIZE, MAX_OVERLAY_CORNER_RADIUS,
    MAX_OVERLAY_GATE_INTERVAL_MS, MIN_OVERLAY_BORDER_WIDTH, MIN_OVERLAY_BUTTON_SIZE,
    MIN_OVERLAY_CORNER_RADIUS, MIN_OVERLAY_GATE_INTERVAL_MS, QUICK_ACCESS_HUB_DISMISS,
    QUICK_ACCESS_HUB_ICON, QUICK_ACCESS_HUB_ID, QUICK_ACCESS_HUB_LABEL,
};
pub use paths::{move_dir, open_path_in_file_manager, open_sqyre_dir, settings_path};
pub use runtime::{
    available_worker_threads, default_worker_threads, HotkeyWhileRunning, RuntimePrefs,
    DEFAULT_ACTION_LOG_MAX_ENTRIES, DEFAULT_CLICK_FLASH_MS,
    DEFAULT_IMAGE_SEARCH_CLOSE_MATCHES_DISTANCE, DEFAULT_IMAGE_SEARCH_VARIANT_EXIT_EARLY,
    DEFAULT_MACRO_KEYBOARD_DELAY_MS, DEFAULT_MACRO_MOUSE_DELAY_MS, DEFAULT_PANIC_KEY,
    DEFAULT_RECORD_MOVE_MIN_DISTANCE, DEFAULT_RECORD_SNAP_POINT_PX, DEFAULT_RECORD_TAP_MAX_MS,
    DEFAULT_RECORD_WAIT_THRESHOLD_MS, DEFAULT_RELEASE_HELD_INPUTS_ON_END,
    DEFAULT_RUN_MACRO_MAX_DEPTH, DEFAULT_WHILE_MAX_ITERATIONS, DEFAULT_WORKER_THREADS_CAP,
    MAX_ACTION_LOG_MAX_ENTRIES, MAX_CLICK_FLASH_MS, MAX_LOG_IMAGE_MAX_EDGE,
    MAX_MEMORY_SAMPLE_INTERVAL_MS, MAX_RECORD_MOVE_MIN_DISTANCE, MAX_RECORD_SNAP_POINT_PX,
    MAX_RECORD_TAP_MAX_MS, MAX_RECORD_WAIT_THRESHOLD_MS, MAX_RUN_MACRO_MAX_DEPTH,
    MAX_WHILE_MAX_ITERATIONS, MIN_ACTION_LOG_MAX_ENTRIES, MIN_CLICK_FLASH_MS,
    MIN_LOG_IMAGE_MAX_EDGE, MIN_MEMORY_SAMPLE_INTERVAL_MS, MIN_RECORD_MOVE_MIN_DISTANCE,
    MIN_RECORD_SNAP_POINT_PX, MIN_RECORD_TAP_MAX_MS, MIN_RECORD_WAIT_THRESHOLD_MS,
    MIN_RUN_MACRO_MAX_DEPTH, MIN_WHILE_MAX_ITERATIONS, MIN_WORKER_THREADS,
};
pub use sound::{SoundSettings, DEFAULT_PLAY_FINISH_SOUND, DEFAULT_SOUND_VOLUME};
pub use ui::{
    ActionColorPrefs, UiDensity, UiFontFamily, UiLayout, DEFAULT_DATA_EDITOR_LEFT_FRAC,
    DEFAULT_HIDE_APP_DURING_RECORDING, DEFAULT_MACRO_LIST_FRAC, DEFAULT_UI_FONT_SIZE,
    DEFAULT_UI_SCALE, MAX_DATA_EDITOR_LEFT_FRAC, MAX_MACRO_LIST_FRAC, MAX_UI_SCALE,
    MIN_DATA_EDITOR_LEFT_FRAC, MIN_MACRO_LIST_FRAC, MIN_UI_SCALE,
};

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

use crate::{PersistError, Result};
#[cfg(not(target_arch = "wasm32"))]
use paths::{apply_data_dir_pointer, write_data_dir_pointer};
use sqyre_domain::{rename_coordinate_entity, rename_coordinate_program, CoordinateRef};

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct UserSettings {
    #[serde(default)]
    pub runtime: RuntimePrefs,
    #[serde(default)]
    pub ui: UiLayout,
    #[serde(default)]
    pub overlay: OverlaySettings,
    #[serde(default)]
    pub sound: SoundSettings,
    #[serde(default)]
    pub data: DataLifecycle,
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
            write_data_dir_pointer(&self.data.sqyre_dir)?;
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
        for btn in &mut self.overlay.overlay_buttons {
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
        for btn in &mut self.overlay.overlay_buttons {
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
        for btn in &mut self.overlay.overlay_buttons {
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
            &mut self.runtime.hotkey_tag_filters,
            old_program,
            new_program,
        )
    }

    /// Propagate a program rename into overlay button point / gate search-area refs.
    pub fn rename_overlay_point_program(&mut self, old_program: &str, new_program: &str) -> bool {
        let mut changed = false;
        for btn in &mut self.overlay.overlay_buttons {
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
        let before = self.overlay.overlay_buttons.len();
        self.overlay
            .overlay_buttons
            .retain(|btn| btn.program != program || is_quick_access_hub_id(&btn.id));
        self.overlay.overlay_buttons.len() != before
    }

    /// Clear overlay button point refs that target a deleted catalog point.
    pub fn clear_overlay_point_refs(&mut self, program: &str, name: &str) -> bool {
        let mut changed = false;
        for btn in &mut self.overlay.overlay_buttons {
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
        for btn in &mut self.overlay.overlay_buttons {
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
        self.runtime.clamp();
        self.ui.clamp();
        self.overlay.clamp();
        self.sound.clamp();
        self.data.clamp();
        let _ = ensure_quick_access_hub_button(self);
    }

    /// Apply `sqyre_dir` override to the process-wide data directory.
    pub fn apply_sqyre_dir_override(&self) {
        let path = self.data.sqyre_dir.trim();
        if path.is_empty() {
            crate::set_sqyre_dir_override(None);
        } else {
            crate::set_sqyre_dir_override(Some(PathBuf::from(path)));
        }
    }

    /// Apply `backup_dir` override to the process-wide backups directory.
    pub fn apply_backup_dir_override(&self) {
        let path = self.data.backup_dir.trim();
        if path.is_empty() {
            crate::set_backup_dir_override(None);
        } else {
            crate::set_backup_dir_override(Some(PathBuf::from(path)));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqyre_domain::CatalogItemSort;
    use tempfile::tempdir;

    #[test]
    fn roundtrip_settings() {
        let dir = tempdir().unwrap();
        let path = dir.path().join(crate::SETTINGS_FILE);
        let mut s = UserSettings {
            runtime: RuntimePrefs {
                save_meta_images: true,
                highlight_active_action: true,
                image_search_close_matches_distance: 25,
                image_search_variant_exit_early: false,
                hotkey_tag_filters: vec!["combat".into()],
                last_selected_macro: "farm loop".into(),
                permissions_intro_done: true,
                ..Default::default()
            },
            ui: UiLayout {
                show_empty_program_headers: true,
                ui_scale: 1.2,
                ui_density: UiDensity::Spaced,
                ..Default::default()
            },
            ..Default::default()
        };
        s.ui.action_colors.detection = "#aabbcc".into();
        s.overlay.overlay_buttons.push(OverlayButtonConfig {
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
        assert!(loaded.runtime.save_meta_images);
        assert!(loaded.runtime.highlight_active_action);
        assert_eq!(loaded.runtime.image_search_close_matches_distance, 25);
        assert!(!loaded.runtime.image_search_variant_exit_early);
        assert!((loaded.ui.ui_scale - 1.2).abs() < f32::EPSILON);
        assert_eq!(loaded.ui.action_colors.detection, "#aabbcc");
        assert!(
            loaded
                .overlay
                .overlay_buttons
                .iter()
                .any(|b| is_quick_access_hub_id(&b.id)),
            "load clamps and seeds the hub"
        );
        let btn = loaded
            .overlay
            .overlay_buttons
            .iter()
            .find(|b| b.id == "btn-1")
            .expect("demo button");
        assert!(btn.enabled);
        assert_eq!(btn.program, "Demo Game");
        assert_eq!(btn.macro_name, "demo");
        assert_eq!(btn.icon, "bolt");
        assert!((btn.size - 64.0).abs() < f32::EPSILON);
        assert!((btn.corner_radius - 12.0).abs() < f32::EPSILON);
        assert_eq!(btn.border_color, "#ff0000");
        assert_eq!(btn.border_alpha, 200);
        assert_eq!(btn.bg_color, "#112233");
        assert_eq!(btn.bg_alpha, 128);
        assert_eq!(btn.icon_color, "#abcdef");
        assert_eq!(btn.icon_hover_color, "#fedcba");
        assert_eq!(
            loaded.runtime.hotkey_tag_filters,
            vec!["combat".to_string()]
        );
        assert_eq!(loaded.ui.ui_density, UiDensity::Spaced);
        assert_eq!(loaded.runtime.last_selected_macro, "farm loop");
        assert!(loaded.runtime.permissions_intro_done);
        assert!(loaded.overlay.overlay_press_stops_run);
        assert!(loaded.overlay.overlay_buttons_in_detection);
        assert!(!loaded.overlay.overlay_buttons_over_sqyre);
        assert!(!loaded.runtime.hotkey_tags_while_focused);
        assert!(
            (loaded.ui.data_editor_left_split - DEFAULT_DATA_EDITOR_LEFT_FRAC).abs() < f32::EPSILON
        );
    }

    #[test]
    fn default_settings_do_not_seed_hub_until_clamp() {
        assert!(
            UserSettings::default().overlay.overlay_buttons.is_empty(),
            "defaults do not seed the hub until clamp"
        );
    }

    #[test]
    fn ensure_quick_access_hub_button_seeds_and_repairs() {
        let mut s = UserSettings::default();
        assert!(ensure_quick_access_hub_button(&mut s));
        assert_eq!(s.overlay.overlay_buttons.len(), 1);
        let hub = &s.overlay.overlay_buttons[0];
        assert_eq!(hub.id, QUICK_ACCESS_HUB_ID);
        assert_eq!(hub.program, sqyre_domain::GENERAL_PROGRAM);
        assert_eq!(hub.macro_name, QUICK_ACCESS_HUB_ID);
        assert_eq!(hub.icon, QUICK_ACCESS_HUB_ICON);
        assert_eq!(hub.display_name(), QUICK_ACCESS_HUB_LABEL);
        assert_eq!(hub.border_width, 0.0);
        assert_eq!(hub.bg_alpha, 0);
        assert_eq!(hub.icon_alpha, DEFAULT_QUICK_ACCESS_OPACITY);
        assert!(!ensure_quick_access_hub_button(&mut s));

        s.overlay.overlay_buttons[0].program = "Other".into();
        s.overlay.overlay_buttons[0].macro_name = "wrong".into();
        s.overlay.overlay_buttons[0].icon = "bolt".into();
        s.overlay.overlay_buttons[0].point = "General~Monitor 1 Center".into();
        s.overlay.overlay_buttons[0].border_width = 2.0;
        s.overlay.overlay_buttons[0].border_alpha = 200;
        s.overlay.overlay_buttons[0].bg_alpha = 128;
        assert!(ensure_quick_access_hub_button(&mut s));
        let hub = &s.overlay.overlay_buttons[0];
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
            .overlay
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

        // 50% of the face may hang past the right edge; 50% stays on-screen.
        let size = 40.0;
        let (edge, x, _) = snap_hub_xy_to_monitor(1900.0, 500.0, size, 0.0, 0.0, 1920.0, 1080.0);
        assert_eq!(edge, HubEdge::Right);
        let flush = 1920.0 - size;
        let max_out = size * HUB_MAX_EDGE_OVERHANG;
        assert!(x >= flush - 0.5 && x <= flush + max_out + 0.5, "x={x}");
        // Dragged past 50%: clamp to max overhang (only 50% visible).
        let (_, x_hang, _) = snap_hub_xy_to_monitor(1920.0, 500.0, size, 0.0, 0.0, 1920.0, 1080.0);
        assert!((x_hang - (flush + max_out)).abs() < 1.0, "x_hang={x_hang}");

        // Left edge: past left + past top; center still nearer Left than Top.
        let (edge, x_l, y_l) =
            snap_hub_xy_to_monitor(-100.0, -100.0, size, 0.0, 0.0, 1920.0, 1080.0);
        assert_eq!(edge, HubEdge::Left);
        assert!((x_l - (-max_out)).abs() < 1.0, "x_l={x_l}");
        assert!((y_l - (-max_out)).abs() < 1.0, "y_l={y_l}");
    }

    #[test]
    fn data_editor_items_sort_and_tag_priority_roundtrip() {
        let s = UserSettings {
            ui: UiLayout {
                data_editor_items_sort: CatalogItemSort::Tags,
                data_editor_items_tag_priority: vec!["rare".into(), "heal".into()],
                ..Default::default()
            },
            ..Default::default()
        };
        let yaml = serde_yaml::to_string(&s).unwrap();
        assert!(yaml.contains("data_editor_items_sort: tags"));
        assert!(yaml.contains("ui:"));
        assert!(yaml.contains("rare"));
        let back: UserSettings = serde_yaml::from_str(&yaml).unwrap();
        assert_eq!(back.ui.data_editor_items_sort, CatalogItemSort::Tags);
        assert_eq!(
            back.ui.data_editor_items_tag_priority,
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
            runtime: RuntimePrefs {
                diagnostic_logging: true,
                memory_logging: true,
                memory_logging_interval_ms: 2_000,
                restore_last_macro: false,
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
                ..Default::default()
            },
            ui: UiLayout {
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
            },
            overlay: OverlaySettings {
                overlay_default_size: 40.0,
                overlay_gate_default_interval_ms: 400,
                ..Default::default()
            },
            sound: SoundSettings {
                finish_sound_file: "/tmp/finish.mp3".into(),
                ..Default::default()
            },
            data: DataLifecycle {
                close_to_tray: true,
                start_minimized: true,
                backup_dir: "/tmp/backups".into(),
                update_check_interval_hours: 12,
                update_include_prerelease: true,
                ..Default::default()
            },
        };
        let back: UserSettings = serde_yaml::from_str(&serde_yaml::to_string(&s).unwrap()).unwrap();
        assert!(back.runtime.diagnostic_logging);
        assert!(!back.ui.window_decorations);
        assert!(back.runtime.suspend_screen_capture_when_idle);
        assert!(back.runtime.memory_logging);
        assert_eq!(back.runtime.memory_logging_interval_ms, 2_000);
        assert!(!back.runtime.restore_last_macro);
        assert!(back.data.close_to_tray);
        assert!(back.data.start_minimized);
        assert!(back.runtime.restore_cursor_on_end);
        assert_eq!(back.runtime.click_flash_ms, 500);
        assert_eq!(back.runtime.default_keyboard_delay_ms, 40);
        assert_eq!(back.runtime.default_mouse_delay_ms, 60);
        assert_eq!(back.runtime.panic_key, "f12");
        assert_eq!(
            back.runtime.hotkey_while_running,
            HotkeyWhileRunning::Restart
        );
        assert_eq!(back.runtime.default_hotkey_trigger, "release");
        assert_eq!(back.runtime.record_wait_threshold_ms, 100);
        assert_eq!(back.runtime.record_move_min_distance, 16);
        assert_eq!(back.runtime.record_tap_max_ms, 300);
        assert_eq!(back.runtime.record_snap_point_px, 40);
        assert!((back.overlay.overlay_default_size - 40.0).abs() < f32::EPSILON);
        assert_eq!(back.overlay.overlay_gate_default_interval_ms, 400);
        assert_eq!(back.sound.finish_sound_file, "/tmp/finish.mp3");
        assert_eq!(back.data.backup_dir, "/tmp/backups");
        assert_eq!(back.data.update_check_interval_hours, 12);
        assert!(back.data.update_include_prerelease);
        assert_eq!(back.ui.accent_color, "#336699");
        assert_eq!(back.ui.mono_font_size, 15);
        assert_eq!(back.ui.ui_font_family, UiFontFamily::Monospace);
        assert!((back.ui.window_width - 1100.0).abs() < f32::EPSILON);
        assert_eq!(back.ui.window_pos_x, Some(10.0));
        assert!(back.ui.window_maximized);
    }

    #[test]
    fn window_decorations_defaults_on_and_roundtrips_off() {
        let missing: UserSettings = serde_yaml::from_str("{}\n").unwrap();
        assert!(missing.ui.window_decorations);
        let off = UserSettings {
            ui: UiLayout {
                window_decorations: false,
                ..Default::default()
            },
            ..Default::default()
        };
        let back: UserSettings =
            serde_yaml::from_str(&serde_yaml::to_string(&off).unwrap()).unwrap();
        assert!(!back.ui.window_decorations);
    }

    #[test]
    fn suspend_screen_capture_defaults_on_and_roundtrips_off() {
        let missing: UserSettings = serde_yaml::from_str("{}\n").unwrap();
        assert!(missing.runtime.suspend_screen_capture_when_idle);
        let off = UserSettings {
            runtime: RuntimePrefs {
                suspend_screen_capture_when_idle: false,
                ..Default::default()
            },
            ..Default::default()
        };
        let back: UserSettings =
            serde_yaml::from_str(&serde_yaml::to_string(&off).unwrap()).unwrap();
        assert!(!back.runtime.suspend_screen_capture_when_idle);
    }

    #[test]
    fn clamp_normalizes_new_fields() {
        let mut s = UserSettings {
            runtime: RuntimePrefs {
                panic_key: "  F12  ".into(),
                default_hotkey_trigger: "bogus".into(),
                record_wait_threshold_ms: 1,
                ..Default::default()
            },
            ui: UiLayout {
                accent_color: "not-a-color".into(),
                ..Default::default()
            },
            data: DataLifecycle {
                update_check_interval_hours: -5,
                ..Default::default()
            },
            ..Default::default()
        };
        s.clamp();
        assert_eq!(s.runtime.panic_key, "f12");
        assert_eq!(s.runtime.default_hotkey_trigger, "press");
        assert!(s.ui.accent_color.is_empty());
        assert_eq!(
            s.runtime.record_wait_threshold_ms,
            DEFAULT_RECORD_WAIT_THRESHOLD_MS
        );
        assert_eq!(s.data.update_check_interval_hours, 0);
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
        let part = value
            .as_mapping_mut()
            .unwrap()
            .get_mut(serde_yaml::Value::String("ui".into()))
            .unwrap()
            .as_mapping_mut()
            .unwrap();
        part.remove(serde_yaml::Value::String("ui_density".into()));
        let settings: UserSettings = serde_yaml::from_value(value).unwrap();
        assert_eq!(settings.ui.ui_density, UiDensity::Normal);
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
        assert!(UserSettings::default().overlay.overlay_press_stops_run);
        let mut value = serde_yaml::to_value(UserSettings::default()).unwrap();
        let part = value
            .as_mapping_mut()
            .unwrap()
            .get_mut(serde_yaml::Value::String("overlay".into()))
            .unwrap()
            .as_mapping_mut()
            .unwrap();
        part.remove(serde_yaml::Value::String("overlay_press_stops_run".into()));
        let settings: UserSettings = serde_yaml::from_value(value).unwrap();
        assert!(settings.overlay.overlay_press_stops_run);

        let off = UserSettings {
            overlay: OverlaySettings {
                overlay_press_stops_run: false,
                ..Default::default()
            },
            ..Default::default()
        };
        let back: UserSettings =
            serde_yaml::from_str(&serde_yaml::to_string(&off).unwrap()).unwrap();
        assert!(!back.overlay.overlay_press_stops_run);
    }

    #[test]
    fn overlay_buttons_in_detection_defaults_on_when_absent() {
        assert!(UserSettings::default().overlay.overlay_buttons_in_detection);
        let mut value = serde_yaml::to_value(UserSettings::default()).unwrap();
        let part = value
            .as_mapping_mut()
            .unwrap()
            .get_mut(serde_yaml::Value::String("overlay".into()))
            .unwrap()
            .as_mapping_mut()
            .unwrap();
        part.remove(serde_yaml::Value::String(
            "overlay_buttons_in_detection".into(),
        ));
        let settings: UserSettings = serde_yaml::from_value(value).unwrap();
        assert!(settings.overlay.overlay_buttons_in_detection);

        let off = UserSettings {
            overlay: OverlaySettings {
                overlay_buttons_in_detection: false,
                ..Default::default()
            },
            ..Default::default()
        };
        let back: UserSettings =
            serde_yaml::from_str(&serde_yaml::to_string(&off).unwrap()).unwrap();
        assert!(!back.overlay.overlay_buttons_in_detection);
    }

    #[test]
    fn overlay_buttons_over_sqyre_defaults_off_when_absent() {
        assert!(!UserSettings::default().overlay.overlay_buttons_over_sqyre);
        let mut value = serde_yaml::to_value(UserSettings::default()).unwrap();
        let part = value
            .as_mapping_mut()
            .unwrap()
            .get_mut(serde_yaml::Value::String("overlay".into()))
            .unwrap()
            .as_mapping_mut()
            .unwrap();
        part.remove(serde_yaml::Value::String(
            "overlay_buttons_over_sqyre".into(),
        ));
        let settings: UserSettings = serde_yaml::from_value(value).unwrap();
        assert!(!settings.overlay.overlay_buttons_over_sqyre);

        let on = UserSettings {
            overlay: OverlaySettings {
                overlay_buttons_over_sqyre: true,
                ..Default::default()
            },
            ..Default::default()
        };
        let back: UserSettings =
            serde_yaml::from_str(&serde_yaml::to_string(&on).unwrap()).unwrap();
        assert!(back.overlay.overlay_buttons_over_sqyre);
    }

    #[test]
    fn click_effect_defaults_off_when_absent() {
        assert!(!UserSettings::default().runtime.click_effect);
        let mut value = serde_yaml::to_value(UserSettings::default()).unwrap();
        let part = value
            .as_mapping_mut()
            .unwrap()
            .get_mut(serde_yaml::Value::String("runtime".into()))
            .unwrap()
            .as_mapping_mut()
            .unwrap();
        part.remove(serde_yaml::Value::String("click_effect".into()));
        let settings: UserSettings = serde_yaml::from_value(value).unwrap();
        assert!(!settings.runtime.click_effect);

        let on = UserSettings {
            runtime: RuntimePrefs {
                click_effect: true,
                ..Default::default()
            },
            ..Default::default()
        };
        let back: UserSettings =
            serde_yaml::from_str(&serde_yaml::to_string(&on).unwrap()).unwrap();
        assert!(back.runtime.click_effect);
    }

    #[test]
    fn clamp_action_log_max_entries() {
        let mut s = UserSettings {
            runtime: RuntimePrefs {
                action_log_max_entries: 0,
                ..Default::default()
            },
            ..Default::default()
        };
        s.clamp();
        assert_eq!(
            s.runtime.action_log_max_entries,
            DEFAULT_ACTION_LOG_MAX_ENTRIES
        );
        s.runtime.action_log_max_entries = 5;
        s.clamp();
        assert_eq!(s.runtime.action_log_max_entries, MIN_ACTION_LOG_MAX_ENTRIES);
        s.runtime.action_log_max_entries = 999_999;
        s.clamp();
        assert_eq!(s.runtime.action_log_max_entries, MAX_ACTION_LOG_MAX_ENTRIES);
        assert_eq!(
            usize::try_from(MAX_ACTION_LOG_MAX_ENTRIES).unwrap(),
            sqyre_ports::MAX_ENTRIES_PER_ACTION
        );
    }

    #[test]
    fn action_log_max_entries_defaults_when_absent() {
        let mut value = serde_yaml::to_value(UserSettings::default()).unwrap();
        let part = value
            .as_mapping_mut()
            .unwrap()
            .get_mut(serde_yaml::Value::String("runtime".into()))
            .unwrap()
            .as_mapping_mut()
            .unwrap();
        part.remove(serde_yaml::Value::String("action_log_max_entries".into()));
        let settings: UserSettings = serde_yaml::from_value(value).unwrap();
        assert_eq!(
            settings.runtime.action_log_max_entries,
            DEFAULT_ACTION_LOG_MAX_ENTRIES
        );
    }

    #[test]
    fn clamp_data_editor_left_frac() {
        let mut s = UserSettings {
            ui: UiLayout {
                data_editor_left_split: 0.05,
                ..Default::default()
            },
            ..Default::default()
        };
        s.clamp();
        assert!((s.ui.data_editor_left_split - MIN_DATA_EDITOR_LEFT_FRAC).abs() < f32::EPSILON);
        s.ui.data_editor_left_split = 0.9;
        s.clamp();
        assert!((s.ui.data_editor_left_split - MAX_DATA_EDITOR_LEFT_FRAC).abs() < f32::EPSILON);
    }

    #[test]
    fn clamp_macro_list_frac() {
        let mut s = UserSettings {
            ui: UiLayout {
                macro_list_split: 0.05,
                ..Default::default()
            },
            ..Default::default()
        };
        s.clamp();
        assert!((s.ui.macro_list_split - MIN_MACRO_LIST_FRAC).abs() < f32::EPSILON);
        s.ui.macro_list_split = 0.9;
        s.clamp();
        assert!((s.ui.macro_list_split - MAX_MACRO_LIST_FRAC).abs() < f32::EPSILON);
    }

    #[test]
    fn default_worker_threads_caps_at_sixteen() {
        let d = default_worker_threads();
        assert!(d >= MIN_WORKER_THREADS);
        assert!(d <= DEFAULT_WORKER_THREADS_CAP);
        assert!(d <= available_worker_threads());
        let mut s = UserSettings::default();
        assert_eq!(s.runtime.worker_threads, d);
        s.runtime.worker_threads = 0;
        s.clamp();
        assert_eq!(s.runtime.worker_threads, d);
        s.runtime.worker_threads = available_worker_threads().saturating_add(100);
        s.clamp();
        assert_eq!(s.runtime.worker_threads, available_worker_threads());
    }

    #[test]
    fn rename_overlay_macro_updates_matching_buttons() {
        let mut s = UserSettings::default();
        let mut a = OverlayButtonConfig::new("a", "P");
        a.macro_name = "old".into();
        let mut b = OverlayButtonConfig::new("b", "P");
        b.macro_name = "other".into();
        s.overlay.overlay_buttons.push(a);
        s.overlay.overlay_buttons.push(b);
        assert!(s.rename_overlay_macro("old", "new"));
        assert_eq!(s.overlay.overlay_buttons[0].macro_name, "new");
        assert_eq!(s.overlay.overlay_buttons[1].macro_name, "other");
        assert!(!s.rename_overlay_macro("old", "new"));
        assert!(!s.rename_overlay_macro("", "x"));
        assert!(!s.rename_overlay_macro("new", "new"));
    }

    #[test]
    fn remove_overlay_buttons_for_program_drops_matching() {
        let mut s = UserSettings::default();
        s.overlay
            .overlay_buttons
            .push(OverlayButtonConfig::new("keep", "Other"));
        s.overlay
            .overlay_buttons
            .push(OverlayButtonConfig::new("drop-a", "Gone"));
        s.overlay
            .overlay_buttons
            .push(OverlayButtonConfig::new("drop-b", "Gone"));
        s.overlay
            .overlay_buttons
            .push(default_quick_access_hub_button());
        assert!(s.remove_overlay_buttons_for_program("Gone"));
        assert_eq!(s.overlay.overlay_buttons.len(), 2);
        assert!(s.overlay.overlay_buttons.iter().any(|b| b.id == "keep"));
        assert!(s
            .overlay
            .overlay_buttons
            .iter()
            .any(|b| is_quick_access_hub_id(&b.id)));
        assert!(!s.remove_overlay_buttons_for_program("Gone"));
        assert!(!s.remove_overlay_buttons_for_program("Missing"));
        // Hub stays even when clearing General program buttons.
        assert!(!s.remove_overlay_buttons_for_program(sqyre_domain::GENERAL_PROGRAM));
        assert!(s
            .overlay
            .overlay_buttons
            .iter()
            .any(|b| is_quick_access_hub_id(&b.id)));

        let dir = tempdir().unwrap();
        let path = dir.path().join(crate::SETTINGS_FILE);
        s.save_to_path(&path).unwrap();
        let loaded = UserSettings::load_from_path(&path).unwrap();
        assert!(loaded
            .overlay
            .overlay_buttons
            .iter()
            .any(|b| b.id == "keep"));
        assert!(loaded
            .overlay
            .overlay_buttons
            .iter()
            .any(|b| is_quick_access_hub_id(&b.id)));
        assert_eq!(
            loaded
                .overlay
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
            runtime: RuntimePrefs {
                hotkey_tag_filters: vec!["Old".into(), "Old/raid".into(), "keep".into()],
                ..Default::default()
            },
            ..Default::default()
        };
        assert!(s.rename_hotkey_tag_program("Old", "New"));
        assert_eq!(
            s.runtime.hotkey_tag_filters,
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
        std::fs::write(&path, "runtime:\n  save_meta_images: true\n").unwrap();
        let loaded = UserSettings::load_from_path(&path).unwrap();
        assert!(loaded.runtime.hotkey_tag_filters.is_empty());
        assert!(!loaded.runtime.hotkey_tags_while_focused);
        assert!(loaded.runtime.image_search_variant_exit_early);
    }

    #[test]
    fn overlay_style_defaults_when_omitted() {
        let dir = tempdir().unwrap();
        let path = dir.path().join(crate::SETTINGS_FILE);
        std::fs::write(
            &path,
            r#"
overlay:
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
        let btn = loaded
            .overlay
            .overlay_buttons
            .iter()
            .find(|b| b.id == "btn-legacy")
            .expect("legacy button");
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
        assert!(loaded
            .overlay
            .overlay_buttons
            .iter()
            .any(|b| is_quick_access_hub_id(&b.id)));
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
