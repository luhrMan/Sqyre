//! Shared entity pickers: item icon grids, point / search-area lists, collection cells,
//! macros, and Focus Window live-window lists.

use crate::data_editor_preview::{paint_grid_overlay_painter, show_file_hover};
use crate::icon_cache::IconCache;
use crate::image_view::{self, ImageViewTransform};
use crate::paint_ctx::CatalogPaint;
use crate::preview_tooltip::PreviewKind;
use eframe::egui::{self, Color32, Pos2, Sense, Vec2};
use crate::window_types::WindowInfo;
use sqyre_domain::{CoordinateRef, PROGRAM_DELIMITER};
use sqyre_persist::ProgramCatalog;
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::thread;

/// Fixed cell size (thumb + padding; no under-icon label).
pub(crate) const GRID_CELL: f32 = 64.0;
pub(crate) const GRID_THUMB: f32 = 52.0;
pub(crate) const GRID_GAP: f32 = 6.0;
/// Compact cells for Image Search tip edit (remove badge overlay).
pub(crate) const EDIT_CELL: f32 = 40.0;
pub(crate) const EDIT_THUMB: f32 = 36.0;
pub(crate) const EDIT_GAP: f32 = 4.0;
pub(crate) const REMOVE_BTN: f32 = 12.0;
pub(crate) const HEADER_SIZE: f32 = 16.0;
/// In-progress collection cell selection (1-based inclusive).
#[derive(Debug, Clone)]
pub struct CollectionCellPick {
    pub program: String,
    pub collection: String,
    pub rows: i32,
    pub cols: i32,
    /// Current selection; `None` until the user clicks.
    pub sel: Option<(i32, i32, i32, i32)>,
    /// Drag start cell while pointer is down (selection mode).
    pub(crate) drag_anchor: Option<(i32, i32)>,
    pub(crate) view: ImageViewTransform,
}

impl CollectionCellPick {
    pub fn new(
        program: impl Into<String>,
        collection: impl Into<String>,
        rows: i32,
        cols: i32,
    ) -> Self {
        Self {
            program: program.into(),
            collection: collection.into(),
            rows: rows.max(1),
            cols: cols.max(1),
            sel: None,
            drag_anchor: None,
            view: ImageViewTransform::default(),
        }
    }

    pub fn with_initial_sel(mut self, sel: Option<(i32, i32, i32, i32)>) -> Self {
        self.sel = sel;
        self
    }

    pub fn reset_view(&mut self) {
        self.view.reset();
    }

    pub fn to_ref(&self) -> Option<CoordinateRef> {
        let (r1, c1, r2, c2) = self.sel?;
        Some(CoordinateRef::collection(
            &self.program,
            &self.collection,
            r1,
            c1,
            r2,
            c2,
        ))
    }
}

/// Which modal picker is open from an action edit tip (or similar).
#[derive(Debug, Default)]
pub enum ActivePicker {
    #[default]
    None,
    /// Multi-select item targets (`program~item`).
    Items { search: String, staged: Vec<String> },
    /// Point or search-area coordinate picker (`kind` selects catalog + result).
    Coord {
        kind: CoordKind,
        search: String,
        /// Working value shown/edited in the picker.
        value: String,
        /// When set, list is replaced by the collection cell grid.
        cell_pick: Option<CollectionCellPick>,
        /// Scroll selected row into view once on open.
        scroll_to_selection: bool,
    },
    Macro {
        search: String,
        value: String,
        scroll_to_selection: bool,
    },
    /// Live OS windows for Focus Window (`process_path` + `window_title`).
    Window {
        search: String,
        process_path: String,
        window_title: String,
        windows: Vec<WindowInfo>,
        load_error: Option<String>,
        scroll_to_selection: bool,
        /// Background `list_open_windows` result; polled each frame while open.
        pending: Option<Receiver<Result<Vec<WindowInfo>, String>>>,
    },
}

impl Clone for ActivePicker {
    fn clone(&self) -> Self {
        match self {
            ActivePicker::None => ActivePicker::None,
            ActivePicker::Items { search, staged } => ActivePicker::Items {
                search: search.clone(),
                staged: staged.clone(),
            },
            ActivePicker::Coord {
                kind,
                search,
                value,
                cell_pick,
                scroll_to_selection,
            } => ActivePicker::Coord {
                kind: *kind,
                search: search.clone(),
                value: value.clone(),
                cell_pick: cell_pick.clone(),
                scroll_to_selection: *scroll_to_selection,
            },
            ActivePicker::Macro {
                search,
                value,
                scroll_to_selection,
            } => ActivePicker::Macro {
                search: search.clone(),
                value: value.clone(),
                scroll_to_selection: *scroll_to_selection,
            },
            ActivePicker::Window {
                search,
                process_path,
                window_title,
                windows,
                load_error,
                scroll_to_selection,
                pending: _,
            } => ActivePicker::Window {
                search: search.clone(),
                process_path: process_path.clone(),
                window_title: window_title.clone(),
                windows: windows.clone(),
                load_error: load_error.clone(),
                scroll_to_selection: *scroll_to_selection,
                // In-flight loads are not cloned.
                pending: None,
            },
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum CoordKind {
    Point,
    SearchArea,
}

#[derive(Debug, Clone)]
pub enum PickerResult {
    None,
    Items(Vec<String>),
    Point(CoordinateRef),
    SearchArea(CoordinateRef),
    MacroName(String),
    Window { process_path: String, window_title: String },
}

impl ActivePicker {
    pub fn is_open(&self) -> bool {
        !matches!(self, Self::None)
    }

    pub(crate) fn cell_pick_mut(&mut self) -> Option<&mut Option<CollectionCellPick>> {
        match self {
            Self::Coord { cell_pick, .. } => Some(cell_pick),
            _ => None,
        }
    }

    pub(crate) fn coord_kind(&self) -> Option<CoordKind> {
        match self {
            Self::Coord { kind, .. } => Some(*kind),
            _ => None,
        }
    }
}

