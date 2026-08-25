//! Golden legacy `db.yaml` through [`sqyre_persist::migrate_db_yaml`].

use sqyre_persist::{migrate_db_yaml, Database};
use std::path::PathBuf;

#[test]
fn legacy_inline_fixture_migrates_and_loads() {
    let path =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/db/legacy_inline.yaml");
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| {
        panic!("read {}: {e}", path.display());
    });
    let out = migrate_db_yaml(&text).expect("migrate");
    let db = Database::from_yaml(&out).expect("load migrated yaml");
    assert!(db.macros.contains_key("Demo"));
    sqyre_validate::validate_macro(&db.macros["Demo"]).expect("validate migrated Demo");
}
