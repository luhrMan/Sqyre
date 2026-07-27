//! Domain-coupled ports shared by catalog adapters and the macro executor.

use crate::{ItemMeta, PortError};
use sqyre_domain::{CoordinateRef, Macro};
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

/// Resolve `program~point` / search-area refs using the loaded program catalog.
pub trait CoordinateResolver {
    fn resolve_point(&self, r: &CoordinateRef, macro_: &Macro) -> Result<(i32, i32), PortError>;
    fn resolve_search_area(
        &self,
        r: &CoordinateRef,
        macro_: &Macro,
    ) -> Result<(i32, i32, i32, i32), PortError>;

    /// Collection grid size `(rows, cols)` for `program` + collection name.
    fn collection_grid(&self, program: &str, collection: &str) -> Result<(i32, i32), PortError> {
        let _ = (program, collection);
        Err(PortError::not_configured("collection grid lookup"))
    }

    /// Member Collection names for `program` + atlas name.
    fn atlas_members(&self, program: &str, atlas: &str) -> Result<Vec<String>, PortError> {
        let _ = (program, atlas);
        Err(PortError::not_configured("atlas lookup"))
    }
}

/// Resolve image-search targets to on-disk icon / mask paths.
pub trait IconStore {
    /// Variant icon paths for `program~item` (may be empty).
    fn variant_paths(&self, target: &str) -> Vec<std::path::PathBuf>;
    /// Optional mask PNG for the item (resized by caller).
    fn mask_path(&self, target: &str) -> Option<std::path::PathBuf>;
    fn item_meta(&self, target: &str) -> Option<ItemMeta>;
}

/// Look up another macro by name.
pub trait MacroLookup: Send + Sync {
    fn get(&self, name: &str) -> Option<Arc<Macro>>;
}

/// Block until the user presses a continue chord.
pub trait ContinueKeyWaiter: Send + Sync {
    fn wait_for_continue(
        &self,
        keys: &[String],
        pass_through: bool,
        stop: &AtomicBool,
    ) -> Result<(), PortError>;

    /// Wait until one of `chords` is pressed. Returns the matched index.
    fn wait_for_any_chord(
        &self,
        chords: &[Vec<String>],
        hold_repeat: &[bool],
        pass_through: bool,
        stop: &AtomicBool,
    ) -> Result<usize, PortError> {
        let _ = hold_repeat;
        if chords.is_empty() {
            return Err(PortError::invalid("key wait: no chords configured"));
        }
        self.wait_for_continue(&chords[0], pass_through, stop)?;
        Ok(0)
    }
}
