# Growing lists are sorted, filterable, and honest when empty

## Rules

- Named lists present a stable, predictable order (usually display-name, case-insensitive).
- Lists that can grow beyond ~a screenful offer a filter; empty and no-match bodies show plain copy (not a blank void).
- Expand/collapse-all and similar bulk list actions respect the active filter.

## Examples

- the one where display names are sorted → order is case-insensitive by name  
  `Test: crates/sqyre-app/src/pickers/tests.rs::sort_by_display_name_orders_case_insensitive`
- the one where the data-editor header search is typed → rows filter and the query is preserved  
  `Test: crates/sqyre-app/tests/ui_interaction.rs::data_editor_header_search_filters_and_preserves_query`
- the one where items sort menu hosts tag priority → control is available in the sort surface  
  `Test: crates/sqyre-app/tests/ui_interaction.rs::data_editor_items_sort_menu_hosts_tag_priority`
- the one where macro-list expand/collapse-all runs with an active search → expand trails the filter  
  `Test: crates/sqyre-app/tests/ui_interaction.rs::macro_list_expand_collapse_all_trails_search`
- the one where the list query is empty vs non-empty → vacancy copy distinguishes “none yet” from “no match”  
  `Test: crates/sqyre-app/src/widgets/empty_state.rs::list_vacancy_copy_empty_vs_filter`

## Questions

_(none)_

## Out of scope

- Settings window search keywords (see [settings-preferences.md](settings-preferences.md))
- Focus search field on open (see [search-on-open.md](search-on-open.md))
- Icon-first chrome (see [icon-chrome.md](icon-chrome.md))

## Implementation note (not acceptance criteria)

Prefer `macro_meta::cmp_display_name` / `pickers::sort_by_display_name`, `widgets::search_field` / `searchable_combo`, `list_vacancy` / `empty_state`.
