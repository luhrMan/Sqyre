use sqyre_domain::{CoordinateRef, Macro};
use sqyre_executor::{CoordinateResolver, IconStore, ItemMeta, MacroLookup};
use sqyre_persist::ProgramCatalog;
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Arc;

/// Set catalog resolution to the primary monitor size.
/// No-op when capture is unavailable (headless).
pub fn apply_main_monitor_resolution(catalog: &mut ProgramCatalog) {
    if let Some(key) = sqyre_capture::main_monitor_resolution_key() {
        catalog.set_resolution_key(key);
    }
}

pub struct CatalogResolver<'a>(pub &'a ProgramCatalog);

impl CoordinateResolver for CatalogResolver<'_> {
    fn resolve_point(&self, r: &CoordinateRef, macro_: &Macro) -> Result<(i32, i32), String> {
        self.0.resolve_point(r, macro_)
    }

    fn resolve_search_area(
        &self,
        r: &CoordinateRef,
        macro_: &Macro,
    ) -> Result<(i32, i32, i32, i32), String> {
        self.0.resolve_search_area(r, macro_)
    }

    fn collection_grid(&self, program: &str, collection: &str) -> Result<(i32, i32), String> {
        let r = if program.is_empty() {
            CoordinateRef(collection.to_string())
        } else {
            CoordinateRef(format!("{program}~{collection}"))
        };
        let col = self.0.lookup_collection(&r)?;
        Ok((col.rows, col.cols))
    }
}

pub struct CatalogIcons<'a>(pub &'a ProgramCatalog);

impl IconStore for CatalogIcons<'_> {
    fn variant_paths(&self, target: &str) -> Vec<PathBuf> {
        self.0.variant_paths(target)
    }

    fn mask_path(&self, target: &str) -> Option<PathBuf> {
        self.0.mask_path(target)
    }

    fn item_meta(&self, target: &str) -> Option<ItemMeta> {
        self.0
            .item_meta(target)
            .map(|(name, stack_max, cols, rows)| ItemMeta {
                name,
                stack_max,
                cols,
                rows,
            })
    }
}

/// Snapshot of macros available to RunMacro during a run.
pub struct SnapshotMacros(pub Arc<BTreeMap<String, Arc<Macro>>>);

impl MacroLookup for SnapshotMacros {
    fn get(&self, name: &str) -> Option<Arc<Macro>> {
        self.0.get(name).cloned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_yaml::Value;

    fn sample_catalog() -> ProgramCatalog {
        let yaml = r#"
Game:
  name: Game
  items:
    Sword:
      name: Sword
      mask: ""
      stackmax: 1
      gridsize: [1, 1]
  coordinates:
    1920x1080:
      points:
        Spawn:
          name: Spawn
          x: 10
          y: 20
      searchareas:
        Arena:
          name: Arena
          leftx: 0
          topy: 0
          rightx: 100
          bottomy: 80
  collections:
    Bag:
      name: Bag
      searcharea: Arena
      rows: 4
      cols: 5
"#;
        let v: Value = serde_yaml::from_str(yaml).unwrap();
        let mut cat = ProgramCatalog::from_yaml_value(&v).unwrap();
        cat.set_resolution_key("1920x1080");
        cat
    }

    #[test]
    fn catalog_resolver_resolves_point_and_area() {
        let cat = sample_catalog();
        let resolver = CatalogResolver(&cat);
        let m = Macro::new("t", 0, vec![]);
        assert_eq!(
            resolver
                .resolve_point(&CoordinateRef("Game~Spawn".into()), &m)
                .unwrap(),
            (10, 20)
        );
        assert_eq!(
            resolver
                .resolve_search_area(&CoordinateRef("Game~Arena".into()), &m)
                .unwrap(),
            (0, 0, 100, 80)
        );
        assert_eq!(resolver.collection_grid("Game", "Bag").unwrap(), (4, 5));
    }

    #[test]
    fn catalog_icons_expose_item_meta() {
        let cat = sample_catalog();
        let icons = CatalogIcons(&cat);
        let meta = icons.item_meta("Game~Sword").expect("meta");
        assert_eq!(meta.name, "Sword");
        assert_eq!(meta.stack_max, 1);
    }

    #[test]
    fn snapshot_macros_lookup() {
        let mut map = BTreeMap::new();
        map.insert("alpha".into(), Arc::new(Macro::new("alpha", 0, vec![])));
        let snap = SnapshotMacros(Arc::new(map));
        assert!(snap.get("alpha").is_some());
        assert!(snap.get("missing").is_none());
    }
}
