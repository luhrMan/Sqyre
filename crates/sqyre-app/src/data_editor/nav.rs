//! Navigation, selection, and left-list cache for the Data Editor.

use super::tabs::{EditorTab, SearchScope};
use std::collections::HashMap;

/// Cached left-list / program-selector data; invalidated via catalog generation.
#[derive(Debug, Clone, Default)]
pub(crate) struct ListCache {
    pub catalog_generation: u64,
    pub resolution_key: String,
    pub tab: EditorTab,
    pub program_names: Vec<String>,
    pub entities_by_program: HashMap<String, Vec<String>>,
}

/// Window open state, active tab, search/scope, selection, and list cache.
#[derive(Debug)]
pub struct EditorNav {
    pub open: bool,
    pub(crate) tab: EditorTab,
    /// Header search query — applies to the left list while `search_open`,
    /// and is preserved (unfiltered) while the field is hidden.
    pub(crate) search: String,
    /// Header search field is expanded (magnifier toggle).
    pub(crate) search_open: bool,
    /// Header program selector is expanded (program-icon toggle). Exclusive
    /// with `search_open` — the two pairs share the same header slot.
    pub(crate) program_open: bool,
    /// Left-list scope for entity tabs (flat selected program vs all programs).
    pub(crate) search_scope: SearchScope,
    /// Draft for the Items / Match Check tag-priority chip editor (ephemeral).
    pub(crate) items_tag_priority_draft: String,
    /// Width of the left list pane (drag-adjustable).
    pub(crate) left_width: f32,
    /// Set when the pane divider was moved via arrow keys; persisted on key release.
    pub(crate) split_key_moved: bool,
    /// Selected program name (all tabs).
    pub(crate) selected_program: Option<String>,
    /// Focused catalog program (from overlay last-foreign / active window).
    /// When set, New entity actions target this program.
    pub(crate) focused_program_hint: Option<String>,
    /// Selected entity within program (items / points / search areas).
    pub(crate) selected_entity: Option<String>,
    /// Expand + scroll the left list to [`Self::selected_program`] (armed on tab switch).
    pub(crate) scroll_left_list_to_selection: bool,
    /// Cached program/entity name lists keyed by catalog generation.
    pub(crate) list_cache: ListCache,
}

impl Default for EditorNav {
    fn default() -> Self {
        Self {
            open: false,
            tab: EditorTab::Programs,
            search: String::new(),
            search_open: false,
            program_open: false,
            search_scope: SearchScope::default(),
            items_tag_priority_draft: String::new(),
            left_width: 0.0,
            split_key_moved: false,
            selected_program: None,
            focused_program_hint: None,
            selected_entity: None,
            scroll_left_list_to_selection: false,
            list_cache: ListCache::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_nav_starts_closed_on_programs() {
        let nav = EditorNav::default();
        assert!(!nav.open);
        assert_eq!(nav.tab, EditorTab::Programs);
        assert!(nav.selected_program.is_none());
        assert!(nav.selected_entity.is_none());
        assert!(nav.list_cache.program_names.is_empty());
    }
}
