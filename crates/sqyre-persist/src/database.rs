//! On-disk macro database (`db.yaml`).

use crate::error::{PersistError, Result};
use crate::fs_util::atomic_write;
use crate::paths::{db_path, initialize_directories, MAX_DB_YAML_BYTES, MAX_MACROS};
use crate::programs::ProgramCatalog;
use serde_yaml::{Mapping, Value};
use sqyre_domain::Macro;
use sqyre_serialize::{
    check_yaml_nesting_depth, check_yaml_safety, decode_macro_from_map, encode_macro_to_map,
};
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// In-memory view of `db.yaml`.
#[derive(Debug, Clone, Default)]
pub struct Database {
    pub macros: BTreeMap<String, Macro>,
    /// Programs remain as raw YAML; use [`Self::program_catalog`] for lookups.
    pub programs: Value,
    /// Parsed catalog cache; invalidated when `programs` is replaced via known mutators.
    pub(crate) catalog_cache: RefCell<Option<Arc<ProgramCatalog>>>,
}

impl Database {
    /// Parse (or reuse the cached parse of) `programs` into a [`ProgramCatalog`].
    ///
    /// Returns a shared `Arc` so cache hits are a refcount bump rather than a deep clone;
    /// callers that need an owned, mutable catalog should use `Arc::unwrap_or_clone`.
    pub fn program_catalog(&self) -> Result<Arc<ProgramCatalog>> {
        if let Some(cached) = self.catalog_cache.borrow().as_ref() {
            return Ok(Arc::clone(cached));
        }
        let catalog = Arc::new(ProgramCatalog::from_yaml_value(&self.programs)?);
        *self.catalog_cache.borrow_mut() = Some(Arc::clone(&catalog));
        Ok(catalog)
    }

    fn invalidate_catalog_cache(&mut self) {
        *self.catalog_cache.get_mut() = None;
    }

    /// Replace `programs` from a typed catalog, preserving masks/collections/atlases via merge.
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
        Ok(Self::from_yaml_with_warnings(&text)?.0)
    }

    /// Load default `db.yaml`, returning any per-macro decode/validation warnings.
    pub fn load_default_with_warnings() -> Result<(Self, Vec<String>)> {
        {
            let path = db_path();
            if !path.exists() {
                return Ok((Self::default(), Vec::new()));
            }
            let text = fs::read_to_string(path)?;
            Self::from_yaml_with_warnings(&text)
        }
    }

    pub fn load_default() -> Result<Self> {
        Self::load_from_path(db_path())
    }

    /// Parse `db.yaml` bytes (UTF-8). Used by WASM import and tests.
    pub fn from_yaml_bytes(bytes: &[u8]) -> Result<Self> {
        let text = std::str::from_utf8(bytes)
            .map_err(|e| PersistError::Message(format!("db.yaml is not UTF-8: {e}")))?;
        Ok(Self::from_yaml_with_warnings(text)?.0)
    }

    /// Serialize to YAML bytes (UTF-8). Used by WASM export.
    pub fn to_yaml_bytes(&self) -> Result<Vec<u8>> {
        Ok(self.to_yaml()?.into_bytes())
    }

    pub fn from_yaml(text: &str) -> Result<Self> {
        Ok(Self::from_yaml_with_warnings(text)?.0)
    }

    /// Load `db.yaml`, skipping individual macros that fail to decode so one
    /// corrupt entry cannot lock the user out of the rest of the library.
    ///
    /// Returns `(database, warnings)` where warnings describe skipped macros.
    pub fn from_yaml_with_warnings(text: &str) -> Result<(Self, Vec<String>)> {
        if text.len() > MAX_DB_YAML_BYTES {
            return Err(PersistError::Message(format!(
                "db.yaml too large ({} bytes; max {MAX_DB_YAML_BYTES})",
                text.len()
            )));
        }
        check_yaml_safety(text)?;
        let root: Value = serde_yaml::from_str(text)?;
        check_yaml_nesting_depth(&root)?;
        let mapping = root
            .as_mapping()
            .ok_or_else(|| PersistError::Message("db.yaml root must be a mapping".into()))?;

        let mut macros = BTreeMap::new();
        let mut warnings = Vec::new();
        if let Some(Value::Mapping(mm)) = mapping.get(Value::String("macros".into())) {
            if mm.len() > MAX_MACROS {
                return Err(PersistError::Message(format!(
                    "too many macros ({}; max {MAX_MACROS})",
                    mm.len()
                )));
            }
            for (k, v) in mm {
                let key = match k.as_str() {
                    Some(s) => s.to_string(),
                    None => {
                        warnings.push("skipped macro with non-string key".into());
                        continue;
                    }
                };
                match decode_macro_from_map(v) {
                    Ok(mut macro_) => {
                        if macro_.name.is_empty() {
                            macro_.name = key.clone();
                        }
                        // Structural validation is surfaced in the UI on the macro /
                        // action rows — do not turn it into an app-wide load banner.
                        macros.insert(key, macro_);
                    }
                    Err(e) => {
                        warnings.push(format!("skipped macro \"{key}\": {e}"));
                    }
                }
            }
        }

        let programs = mapping
            .get(Value::String("programs".into()))
            .cloned()
            .unwrap_or(Value::Mapping(Mapping::new()));

        Ok((
            Self {
                macros,
                programs,
                catalog_cache: RefCell::new(None),
            },
            warnings,
        ))
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
