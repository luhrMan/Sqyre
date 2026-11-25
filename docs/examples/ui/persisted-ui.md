# Layout chrome persists without corruption

## Rules

- Persist UI layout chrome (splits, panel fractions, similar geometry) only on a user gesture or an explicit settled lifecycle event — never from unconditional per-frame draw or idle window resize.
- Store semantic values (fraction / ratio / enum), not mid-layout pixel samples.
- Restore clamps are read-side only; never write a clamped value back as a “fix.”
- Stale egui `PanelState` must not override the saved settings value.

## Examples

- the one where the user drags the macro-list split → fraction is saved and restores after restart  
  `Test: crates/sqyre-app/tests/macro_list_split.rs::macro_list_drag_persists_split_to_settings`
- the one where the window is resized without dragging the divider → saved split fraction is unchanged  
  `Test: crates/sqyre-app/tests/macro_list_split.rs::macro_list_window_resize_keeps_saved_split`
- the one where stale eframe `PanelState` disagrees with settings → settings fraction wins  
  `Test: crates/sqyre-app/tests/macro_list_split.rs::macro_list_stale_panel_state_yields_to_settings`

## Questions

_(none — confirmed: gesture/settled-only write-back for layout chrome is non-negotiable)_

## Out of scope

- User preference settings (sort, checkboxes, scale, …) — see [settings-preferences.md](settings-preferences.md)
- Catalog / program data persistence

## Implementation note (not acceptance criteria)

New layout fields: round-trip test (precedent `macro_list_split.rs`); also assert idle resize does not rewrite the store. Defaults on the owning settings type; prefer partitioned structs (`crate-boundaries`). Window geometry may use a settled lifecycle save (`save_window_geometry`) — still not per-frame panel math. Cookbook: `egui-interaction-testing` Pattern 5.
