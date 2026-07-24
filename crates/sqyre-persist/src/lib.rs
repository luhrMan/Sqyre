//! Persistence for `~/.sqyre` (db.yaml, settings, program catalog).

#[cfg(not(target_arch = "wasm32"))]
mod backup;
mod migrate;
mod programs;
mod settings;

#[cfg(not(target_arch = "wasm32"))]
pub use backup::{
    backups_dir, create_backup, list_backups, prune_backups, restore_backup, BackupError,
};
pub use migrate::{migrate_db_yaml, migrate_db_yaml_value, LegacyCatalog};
pub use programs::{
    ProgramCatalog, ProgramCollection, ProgramData, ProgramItem, ProgramMask, ProgramPoint,
    ProgramSearchArea,
};
pub use settings::{
    move_dir, open_path_in_file_manager, open_sqyre_dir, settings_path, ActionColorPrefs,
    OverlayButtonConfig, UserSettings, DEFAULT_AUTO_UPDATE_CHECK, DEFAULT_BACKUP_INTERVAL_HOURS,
    DEFAULT_BACKUP_MAX_KEEP, DEFAULT_DRAG_PREVIEW_DEBOUNCE_MS, DEFAULT_HIDE_APP_DURING_RECORDING,
    DEFAULT_IMAGE_SEARCH_CLOSE_MATCHES_DISTANCE, DEFAULT_OVERLAY_ACCENT_HEX,
    DEFAULT_OVERLAY_BORDER_WIDTH, DEFAULT_OVERLAY_BUTTON_SIZE, DEFAULT_OVERLAY_CORNER_RADIUS,
    DEFAULT_OVERLAY_ICON_HEX, DEFAULT_PLAY_FINISH_SOUND, DEFAULT_PLAY_UI_SOUNDS,
    DEFAULT_SOUND_VOLUME, DEFAULT_UI_FONT_SIZE, DEFAULT_UI_SCALE, MAX_BACKUP_INTERVAL_HOURS,
    MAX_BACKUP_MAX_KEEP, MAX_OVERLAY_BORDER_WIDTH, MAX_OVERLAY_BUTTON_SIZE,
    MAX_OVERLAY_CORNER_RADIUS, MIN_BACKUP_INTERVAL_HOURS, MIN_BACKUP_MAX_KEEP,
    MIN_DRAG_PREVIEW_DEBOUNCE_MS, MIN_OVERLAY_BORDER_WIDTH, MIN_OVERLAY_BUTTON_SIZE,
    MIN_OVERLAY_CORNER_RADIUS,
};
pub use sqyre_domain::resolve_scalar_int;

use serde_yaml::{Mapping, Value};
use sqyre_domain::Macro;
use sqyre_serialize::{decode_macro_from_map, encode_macro_to_map};
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock, RwLock};
use thiserror::Error;

const SQYRE_DIR: &str = ".sqyre";
const DB_FILE: &str = "db.yaml";

static DIR_OVERRIDE: RwLock<Option<PathBuf>> = RwLock::new(None);

/// Serializes tests that mutate [`set_sqyre_dir_override`].
fn dir_override_test_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

