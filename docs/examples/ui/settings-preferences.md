# Preference settings are saved, navigable, searchable, and understandable

## Rules

- **Saved** — Changing a preference (checkbox, sort, scale, …) persists and restores on the next open/session.
- **Navigable** — Settings can be reached and operated with pointer and keyboard; the settings search field keeps focus while typing (including when the clear control appears/disappears).
- **Searchable** — A filter finds settings and sections by everyday keywords; a section/subsection title hit reveals its settings; Advanced groups open when the query names a knob inside them.
- **Understandable** — Labels and hover tips use plain language (see also [progressive-disclosure.md](progressive-disclosure.md) for Advanced placement).

## Examples

### Saved

- the one where log-meta-images is toggled in Settings → value flips and stays the preference  
  `Test: crates/sqyre-app/tests/ui_interaction.rs::settings_checkbox_toggles_log_meta_images`
- the one where OS title bar is toggled under Appearance → preference flips  
  `Test: crates/sqyre-app/tests/ui_interaction.rs::settings_checkbox_toggles_os_title_bar`
- the one where items sort preference was saved → next open restores it  
  `Test: crates/sqyre-app/tests/ui_interaction.rs::data_editor_items_sort_restores_from_settings`

### Navigable

- the one where settings search is typed through empty ↔ non-empty → the field keeps focus  
  `Test: crates/sqyre-app/src/settings/mod.rs::search_keeps_focus_when_query_crosses_empty`

### Searchable

- the one where the query is a section title → settings in that section stay visible  
  `Test: crates/sqyre-app/src/settings/mod.rs::section_title_reveals_all_settings_in_section`
- the one where the query is “image search” → Macros section matches, General does not  
  `Test: crates/sqyre-app/src/settings/mod.rs::image_search_matches_macros_not_general`
- the one where the query is “borderless” / “title bar” → OS title bar setting matches  
  `Test: crates/sqyre-app/src/settings/mod.rs::borderless_matches_os_title_bar_setting`

## Questions

_(none)_

## Out of scope

- Layout chrome write-back / split corruption (see [persisted-ui.md](persisted-ui.md))
- Search-first focus-on-open for pickers/palettes (see [search-on-open.md](search-on-open.md))
- Action-editor Advanced placement (see [progressive-disclosure.md](progressive-disclosure.md))
- Brand / theme redesigns
