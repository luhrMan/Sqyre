//! Program catalog entity types.

use sqyre_domain::{MaskShape, ScalarValue};
use std::collections::BTreeMap;
use std::path::PathBuf;

#[derive(Debug, Clone, Default)]
pub struct ProgramPoint {
    pub name: String,
    pub x: ScalarValue,
    pub y: ScalarValue,
}

#[derive(Debug, Clone, Default)]
pub struct ProgramSearchArea {
    pub name: String,
    pub left_x: ScalarValue,
    pub top_y: ScalarValue,
    pub right_x: ScalarValue,
    pub bottom_y: ScalarValue,
}

#[derive(Debug, Clone, Default)]
pub struct ProgramItem {
    pub name: String,
    pub mask: String,
    pub stack_max: i32,
    pub grid_cols: i32,
    pub grid_rows: i32,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct ProgramMask {
    pub name: String,
    /// Rectangle or circle overlay geometry.
    pub shape: MaskShape,
    /// Percent of template width (literal or `${var}` expression).
    pub center_x: String,
    pub center_y: String,
    pub base: String,
    pub height: String,
    pub radius: String,
    pub inverse: bool,
}

impl Default for ProgramMask {
    fn default() -> Self {
        Self {
            name: String::new(),
            shape: MaskShape::Rectangle,
            center_x: "50".into(),
            center_y: "50".into(),
            base: String::new(),
            height: String::new(),
            radius: String::new(),
            inverse: false,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct ProgramCollection {
    pub name: String,
    /// Search area name in the same program.
    pub search_area: String,
    pub rows: i32,
    pub cols: i32,
}

#[derive(Debug, Clone, Default)]
pub struct ProgramData {
    pub name: String,
    /// Absolute executable path of the bound OS process (from a running-window pick).
    /// Empty = no binding; overlay falls back to fuzzy name match.
    pub process_path: String,
    /// Window title captured with the process pick.
    /// With `process_path`, overlay + Focus Window require both (disambiguates shared exes).
    pub window_title: String,
    /// resolution key → points
    pub points: BTreeMap<String, BTreeMap<String, ProgramPoint>>,
    pub search_areas: BTreeMap<String, BTreeMap<String, ProgramSearchArea>>,
    pub items: BTreeMap<String, ProgramItem>,
    pub masks: BTreeMap<String, ProgramMask>,
    pub collections: BTreeMap<String, ProgramCollection>,
}

#[derive(Debug, Clone, Default)]
pub struct ProgramCatalog {
    pub(super) programs: BTreeMap<String, ProgramData>,
    /// Override for tests; empty → `images_path()`.
    pub(super) images_root: Option<PathBuf>,
    /// Main monitor resolution key. Empty → first key found.
    pub(super) resolution_key: String,
    /// Bumped on structural mutations; UI caches key off this.
    pub(super) generation: u64,
}
