# Search-first surfaces focus the filter on open

## Rules

- When a surface’s main job is type-to-filter (picker, palette, searchable combo), opening it focuses the search field once — not every frame.
- Mouse/click paths still work if focus was moved away; keyboard type-to-filter is the default affordance on open.

## Examples

- the one where the add-action picker opens → search field has focus  
  `Test: crates/sqyre-app/tests/ui_interaction.rs::add_action_picker_focuses_search_on_open`
- the one where a searchable combo opens → search focuses once and keeps the query  
  `Test: crates/sqyre-app/src/widgets/fields.rs::searchable_combo_focuses_search_once_and_keeps_query`

## Questions

_(none — confirmed as its own map; applies to search-first surfaces, not every dialog)_

## Out of scope

- Layer-safe Esc/Enter after foreground dialogs (see [focus-layers.md](focus-layers.md))
- Filter behavior / empty states once search has focus (see [usability.md](usability.md))
