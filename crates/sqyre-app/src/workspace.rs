//! Persistent working copy: database, macros, catalog, and selection.

use std::sync::Arc;

use crate::catalog::apply_main_monitor_resolution;
use crate::macro_meta::MacroMetaUi;
use sqyre_domain::Macro;
use sqyre_persist::{Database, ProgramCatalog};

pub(crate) struct Workspace {
    pub(crate) db: Database,
    /// Mutable source of truth for macro trees; synced into `db` only at save time.
    pub(crate) macros: Vec<Macro>,
    pub(crate) catalog: ProgramCatalog,
    pub(crate) selected_macro: usize,
    pub(crate) load_error: Option<String>,
    /// Non-fatal platform/session advisory (e.g. Wayland without X11).
    pub(crate) platform_warning: Option<String>,
    /// Last failed macro/db save; shown in the macro list until a save succeeds.
    pub(crate) save_error: Option<String>,
    pub(crate) macro_meta: MacroMetaUi,
    /// When set, macros matching any of these tags (empty string = untagged) have hotkeys enabled.
    /// Empty = hotkeys off. Persisted in [`sqyre_persist::RuntimePrefs::hotkey_tag_filters`].
    pub(crate) hotkey_tag_filters: Vec<String>,
}

/// Sync macros/catalog into `db` and write `db.yaml`. The single database-save
/// implementation shared by [`Workspace::persist_database`] and
/// [`Workspace::persist_database_for_editor`] so the two call sites cannot drift.
fn sync_and_save_database(
    db: &mut Database,
    macros: &[Macro],
    catalog: &ProgramCatalog,
) -> Result<(), String> {
    db.set_programs_from_catalog(catalog);
    db.replace_macros(macros.iter().cloned());
    db.save_default().map_err(|e| e.to_string())
}

impl Workspace {
    /// Clamped index of the selected macro, or `None` when the list is empty.
    pub(crate) fn selected_macro_index(&self) -> Option<usize> {
        (!self.macros.is_empty()).then(|| self.selected_macro.min(self.macros.len() - 1))
    }

    /// Sync working macros + catalog into `db` and write `db.yaml`.
    pub(crate) fn persist_database(&mut self) -> Result<(), String> {
        match sync_and_save_database(&mut self.db, &self.macros, &self.catalog) {
            Ok(()) => {
                self.save_error = None;
                Ok(())
            }
            Err(msg) => {
                self.save_error = Some(msg.clone());
                Err(msg)
            }
        }
    }

    /// Data Editor's persist path: the same save implementation as [`Self::persist_database`],
    /// plus catalog-generation continuity the editor's `ListCache` invalidation relies on.
    ///
    /// Takes explicit `db`/`macros`/`catalog` rather than `&mut self` because
    /// `DataEditor::show` (and its `on_new`/`on_update`/`on_delete` helpers) run with those
    /// fields already disjointly borrowed out of `Workspace`, so `&mut Workspace` isn't
    /// available at the call site.
    pub(crate) fn persist_database_for_editor(
        db: &mut Database,
        macros: &[Macro],
        catalog: &mut ProgramCatalog,
    ) -> Result<(), String> {
        let previous_generation = catalog.generation();
        // Runtime layout/scale are not serialized in db.yaml — preserve across reload.
        let saved_rects = catalog.monitor_rects().to_vec();
        let saved_res_key = catalog.resolution_key().to_string();
        let saved_scale = catalog.runtime_scale();
        sync_and_save_database(db, macros, catalog)?;
        *catalog = Arc::unwrap_or_clone(db.program_catalog().map_err(|e| e.to_string())?);
        // YAML reload resets generation to 0; keep ListCache invalidation working.
        catalog.continue_generation_after_reload(previous_generation);
        if !saved_rects.is_empty() {
            catalog.set_monitor_rects(saved_rects);
        }
        if !saved_res_key.is_empty() {
            catalog.set_resolution_key(saved_res_key);
        }
        catalog.set_runtime_scale(saved_scale);
        // Refresh from live layout when available (won't shrink a richer cache).
        apply_main_monitor_resolution(catalog);
        Ok(())
    }
}
