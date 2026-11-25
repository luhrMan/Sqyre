# Common path first; Advanced stays collapsed

## Rules

- Show the beginner / common controls first.
- Expert or rarely needed knobs live under `Advanced` (or a named feature `collapsible_section`) — collapsed by default.
- Do not nest an Advanced group inside another disclosure.

## Examples

- the one where image-search edit opens → output and Wait mode are visible without expanding Advanced  
  `Test: crates/sqyre-app/tests/ui_interaction.rs::image_search_edit_shows_output_and_wait_without_advanced`

## Questions

_(none)_

## Out of scope

- Settings search forcing Advanced open when a query hits a knob (see [settings-preferences.md](settings-preferences.md) — searchable)
- Brand / theme redesigns

## Implementation note (not acceptance criteria)

Use `widgets::advanced_section` / `tip_advanced` / `collapsible_section` — never raw `ui.collapsing` for that role.