#[derive(Debug, Error)]
pub enum PersistError {
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Yaml(#[from] serde_yaml::Error),
    #[error(transparent)]
    Serialize(#[from] sqyre_serialize::SerializeError),
    #[error("{0}")]
    Message(String),
}

pub type Result<T> = std::result::Result<T, PersistError>;

/// Write `bytes` to `path` via a sibling temp file + rename (atomic on same filesystem).
pub(crate) fn atomic_write(path: &Path, bytes: impl AsRef<[u8]>) -> std::io::Result<()> {
    use std::io::Write;

    let mut tmp = path.as_os_str().to_owned();
    tmp.push(".tmp");
    let tmp = PathBuf::from(tmp);

    let write_tmp = || -> std::io::Result<()> {
        let mut f = fs::File::create(&tmp)?;
        f.write_all(bytes.as_ref())?;
        f.sync_all()?;
        Ok(())
    };
    if let Err(e) = write_tmp() {
        let _ = fs::remove_file(&tmp);
        return Err(e);
    }
    if let Err(e) = fs::rename(&tmp, path) {
        let _ = fs::remove_file(&tmp);
        return Err(e);
    }
    Ok(())
}

/// Override the Sqyre data directory (empty clears → `~/.sqyre`).
pub fn set_sqyre_dir_override(path: Option<PathBuf>) {
    *DIR_OVERRIDE.write().unwrap() = path;
}

/// Per-user config directory for Sqyre (`~/.config/sqyre` on Linux).
///
/// Holds the data-dir pointer and the single-instance lock — paths that must
/// not move when the relocatable data directory changes.
#[cfg(not(target_arch = "wasm32"))]
pub fn sqyre_config_dir() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| {
            dirs::home_dir()
                .unwrap_or_else(std::env::temp_dir)
                .join(".config")
        })
        .join("sqyre")
}

/// Run `f` with a temporary Sqyre dir override, serialized against other override users.
pub fn with_sqyre_dir_override<R>(path: PathBuf, f: impl FnOnce() -> R) -> R {
    let _guard = dir_override_test_lock()
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    set_sqyre_dir_override(Some(path));
    let result = f();
    set_sqyre_dir_override(None);
    result
}

pub fn sqyre_dir() -> PathBuf {
    if let Some(p) = DIR_OVERRIDE.read().unwrap().clone() {
        return p;
    }
    // `std::env::temp_dir()` panics on wasm32-unknown-unknown ("no filesystem").
    #[cfg(target_arch = "wasm32")]
    {
        return PathBuf::from("/").join(SQYRE_DIR);
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        dirs::home_dir()
            .unwrap_or_else(std::env::temp_dir)
            .join(SQYRE_DIR)
    }
}

pub fn db_path() -> PathBuf {
    sqyre_dir().join(DB_FILE)
}

pub fn variables_path() -> PathBuf {
    sqyre_dir().join("variables")
}

pub fn images_path() -> PathBuf {
    sqyre_dir().join("images")
}

pub fn auto_pic_path() -> PathBuf {
    images_path().join("AutoPic")
}

pub fn initialize_directories() -> Result<()> {
    #[cfg(target_arch = "wasm32")]
    {
        return Ok(());
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        for p in [
            sqyre_dir().join("images/icons"),
            sqyre_dir().join("images/AutoPic"),
            sqyre_dir().join("images/Collections"),
            sqyre_dir().join("images/masks"),
            variables_path(),
        ] {
            fs::create_dir_all(&p)?;
        }
        Ok(())
    }
}

/// In-memory view of `db.yaml`.
#[derive(Debug, Clone, Default)]
pub struct Database {
    pub macros: BTreeMap<String, Macro>,
    /// Programs remain as raw YAML; use [`Self::program_catalog`] for lookups.
    pub programs: Value,
    /// Parsed catalog cache; invalidated when `programs` is replaced via known mutators.
    catalog_cache: RefCell<Option<ProgramCatalog>>,
}

impl Database {
    pub fn program_catalog(&self) -> Result<ProgramCatalog> {
        if let Some(cached) = self.catalog_cache.borrow().as_ref() {
            return Ok(cached.clone());
        }
        let catalog = ProgramCatalog::from_yaml_value(&self.programs)?;
        *self.catalog_cache.borrow_mut() = Some(catalog.clone());
        Ok(catalog)
    }

    fn invalidate_catalog_cache(&mut self) {
        *self.catalog_cache.get_mut() = None;
    }

    /// Replace `programs` from a typed catalog, preserving masks/collections via merge.
    pub fn set_programs_from_catalog(&mut self, catalog: &ProgramCatalog) {
        self.programs = catalog.to_yaml_value(&self.programs);
        self.invalidate_catalog_cache();
    }

    /// Replace the macros map from an ordered list (keyed by macro name).
    pub fn replace_macros(&mut self, macros: impl IntoIterator<Item = Macro>) {
        self.macros = macros.into_iter().map(|m| (m.name.clone(), m)).collect();
    }

