//! Persistence for `~/.sqyre` (db.yaml, settings, program catalog).

mod programs;
mod settings;

pub use programs::{
    resolve_scalar_int, ProgramCatalog, ProgramCollection, ProgramData, ProgramItem, ProgramMask,
    ProgramPoint, ProgramSearchArea,
};
pub use settings::{
    move_dir, open_path_in_file_manager, open_sqyre_dir, settings_path, ActionColorPrefs,
    UserSettings, ACTION_COLOR_DEFAULT, ACTION_COLOR_DETECTION, ACTION_COLOR_MISCELLANEOUS,
    ACTION_COLOR_MOUSE_KEYBOARD, ACTION_COLOR_VARIABLES, ACTION_COLOR_WAIT,
    DEFAULT_DRAG_PREVIEW_DEBOUNCE_MS, DEFAULT_HIDE_APP_DURING_RECORDING,
    DEFAULT_IMAGE_SEARCH_CLOSE_MATCHES_DISTANCE, DEFAULT_UI_FONT_SIZE, DEFAULT_UI_SCALE,
    MIN_DRAG_PREVIEW_DEBOUNCE_MS,
};

use serde_yaml::{Mapping, Value};
use sqyre_domain::Macro;
use sqyre_serialize::{decode_macro_from_map, encode_macro_to_map};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::RwLock;
use thiserror::Error;

const SQYRE_DIR: &str = ".sqyre";
const DB_FILE: &str = "db.yaml";

static DIR_OVERRIDE: RwLock<Option<PathBuf>> = RwLock::new(None);

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

/// Override the Sqyre data directory (empty clears → `~/.sqyre`).
pub fn set_sqyre_dir_override(path: Option<PathBuf>) {
    *DIR_OVERRIDE.write().unwrap() = path;
}

pub fn sqyre_dir() -> PathBuf {
    if let Some(p) = DIR_OVERRIDE.read().unwrap().clone() {
        return p;
    }
    dirs::home_dir()
        .unwrap_or_else(std::env::temp_dir)
        .join(SQYRE_DIR)
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

/// In-memory view of `db.yaml`.
#[derive(Debug, Clone, Default)]
pub struct Database {
    pub macros: BTreeMap<String, Macro>,
    /// Programs remain as raw YAML; use [`Self::program_catalog`] for lookups.
    pub programs: Value,
}

impl Database {
    pub fn program_catalog(&self) -> Result<ProgramCatalog> {
        ProgramCatalog::from_yaml_value(&self.programs)
    }

    /// Replace `programs` from a typed catalog, preserving masks/collections via merge.
    pub fn set_programs_from_catalog(&mut self, catalog: &ProgramCatalog) {
        self.programs = catalog.to_yaml_value(&self.programs);
    }

    /// Replace the macros map from an ordered list (keyed by macro name).
    pub fn replace_macros(&mut self, macros: impl IntoIterator<Item = Macro>) {
        self.macros = macros.into_iter().map(|m| (m.name.clone(), m)).collect();
    }

    pub fn load_from_path(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        if !path.exists() {
            return Ok(Self {
                macros: BTreeMap::new(),
                programs: Value::Mapping(Mapping::new()),
            });
        }
        let text = fs::read_to_string(path)?;
        Self::from_yaml(&text)
    }

    pub fn load_default() -> Result<Self> {
        Self::load_from_path(db_path())
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
                let mut macro_ = decode_macro_from_map(v).map_err(|e| {
                    PersistError::Message(format!("macro \"{key}\": {e}"))
                })?;
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

        Ok(Self { macros, programs })
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
        fs::write(path, self.to_yaml()?)?;
        Ok(())
    }

    pub fn save_default(&self) -> Result<()> {
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
        set_sqyre_dir_override(Some(dir.path().to_path_buf()));
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

        set_sqyre_dir_override(None);
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
}
