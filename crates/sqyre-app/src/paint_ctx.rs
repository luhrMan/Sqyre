//! Shared borrow bundles for UI painters (avoids long argument lists).

use crate::hotkey_record::HotkeyRecordUi;
use crate::icon_cache::IconCache;
use crate::key_record::KeyRecordUi;
use crate::preview_tooltip::PreviewTooltipCache;
use sqyre_domain::ActionId;
use sqyre_executor::HighlightSnapshot;
use sqyre_hotkeys::{MacroHotkeyBridge, ScreenClickBridge};
use sqyre_persist::ProgramCatalog;
use std::collections::HashSet;

pub struct CatalogPaint<'a> {
    pub catalog: &'a ProgramCatalog,
    pub icons: &'a mut IconCache,
    pub previews: &'a mut PreviewTooltipCache,
}

#[derive(Clone, Copy)]
pub struct VarTheme<'a> {
    pub known_vars: &'a HashSet<String>,
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
    pub macro_name: &'a str,
    pub hl_snap: &'a HighlightSnapshot,
    /// Currently selected tree node (action or Else folder sentinel).
    pub selected: Option<ActionId>,
}

/// Catalog paint + var theme + recording bridges (action tooltip / defaults edit).
pub struct TipUiCtx<'a> {
    pub paint: CatalogPaint<'a>,
    pub theme: VarTheme<'a>,
    pub bridges: RecordBridges<'a>,
}

/// Bundled args for [`crate::action_tooltip::edit::paint_edit_fields`].
pub struct EditFieldsCtx<'a> {
    pub paint: CatalogPaint<'a>,
    pub bridges: RecordBridges<'a>,
    pub theme: VarTheme<'a>,
    pub macros: &'a [(String, Vec<String>)],
    pub active_macro: Option<&'a sqyre_domain::Macro>,
}