    pub fn load_from_path(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        if !path.exists() {
            return Ok(Self::default());
        }
        let text = fs::read_to_string(path)?;
        Self::from_yaml(&text)
    }

    pub fn load_default() -> Result<Self> {
        #[cfg(target_arch = "wasm32")]
        {
            return Ok(Self::default());
        }
        #[cfg(not(target_arch = "wasm32"))]
        Self::load_from_path(db_path())
    }

    /// Parse `db.yaml` bytes (UTF-8). Used by WASM import and tests.
    pub fn from_yaml_bytes(bytes: &[u8]) -> Result<Self> {
        let text = std::str::from_utf8(bytes)
            .map_err(|e| PersistError::Message(format!("db.yaml is not UTF-8: {e}")))?;
        Self::from_yaml(text)
    }

    /// Serialize to YAML bytes (UTF-8). Used by WASM export.
    pub fn to_yaml_bytes(&self) -> Result<Vec<u8>> {
        Ok(self.to_yaml()?.into_bytes())
    }

    pub fn from_yaml(text: &str) -> Result<Self> {
        let root: Value = serde_yaml::from_str(text)?;
        let mapping = root
            .as_mapping()
            .ok_or_else(|| PersistError::Message("db.yaml root must be a mapping".into()))?;

        let mut macros = BTreeMap::new();
        if let Some(Value::Mapping(mm)) = mapping.get(Value::String("macros".into())) {
            for (k, v) in mm {
                let key = k
                    .as_str()
                    .ok_or_else(|| PersistError::Message("macro key must be a string".into()))?
                    .to_string();
                let mut macro_ = decode_macro_from_map(v)
                    .map_err(|e| PersistError::Message(format!("macro \"{key}\": {e}")))?;
                if macro_.name.is_empty() {
                    macro_.name = key.clone();
                }
                macros.insert(key, macro_);
            }
        }

        let programs = mapping
            .get(Value::String("programs".into()))
            .cloned()
            .unwrap_or(Value::Mapping(Mapping::new()));

        Ok(Self {
            macros,
            programs,
            catalog_cache: RefCell::new(None),
        })
    }

    pub fn to_yaml(&self) -> Result<String> {
        let mut root = Mapping::new();
        let mut mm = Mapping::new();
        for (key, macro_) in &self.macros {
            let encoded = encode_macro_to_map(macro_)?;
            mm.insert(Value::String(key.clone()), Value::Mapping(encoded));
        }
        root.insert(Value::String("macros".into()), Value::Mapping(mm));
        root.insert(Value::String("programs".into()), self.programs.clone());
        Ok(serde_yaml::to_string(&Value::Mapping(root))?)
    }

    pub fn save_to_path(&self, path: impl AsRef<Path>) -> Result<()> {
        let path = path.as_ref();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        atomic_write(path, self.to_yaml()?)?;
        Ok(())
    }

    pub fn save_default(&self) -> Result<()> {
        #[cfg(target_arch = "wasm32")]
        {
            // In-memory only in the browser; use YAML export to download.
            let _ = self.to_yaml()?;
            return Ok(());
        }
        #[cfg(not(target_arch = "wasm32"))]
        self.save_to_path(db_path())
    }

    pub fn macro_names(&self) -> Vec<String> {
        self.macros.keys().cloned().collect()
    }
}

/// Ensure `db.yaml` exists with empty macros/programs.
pub fn ensure_db_file() -> Result<PathBuf> {
    initialize_directories()?;
    let path = db_path();
    if !path.exists() {
        let db = Database::default();
        db.save_to_path(&path)?;
    }
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqyre_domain::{root_loop, Action, ActionKind, ScalarValue};

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
        assert!(
            matches!(err, PersistError::Yaml(_)),
            "expected yaml error, got {err:?}"
        );
    }

    #[test]
    fn atomic_write_replaces_and_leaves_no_tmp() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("db.yaml");
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
