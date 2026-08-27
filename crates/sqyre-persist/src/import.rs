//! Merge two [`Database`] values preferring the imported side on conflicts.

use crate::{Database, PersistError, Result};
use serde_yaml::{Mapping, Value};

/// How a backup import applies an archive to the live data.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImportMode {
    /// Replace the live data tree with the archive.
    Overwrite,
    /// Merge macros/programs/assets preferring the archive; replace `settings.yaml`.
    Merge,
}

/// Deep-merge `imported` into `live`. Same-name macros and program entities take
/// the imported value; live-only entries are kept.
pub fn merge_databases_prefer_imported(live: &Database, imported: &Database) -> Result<Database> {
    let live_cat = live.program_catalog().map_err(|e| {
        PersistError::Message(format!("live db programs invalid during merge: {e}"))
    })?;
    let imported_cat = imported.program_catalog().map_err(|e| {
        PersistError::Message(format!("imported db programs invalid during merge: {e}"))
    })?;
    let merged_cat = live_cat.merge_prefer_imported(&imported_cat);

    let mut macros = live.macros.clone();
    for (name, macro_) in &imported.macros {
        macros.insert(name.clone(), macro_.clone());
    }

    let mut out = Database {
        macros,
        programs: merge_programs_yaml(&live.programs, &imported.programs),
        catalog_cache: std::cell::RefCell::new(None),
    };
    out.set_programs_from_catalog(&merged_cat);
    Ok(out)
}

/// Overlay imported program YAML keys onto live (imported wins on clash).
fn merge_programs_yaml(live: &Value, imported: &Value) -> Value {
    let mut out = match live {
        Value::Mapping(m) => m.clone(),
        _ => Mapping::new(),
    };
    if let Value::Mapping(imp) = imported {
        for (k, v) in imp {
            out.insert(k.clone(), v.clone());
        }
    }
    Value::Mapping(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqyre_domain::{Macro, ScalarValue};

    fn db_with_macro(name: &str, delay: i32) -> Database {
        let m = Macro::new(name, delay, Vec::new());
        let mut db = Database::default();
        db.macros.insert(name.into(), m);
        db
    }

    #[test]
    fn macros_prefer_imported_and_keep_live_only() {
        let live = db_with_macro("A", 10);
        let mut imported = db_with_macro("A", 99);
        imported
            .macros
            .insert("B".into(), Macro::new("B", 0, Vec::new()));

        let merged = merge_databases_prefer_imported(&live, &imported).unwrap();
        assert_eq!(merged.macros["A"].global_delay, 99);
        assert!(merged.macros.contains_key("B"));
    }

    #[test]
    fn programs_merge_nested_prefer_imported() {
        let live = Database::from_yaml(
            r#"
macros: {}
programs:
  Game:
    items: {}
    processpath: /live/game
    coordinates:
      1920x1080:
        points:
          Spawn: { x: 1, y: 2 }
          Keep: { x: 3, y: 4 }
"#,
        )
        .unwrap();
        let imported = Database::from_yaml(
            r#"
macros: {}
programs:
  Game:
    items: {}
    processpath: /imported/game
    coordinates:
      1920x1080:
        points:
          Spawn: { x: 9, y: 9 }
          New: { x: 5, y: 6 }
  Other:
    items: {}
"#,
        )
        .unwrap();

        let merged = merge_databases_prefer_imported(&live, &imported).unwrap();
        let cat = merged.program_catalog().unwrap();
        let game = cat.get("Game").unwrap();
        assert_eq!(game.process_path, "/imported/game");
        assert_eq!(game.points["1920x1080"]["Spawn"].x, ScalarValue::Int(9));
        assert!(game.points["1920x1080"].contains_key("Keep"));
        assert!(game.points["1920x1080"].contains_key("New"));
        assert!(cat.get("Other").is_some());
    }
}
