//! Program catalog entity types.

use sqyre_domain::{MaskShape, ScalarValue};
use std::collections::BTreeMap;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct ProgramPoint {
    pub name: String,
    /// 1-based logical monitor slot; coords are relative to that monitor's origin.
    pub monitor: u32,
    pub x: ScalarValue,
    pub y: ScalarValue,
}

impl Default for ProgramPoint {
    fn default() -> Self {
        Self {
            name: String::new(),
            monitor: 1,
            x: ScalarValue::default(),
            y: ScalarValue::default(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ProgramSearchArea {
    pub name: String,
    /// 1-based logical monitor slot; corners are relative to that monitor's origin.
    pub monitor: u32,
    pub left_x: ScalarValue,
    pub top_y: ScalarValue,
    pub right_x: ScalarValue,
    pub bottom_y: ScalarValue,
}

impl Default for ProgramSearchArea {
    fn default() -> Self {
        Self {
            name: String::new(),
            monitor: 1,
            left_x: ScalarValue::default(),
            top_y: ScalarValue::default(),
            right_x: ScalarValue::default(),
            bottom_y: ScalarValue::default(),
        }
    }
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

/// Named group of Collections navigated together by Navigate Select.
#[derive(Debug, Clone, Default)]
pub struct ProgramAtlas {
    pub name: String,
    /// Member Collection names in this program.
    pub collections: Vec<String>,
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
    /// resolution key → DPI scale (`dpi/96`) stamped when the bucket was first written.
    pub coord_scales: BTreeMap<String, f32>,
    pub items: BTreeMap<String, ProgramItem>,
    pub masks: BTreeMap<String, ProgramMask>,
    pub collections: BTreeMap<String, ProgramCollection>,
    pub atlases: BTreeMap<String, ProgramAtlas>,
}

/// One display in virtual-desktop coordinates (`x`, `y`, `w`, `h`), sorted slot order.
pub type MonitorRect = (i32, i32, i32, i32);

#[derive(Debug, Clone)]
pub struct ProgramCatalog {
    pub(super) programs: BTreeMap<String, ProgramData>,
    /// Override for tests; empty → `images_path()`.
    pub(super) images_root: Option<PathBuf>,
    /// Main monitor resolution key. Empty → first key found.
    pub(super) resolution_key: String,
    /// Live primary-monitor DPI scale (`dpi/96`). Used when remapping stored coords.
    pub(super) runtime_scale: f32,
    /// Live monitor layout for resolving slot-relative coords to absolute desktop pixels.
    /// Empty → treat as a single `(0,0,1920,1080)` slot for headless / unset layouts.
    pub(super) monitor_rects: Vec<MonitorRect>,
    /// Bumped on structural mutations; UI caches key off this.
    pub(super) generation: u64,
}

impl Default for ProgramCatalog {
    fn default() -> Self {
        Self {
            programs: BTreeMap::new(),
            images_root: None,
            resolution_key: String::new(),
            runtime_scale: 1.0,
            monitor_rects: Vec::new(),
            generation: 0,
        }
    }
}
