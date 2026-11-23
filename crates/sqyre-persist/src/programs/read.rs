//! Read-only view of [`ProgramCatalog`] for UI / paint / resolve paths.
//!
//! Does not expose mutate APIs (`upsert_*`, `delete_*`, `programs_mut`, …) or
//! side-effecting helpers like `clear_process_icon`.

use super::{
    MonitorRect, ProgramAtlas, ProgramCatalog, ProgramCollection, ProgramData, ProgramPoint,
    ProgramSearchArea,
};
use sqyre_domain::{CoordinateRef, Macro};
use sqyre_ports::{CollectionArea, ItemMeta, PortError};
use std::path::PathBuf;

/// Immutable borrow of a [`ProgramCatalog`] with only read/resolve methods.
#[derive(Clone, Copy, Debug)]
pub struct CatalogView<'a>(pub &'a ProgramCatalog);

impl<'a> CatalogView<'a> {
    #[inline]
    pub fn new(catalog: &'a ProgramCatalog) -> Self {
        Self(catalog)
    }

    #[inline]
    pub fn inner(self) -> &'a ProgramCatalog {
        self.0
    }

    pub fn get(self, name: &str) -> Option<&'a ProgramData> {
        self.0.get(name)
    }

    pub fn program_names(self) -> impl Iterator<Item = &'a String> {
        self.0.program_names()
    }

    pub fn program_macro_tags(self, program: &str) -> Vec<String> {
        self.0.program_macro_tags(program)
    }

    pub fn generation(self) -> u64 {
        self.0.generation()
    }

    pub fn resolution_key(self) -> &'a str {
        self.0.resolution_key()
    }

    pub fn runtime_scale(self) -> f32 {
        self.0.runtime_scale()
    }

    pub fn monitor_rects(self) -> &'a [MonitorRect] {
        self.0.monitor_rects()
    }

    pub fn icons_dir(self, program: &str) -> PathBuf {
        self.0.icons_dir(program)
    }

    pub fn process_icons_dir(self) -> PathBuf {
        self.0.process_icons_dir()
    }

    pub fn process_icon_path(self, program: &str) -> PathBuf {
        self.0.process_icon_path(program)
    }

    pub fn masks_dir(self, program: &str) -> PathBuf {
        self.0.masks_dir(program)
    }

    pub fn collections_dir(self, program: &str) -> PathBuf {
        self.0.collections_dir(program)
    }

    pub fn collection_image_path(self, program: &str, collection: &str) -> PathBuf {
        self.0.collection_image_path(program, collection)
    }

    pub fn mask_image_path(self, program: &str, mask: &str) -> PathBuf {
        self.0.mask_image_path(program, mask)
    }

    pub fn lookup_point(
        self,
        r: &CoordinateRef,
        resolution_key: &str,
    ) -> Result<&'a ProgramPoint, PortError> {
        self.0.lookup_point(r, resolution_key)
    }

    pub fn lookup_search_area(
        self,
        r: &CoordinateRef,
        resolution_key: &str,
    ) -> Result<&'a ProgramSearchArea, PortError> {
        self.0.lookup_search_area(r, resolution_key)
    }

    pub fn lookup_collection(self, r: &CoordinateRef) -> Result<&'a ProgramCollection, PortError> {
        self.0.lookup_collection(r)
    }

    pub fn lookup_atlas(self, program: &str, name: &str) -> Result<&'a ProgramAtlas, PortError> {
        self.0.lookup_atlas(program, name)
    }

    pub fn resolve_overlay_xy(self, point: &str, fallback: (f32, f32)) -> (f32, f32) {
        self.0.resolve_overlay_xy(point, fallback)
    }

    pub fn resolve_point(self, r: &CoordinateRef, macro_: &Macro) -> Result<(i32, i32), PortError> {
        self.0.resolve_point(r, macro_)
    }

    pub fn resolve_search_area(
        self,
        r: &CoordinateRef,
        macro_: &Macro,
    ) -> Result<(i32, i32, i32, i32), PortError> {
        self.0.resolve_search_area(r, macro_)
    }

    pub fn resolve_collection_area(
        self,
        r: &CoordinateRef,
        macro_: &Macro,
    ) -> Result<CollectionArea, PortError> {
        self.0.resolve_collection_area(r, macro_)
    }

    pub fn variant_paths(self, target: &str) -> Vec<PathBuf> {
        self.0.variant_paths(target)
    }

    pub fn mask_path(self, target: &str) -> Option<PathBuf> {
        self.0.mask_path(target)
    }

    pub fn item_meta(self, target: &str) -> Option<ItemMeta> {
        self.0.item_meta(target)
    }
}

impl<'a> From<&'a ProgramCatalog> for CatalogView<'a> {
    fn from(catalog: &'a ProgramCatalog) -> Self {
        Self::new(catalog)
    }
}
