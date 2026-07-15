use sqyre_domain::{CoordinateRef, Macro};
use sqyre_executor::{CoordinateResolver, IconStore, ItemMeta, MacroLookup};
use sqyre_persist::ProgramCatalog;
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Arc;

/// Set catalog resolution to the primary monitor size (Go `MainMonitorSizeString`).
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
pub struct SnapshotMacros(pub Arc<BTreeMap<String, Macro>>);

impl MacroLookup for SnapshotMacros {
    fn get(&self, name: &str) -> Option<Macro> {
        self.0.get(name).cloned()
    }
}
