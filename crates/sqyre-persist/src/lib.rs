//! Persistence for `~/.sqyre` (db.yaml, settings, program catalog).

#[cfg(not(target_arch = "wasm32"))]
mod backup;
mod database;
mod error;
mod fs_name;
mod fs_util;
mod icon_fs_cache;
mod import;
mod macro_yaml_draft;
mod migrate;
#[cfg(test)]
#[path = "migrate_tests.rs"]
mod migrate_tests;
mod paths;
mod programs;
mod settings;

pub use database::{ensure_db_file, Database};
pub use error::{PersistError, Result};
pub use paths::{
    backup_dir_override, db_path, images_path, initialize_directories, screen_cap_path,
    screen_cap_trash_path, set_backup_dir_override, set_sqyre_dir_override, sqyre_dir,
    variables_path, with_sqyre_dir_override, with_sqyre_dirs_override, DB_FILE, MAX_DB_YAML_BYTES,
    MAX_MACROS, SETTINGS_FILE,
};
#[cfg(not(target_arch = "wasm32"))]
pub use paths::{set_home_dir, sqyre_config_dir};

#[cfg(not(target_arch = "wasm32"))]
pub use backup::{
    backups_dir, create_backup, import_backup, list_backups, prune_backups, restore_backup,
    BackupError,
};
pub use fs_name::{confined_join, is_safe_fs_entity_name, validate_fs_entity_name};
pub use icon_fs_cache::{clear_icon_fs_cache, invalidate_icon_fs_cache_under};
pub use import::{merge_databases_prefer_imported, ImportMode};
pub use macro_yaml_draft::{macro_yaml_builder_path, MacroYamlBuilderDrafts, MacroYamlDraftEntry};
pub use migrate::{migrate_db_yaml_value, LegacyCatalog};

/// Parse, migrate, and validate against the current [`Database`] loader.
pub fn migrate_db_yaml(text: &str) -> Result<String> {
    if text.len() > MAX_DB_YAML_BYTES {
        return Err(PersistError::Message(format!(
            "db.yaml too large ({} bytes; max {MAX_DB_YAML_BYTES})",
            text.len(),
        )));
    }
    check_yaml_safety(text)?;
    let mut root: serde_yaml::Value = serde_yaml::from_str(text)?;
    migrate::migrate_db_yaml_value(&mut root);
    let out = serde_yaml::to_string(&root)?;
    Database::from_yaml(&out)?;
    Ok(out)
}

