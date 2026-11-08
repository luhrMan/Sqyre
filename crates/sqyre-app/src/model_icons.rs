//! Canonical Phosphor glyph per model — one icon per entity kind, app-wide.
//!
//! Field labels, picker buttons, Data Editor tabs, and list rows must all use
//! these consts so the same model reads as the same icon everywhere.

use egui_phosphor::regular;

/// Catalog program (and the default stand-in for a bound app).
pub(crate) const PROGRAM: &str = regular::APP_WINDOW;
/// Running OS window (window picker, Focus Window target).
pub(crate) const WINDOW: &str = regular::APP_WINDOW;
/// Catalog item (image-search target).
pub(crate) const ITEM: &str = regular::IMAGE;
/// Image mask (shape or PNG that excludes part of a search area).
pub(crate) const MASK: &str = regular::SELECTION_INVERSE;
/// Catalog point (monitor + X/Y).
pub(crate) const POINT: &str = regular::MAP_PIN;
/// Catalog search area (monitor + LTRB rect).
pub(crate) const SEARCH_AREA: &str = regular::SELECTION;
/// Collection (rows × cols grid over a search area).
pub(crate) const COLLECTION: &str = regular::GRID_FOUR;
/// Atlas (ordered set of collections for navigation) — the world the
/// collections map out.
pub(crate) const ATLAS: &str = regular::GLOBE;
/// Floating overlay button.
pub(crate) const OVERLAY: &str = regular::CURSOR_CLICK;
/// Macro (run-macro target, macro pickers).
pub(crate) const MACRO: &str = regular::MAGIC_WAND;
