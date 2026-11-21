//! Tests for macro YAML drafts (kept out of the leaf module).

use super::*;
use crate::paths::with_sqyre_dir_override;
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