pub use programs::{
    absolute_area_to_relative, absolute_point_to_relative, ensure_general_program,
    merge_fs_warning, monitor_slot_for_point, CatalogView, FsWarn, MonitorRect, ProgramAtlas,
    ProgramCatalog, ProgramCollection, ProgramData, ProgramItem, ProgramMask, ProgramPoint,
    ProgramSearchArea,
};
pub use settings::{
    available_worker_threads, default_overlay_position, default_quick_access_hub_button,
    default_worker_threads, ensure_quick_access_hub_button, hub_position_on_monitor,
    is_quick_access_hub_id, move_dir, open_path_in_file_manager, open_sqyre_dir, settings_path,
    snap_hub_button_to_monitor, snap_hub_to_monitor, snap_hub_xy_to_monitor, ActionColorPrefs,
    DataLifecycle, HotkeyWhileRunning, HubEdge, OverlayButtonConfig, OverlaySettings,
    OverlayVisibilityGate, OverlayVisibilityMode, RuntimePrefs, SoundSettings, UiDensity,
    UiFontFamily, UiLayout, UserSettings, DEFAULT_ACTION_LOG_MAX_ENTRIES,
    DEFAULT_AUTO_UPDATE_CHECK, DEFAULT_BACKUP_INTERVAL_HOURS, DEFAULT_BACKUP_MAX_KEEP,
    DEFAULT_CLICK_FLASH_MS, DEFAULT_DATA_EDITOR_LEFT_FRAC, DEFAULT_HIDE_APP_DURING_RECORDING,
    DEFAULT_IMAGE_SEARCH_CLOSE_MATCHES_DISTANCE, DEFAULT_IMAGE_SEARCH_VARIANT_EXIT_EARLY,
    DEFAULT_MACRO_KEYBOARD_DELAY_MS, DEFAULT_MACRO_LIST_FRAC, DEFAULT_MACRO_MOUSE_DELAY_MS,
    DEFAULT_OVERLAY_ACCENT_HEX, DEFAULT_OVERLAY_BORDER_WIDTH, DEFAULT_OVERLAY_BUTTON_SIZE,
    DEFAULT_OVERLAY_CORNER_RADIUS, DEFAULT_OVERLAY_FALLBACK_SCREEN_H,
    DEFAULT_OVERLAY_FALLBACK_SCREEN_W, DEFAULT_OVERLAY_GATE_BLUR, DEFAULT_OVERLAY_GATE_INTERVAL_MS,
    DEFAULT_OVERLAY_GATE_TOLERANCE, DEFAULT_OVERLAY_ICON_HEX, DEFAULT_PANIC_KEY,
    DEFAULT_PLAY_FINISH_SOUND, DEFAULT_QUICK_ACCESS_OPACITY, DEFAULT_RECORD_MOVE_MIN_DISTANCE,
    DEFAULT_RECORD_SNAP_POINT_PX, DEFAULT_RECORD_TAP_MAX_MS, DEFAULT_RECORD_WAIT_THRESHOLD_MS,
    DEFAULT_RELEASE_HELD_INPUTS_ON_END, DEFAULT_RUN_MACRO_MAX_DEPTH, DEFAULT_SOUND_VOLUME,
    DEFAULT_UI_FONT_SIZE, DEFAULT_UI_SCALE, DEFAULT_UPDATE_CHECK_INTERVAL_HOURS,
    DEFAULT_WHILE_MAX_ITERATIONS, DEFAULT_WORKER_THREADS_CAP, HUB_MAX_EDGE_OVERHANG,
    MAX_ACTION_LOG_MAX_ENTRIES, MAX_BACKUP_INTERVAL_HOURS, MAX_BACKUP_MAX_KEEP, MAX_CLICK_FLASH_MS,
    MAX_DATA_EDITOR_LEFT_FRAC, MAX_LOG_IMAGE_MAX_EDGE, MAX_MACRO_LIST_FRAC,
    MAX_MEMORY_SAMPLE_INTERVAL_MS, MAX_OVERLAY_BORDER_WIDTH, MAX_OVERLAY_BUTTON_SIZE,
    MAX_OVERLAY_CORNER_RADIUS, MAX_OVERLAY_GATE_INTERVAL_MS, MAX_RECORD_MOVE_MIN_DISTANCE,
    MAX_RECORD_SNAP_POINT_PX, MAX_RECORD_TAP_MAX_MS, MAX_RECORD_WAIT_THRESHOLD_MS,
    MAX_RUN_MACRO_MAX_DEPTH, MAX_UI_SCALE, MAX_UPDATE_CHECK_INTERVAL_HOURS,
    MAX_WHILE_MAX_ITERATIONS, MIN_ACTION_LOG_MAX_ENTRIES, MIN_BACKUP_INTERVAL_HOURS,
    MIN_BACKUP_MAX_KEEP, MIN_CLICK_FLASH_MS, MIN_DATA_EDITOR_LEFT_FRAC, MIN_LOG_IMAGE_MAX_EDGE,
    MIN_MACRO_LIST_FRAC, MIN_MEMORY_SAMPLE_INTERVAL_MS, MIN_OVERLAY_BORDER_WIDTH,
    MIN_OVERLAY_BUTTON_SIZE, MIN_OVERLAY_CORNER_RADIUS, MIN_OVERLAY_GATE_INTERVAL_MS,
    MIN_RECORD_MOVE_MIN_DISTANCE, MIN_RECORD_SNAP_POINT_PX, MIN_RECORD_TAP_MAX_MS,
    MIN_RECORD_WAIT_THRESHOLD_MS, MIN_RUN_MACRO_MAX_DEPTH, MIN_UI_SCALE,
    MIN_UPDATE_CHECK_INTERVAL_HOURS, MIN_WHILE_MAX_ITERATIONS, MIN_WORKER_THREADS,
    QUICK_ACCESS_HUB_DISMISS, QUICK_ACCESS_HUB_ICON, QUICK_ACCESS_HUB_ID, QUICK_ACCESS_HUB_LABEL,
};
pub use sqyre_domain::resolve_scalar_int;
pub use sqyre_serialize::{check_yaml_nesting_depth, check_yaml_safety, MAX_YAML_NESTING_DEPTH};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fs_util::atomic_write;
    use serde_yaml::{Mapping, Value};
    use sqyre_domain::{root_loop, Action, ActionKind, Macro, ScalarValue};

    #[test]
    fn roundtrip_temp_db() {
        let dir = tempfile::tempdir().unwrap();
        with_sqyre_dir_override(dir.path().to_path_buf(), || {
            initialize_directories().unwrap();

            let mut db = Database::default();
            let mut m = Macro::new("Test", 10, vec!["ctrl".into()]);
            m.root = root_loop(vec![Action {
                id: sqyre_domain::ActionId::new(),
                kind: ActionKind::Wait {
                    time: ScalarValue::Int(50),
                },
            }]);
            db.macros.insert("Test".into(), m);
            db.save_default().unwrap();

            let loaded = Database::load_default().unwrap();
            assert!(loaded.macros.contains_key("Test"));
            assert_eq!(loaded.macros["Test"].root.children().len(), 1);
        });
    }

    #[test]
    fn rejects_too_many_macros() {
        let mut macros = Mapping::new();
        for i in 0..=MAX_MACROS {
            macros.insert(
                Value::String(format!("m{i}")),
                Value::Mapping(Mapping::new()),
            );
        }
        let mut root = Mapping::new();
        root.insert(Value::String("macros".into()), Value::Mapping(macros));
        root.insert(
            Value::String("programs".into()),
            Value::Mapping(Mapping::new()),
        );
        let text = serde_yaml::to_string(&Value::Mapping(root)).unwrap();
        let err = Database::from_yaml_with_warnings(&text).unwrap_err();
        assert!(
            err.to_string().contains("too many macros"),
            "expected macro count error, got {err}"
        );
    }

    #[test]
    fn rejects_oversized_db_yaml() {
        let mut huge = String::from("macros: {}\nprograms: {}\n#");
        huge.push_str(&"x".repeat(MAX_DB_YAML_BYTES));
        let err = Database::from_yaml_with_warnings(&huge).unwrap_err();
        assert!(
            err.to_string().contains("too large"),
            "expected size error, got {err}"
        );
    }

    #[test]
    fn rejects_yaml_aliases_before_expansion() {
        // Billion-laughs shape: serde_yaml would expand `*x` without bound, so
        // the event-level guard must reject it before `Value` materialization.
        let text = "macros:\n  a: &x [1, 2, 3]\n  b: *x\nprograms: {}\n";
        let err = Database::from_yaml_with_warnings(text).unwrap_err();
        assert!(
            err.to_string().contains("aliases"),
            "expected alias rejection, got {err}"
        );
    }

    #[test]
    fn rejects_deeply_nested_yaml() {
        let mut v = Value::Null;
        for _ in 0..=MAX_YAML_NESTING_DEPTH + 1 {
            let mut m = Mapping::new();
            m.insert(Value::String("a".into()), v);
            v = Value::Mapping(m);
        }
        let err = check_yaml_nesting_depth(&v).unwrap_err();
        assert!(
            err.to_string().contains("nesting too deep"),
            "expected nesting error, got {err}"
        );

        let mut root = Mapping::new();
        root.insert(
            Value::String("macros".into()),
            Value::Mapping(Mapping::new()),
        );
        root.insert(Value::String("programs".into()), v);
        let text = serde_yaml::to_string(&Value::Mapping(root)).unwrap();
        let err = Database::from_yaml_with_warnings(&text).unwrap_err();
        assert!(
            err.to_string().contains("nesting too deep"),
            "expected nesting error, got {err}"
        );
    }

    #[test]
    fn one_bad_macro_does_not_block_others() {
        let text = r#"
macros:
  Good:
    name: Good
    globaldelay: 0
    hotkey: []
    root:
      type: loop
      name: root
      count: 1
      subactions:
        - type: wait
          time: 1
  Bad:
    name: Bad
    root: { type: not_a_real_action }
programs: {}
"#;
        let (db, warnings) = Database::from_yaml_with_warnings(text).unwrap();
        assert!(db.macros.contains_key("Good"));
        assert!(!db.macros.contains_key("Bad"));
        assert!(
            warnings.iter().any(|w| w.contains("Bad")),
            "warnings={warnings:?}"
        );
    }

    #[test]
    fn loads_yaml_fixture() {
        let text = r#"
macros:
  Integration Test Macro:
    name: Integration Test Macro
    globaldelay: 10
    hotkey: []
    root:
      type: loop
      name: root
      count: 1
      subactions:
        - type: wait
          time: 1
        - type: wait
          time: 2
        - type: wait
          time: 3
programs: {}
"#;
        let db = Database::from_yaml(text).unwrap();
        assert!(db.macros.contains_key("Integration Test Macro"));
        let m = &db.macros["Integration Test Macro"];
        assert_eq!(m.root.children().len(), 3);
    }

    #[test]
    fn corrupt_yaml_errors_cleanly() {
        let err = Database::from_yaml("macros: [unterminated").unwrap_err();
        // The event-level safety scan may report the syntax error before
        // serde_yaml gets a chance to — either yaml-family error is "clean".
        assert!(
            matches!(err, PersistError::Yaml(_) | PersistError::Serialize(_)),
            "expected yaml error, got {err:?}"
        );
    }

    #[test]
    fn atomic_write_replaces_and_leaves_no_tmp() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(DB_FILE);
        std::fs::write(&path, b"old").unwrap();
        atomic_write(&path, b"new-content").unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "new-content");
        let tmp = dir.path().join("db.yaml.tmp");
        assert!(!tmp.exists(), "tmp sibling should be cleaned up");
    }

    #[test]
    fn save_overwrites_corrupt_file() {
        let dir = tempfile::tempdir().unwrap();
        with_sqyre_dir_override(dir.path().to_path_buf(), || {
            initialize_directories().unwrap();
            let path = db_path();
            std::fs::write(&path, b"not: valid: yaml: [[[").unwrap();
            assert!(Database::load_from_path(&path).is_err());

            let mut db = Database::default();
            db.macros
                .insert("Recovered".into(), Macro::new("Recovered", 0, vec![]));
            db.save_to_path(&path).unwrap();
            let loaded = Database::load_from_path(&path).unwrap();
            assert!(loaded.macros.contains_key("Recovered"));
        });
    }
}
