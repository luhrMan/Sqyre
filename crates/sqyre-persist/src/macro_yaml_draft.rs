//! Per-macro unapplied YAML drafts for the YAML Macro Builder.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use crate::{PersistError, Result};

const FILE_NAME: &str = "macro_yaml_builder.yaml";
const MAX_BYTES: usize = 2 * 1024 * 1024;

/// Absolute path to the YAML Macro Builder draft store.
pub fn macro_yaml_builder_path() -> PathBuf {
    crate::sqyre_dir().join(FILE_NAME)
}

/// One unapplied draft keyed by macro name.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct MacroYamlDraftEntry {
    /// Canonical YAML hash/content when the draft was started.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub base_yaml: String,
    /// Editor contents.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub yaml: String,
}

/// Persisted drafts for the YAML Macro Builder (`~/.sqyre/macro_yaml_builder.yaml`).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct MacroYamlBuilderDrafts {
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub drafts: BTreeMap<String, MacroYamlDraftEntry>,
}

impl MacroYamlBuilderDrafts {
    pub fn load_default() -> Result<Self> {
        #[cfg(target_arch = "wasm32")]
        {
            return Ok(Self::default());
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            Self::load_from_path(macro_yaml_builder_path())
        }
    }

    pub fn load_from_path(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        if !path.exists() {
            return Ok(Self::default());
        }
        let text = fs::read_to_string(path)?;
        if text.len() > MAX_BYTES {
            return Err(PersistError::Message(format!(
                "{FILE_NAME} too large ({} bytes; max {MAX_BYTES})",
                text.len()
            )));
        }
        Ok(serde_yaml::from_str(&text)?)
    }

    pub fn save_default(&self) -> Result<()> {
        #[cfg(target_arch = "wasm32")]
        {
            let _ = self;
            return Ok(());
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            self.save_to_path(macro_yaml_builder_path())
        }
    }

    pub fn save_to_path(&self, path: impl AsRef<Path>) -> Result<()> {
        let path = path.as_ref();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        if self.drafts.is_empty() {
            if path.exists() {
                fs::remove_file(path)?;
            }
            return Ok(());
        }
        crate::atomic_write(path, serde_yaml::to_string(self)?)?;
        Ok(())
    }

    pub fn get(&self, macro_name: &str) -> Option<&MacroYamlDraftEntry> {
        self.drafts.get(macro_name)
    }

    pub fn set(&mut self, macro_name: impl Into<String>, entry: MacroYamlDraftEntry) {
        let name = macro_name.into();
        if entry.yaml.is_empty() || entry.yaml == entry.base_yaml {
            self.drafts.remove(&name);
        } else {
            self.drafts.insert(name, entry);
        }
    }

    pub fn remove(&mut self, macro_name: &str) {
        self.drafts.remove(macro_name);
    }

    pub fn rename(&mut self, old: &str, new: &str) {
        if old == new {
            return;
        }
        if let Some(entry) = self.drafts.remove(old) {
            self.drafts.insert(new.to_string(), entry);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::with_sqyre_dir_override;
    use tempfile::tempdir;

    #[test]
    fn roundtrip_and_clear_removes_file() {
        let dir = tempdir().unwrap();
        with_sqyre_dir_override(dir.path().to_path_buf(), || {
            let mut store = MacroYamlBuilderDrafts::default();
            store.set(
                "demo",
                MacroYamlDraftEntry {
                    base_yaml: "name: demo\n".into(),
                    yaml: "name: demo\nroot:\n  type: loop\n".into(),
                },
            );
            store.save_default().unwrap();
            let loaded = MacroYamlBuilderDrafts::load_default().unwrap();
            assert_eq!(loaded, store);

            store.remove("demo");
            store.save_default().unwrap();
            assert!(!macro_yaml_builder_path().exists());
        });
    }
}
