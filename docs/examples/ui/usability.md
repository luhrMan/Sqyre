# Lists, search, icons, and progressive disclosure stay consistent

## Rules

- Named lists present a stable, predictable order (usually display-name, case-insensitive) via shared comparators (`macro_meta::cmp_display_name`, `pickers::sort_by_display_name`).
- Lists that can grow beyond ~a screenful use `widgets::search_field` (or `searchable_combo`); empty/no-match bodies use `list_vacancy` / `empty_state`.
- Icon-first chrome: glyphs portray, tooltips explain (`icon_button`, `help::label`, Phosphor / `VectorIcon`). Static tip strings live in `h::` / `ht::` registries.
- Beginner path first; expert controls under `widgets::advanced_section` / `tip_advanced` — never nest Advanced inside another disclosure; never raw `ui.collapsing` for that role.
- Compose shared `theme::*` / `widgets::*` / `pickers::*`; no local palettes or one-off chrome.
- Never bypass capability helpers where they apply (`var_ref_text_edit`, `parent_popup`, `visible_*`, search/button helpers) — extend opts instead of dropping to raw egui.
- Density uses only `section_gap` / `element_gap`; gaps flex, control sizes do not.
- AccessKit labels on icon-only and custom-painted widgets; stable ids in loops (`id_salt` / domain id, never row index).
- Prefer in-panel keys (↑↓ / Enter / Esc); do not steal keys while recording macros/hotkeys/chords.

## Examples

- the one where the data-editor header search is typed → rows filter and the query is preserved  
  `Test: crates/sqyre-app/tests/ui_interaction.rs::data_editor_header_search_filters_and_preserves_query`
- the one where items sort menu hosts tag priority → control is available in the sort surface  
  `Test: crates/sqyre-app/tests/ui_interaction.rs::data_editor_items_sort_menu_hosts_tag_priority`
- the one where image-search edit opens → output and wait are visible without expanding Advanced  
  `Test: crates/sqyre-app/tests/ui_interaction.rs::image_search_edit_shows_output_and_wait_without_advanced`
- the one where macro-list expand/collapse-all is used with an active search → expand trails the search filter  
  `Test: crates/sqyre-app/tests/ui_interaction.rs::macro_list_expand_collapse_all_trails_search`

## Questions

_(none)_

## Out of scope

- Brand / theme redesigns
- New global hotkey schemes (see [focus-layers.md](focus-layers.md))
- Search field focus on open (see [search-on-open.md](search-on-open.md))
- Nested parent popup survival (see [nested-popups.md](nested-popups.md))
