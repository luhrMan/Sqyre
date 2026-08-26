//! Shared borrow bundles for UI painters (avoids long argument lists).

use crate::hotkey_record::HotkeyRecordUi;
use crate::icon_cache::IconCache;
use crate::key_record::KeyRecordUi;
use crate::preview_tooltip::PreviewTooltipCache;
use sqyre_domain::{ActionId, KnownVariableNames};
use sqyre_hotkeys::{MacroHotkeyBridge, ScreenClickBridge};
use sqyre_persist::ProgramCatalog;
use sqyre_ports::HighlightSnapshot;
use std::collections::HashMap;

pub struct CatalogPaint<'a> {
    pub catalog: &'a ProgramCatalog,
    pub icons: &'a mut IconCache,
    pub previews: &'a mut PreviewTooltipCache,
    /// When true, Image Search tooltips overlay live template matches on the preview.
    pub image_search_tooltip_preview: bool,
}

#[derive(Clone, Copy)]
pub struct VarTheme<'a> {
    pub known_vars: &'a KnownVariableNames,
    pub is_dark: bool,
}

pub struct RecordBridges<'a> {
    pub key_record: &'a mut KeyRecordUi,
    pub hotkey_record: &'a mut HotkeyRecordUi,
    pub macro_hotkeys: &'a MacroHotkeyBridge,
    pub screen_click: &'a ScreenClickBridge,
}

pub struct TreePaint<'a> {
    pub catalog: &'a ProgramCatalog,
    pub icons: &'a mut IconCache,
    pub theme: VarTheme<'a>,
    pub active_macro: &'a sqyre_domain::Macro,
    pub macro_name: &'a str,
    pub hl_snap: &'a HighlightSnapshot,
    /// Currently selected tree nodes (action or Else folder sentinels).
    pub selected: &'a [ActionId],
    /// Primary selected action when it is a real node (not an Else folder sentinel).
    pub selected_action: Option<&'a sqyre_domain::Action>,
    /// Per-frame summary-pill cache (keyed by action id; filled on demand).
    pub pills_cache: &'a mut HashMap<ActionId, (u64, Vec<sqyre_ui_model::SummaryPill>)>,
    pub paint_revision: u64,
    /// Show per-row Logs buttons (user "Log Meta Images" setting).
    pub show_logs: bool,
}

/// Catalog paint + var theme + recording bridges (action tooltip / defaults edit).
pub struct TipUiCtx<'a> {
    pub paint: CatalogPaint<'a>,
    pub theme: VarTheme<'a>,
    pub bridges: RecordBridges<'a>,
    pub compact_program_headers: bool,
}

/// Bundled args for [`crate::action_tooltip::edit::paint_edit_fields`].
pub struct EditFieldsCtx<'a> {
    pub paint: CatalogPaint<'a>,
    pub bridges: RecordBridges<'a>,
    pub theme: VarTheme<'a>,
    pub macros: &'a [(String, Vec<String>)],
    pub active_macro: Option<&'a sqyre_domain::Macro>,
}
