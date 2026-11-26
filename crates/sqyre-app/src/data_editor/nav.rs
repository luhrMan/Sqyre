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
    /// Pointer is mid-drag on the pane divider — skip re-deriving width from
    /// settings until `drag_stopped` writes the settled fraction.
    pub(crate) split_gesture_active: bool,
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
            split_gesture_active: false,
            split_key_moved: false,
            selected_program: None,
            focused_program_hint: None,
            selected_entity: None,
            scroll_left_list_to_selection: false,
            list_cache: ListCache::default(),
        }
    }
}

impl EditorNav {
    /// Derive the left-pane pixel width from a settings fraction and viewport.
    pub(crate) fn left_width_from_frac(
        avail_w: f32,
        frac: f32,
        min_frac: f32,
        max_frac: f32,
    ) -> f32 {
        let frac = frac.clamp(min_frac, max_frac);
        let min_left = avail_w * min_frac;
        let max_left = avail_w * max_frac;
        (avail_w * frac).clamp(min_left, max_left)
    }

    /// Settings fraction implied by the current left width.
    pub(crate) fn frac_from_left_width(
        avail_w: f32,
        left_width: f32,
        min_frac: f32,
        max_frac: f32,
    ) -> f32 {
        if avail_w <= 1.0 {
            return min_frac;
        }
        (left_width / avail_w).clamp(min_frac, max_frac)
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

    #[test]
    fn left_split_frac_round_trips_across_viewport_sizes() {
        // Idle frames under a different window size must not drift the saved frac
        // when width is re-derived from settings (persisted-ui-state / egui skill).
        const MIN: f32 = 0.15;
        const MAX: f32 = 0.55;
        let saved = 0.32_f32;
        for avail_w in [640.0_f32, 880.0, 1280.0, 1920.0] {
            let width = EditorNav::left_width_from_frac(avail_w, saved, MIN, MAX);
            let back = EditorNav::frac_from_left_width(avail_w, width, MIN, MAX);
            assert!(
                (back - saved).abs() < 1e-5,
                "avail_w={avail_w}: frac drifted {saved} -> width {width} -> {back}"
            );
        }
    }
}
