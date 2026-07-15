use sqyre_domain::{CoordinateRef, Macro};
use sqyre_executor::{CoordinateResolver, IconStore, ItemMeta};
use sqyre_persist::ProgramCatalog;
use std::path::PathBuf;

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
