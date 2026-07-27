//! Persistent working copy: database, macros, catalog, and selection.

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
    /// When set, only macros with this tag (empty string = untagged) have hotkeys enabled.
    pub(crate) hotkey_tag_filter: Option<String>,
}
