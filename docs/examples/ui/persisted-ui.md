# Persisted UI writes only on user gestures

## Rules

- Write settings on user gestures only (drag release, click, Enter, explicit Save, or a debounced settled signal) — never `settings.save()` in unconditional per-frame draw.
- Persist semantic values (fraction / ratio / enum), not transient pixel samples mid-layout.
- No writes during settling (first frames, one-frame transient window sizes, restore-in-progress).
- Restore clamps are read-side only; never write the clamped value back as a “fix”.
- Defaults live on the settings type that owns the field (`#[serde(default)]`); prefer partitioned settings structs (`crate-boundaries`).
- Every new persisted field gets a round-trip test; gesture-gated fields also assert idle resize does not rewrite the store.

## Examples

- the one where the user drags the macro-list split → fraction is saved and restores after restart  
  `Test: crates/sqyre-app/tests/macro_list_split.rs::macro_list_drag_persists_split_to_settings`
- the one where the window is resized without dragging the divider → saved split fraction is unchanged  
  `Test: crates/sqyre-app/tests/macro_list_split.rs::macro_list_window_resize_keeps_saved_split`
- the one where stale eframe `PanelState` disagrees with settings → settings fraction wins  
  `Test: crates/sqyre-app/tests/macro_list_split.rs::macro_list_stale_panel_state_yields_to_settings`
- the one where items sort preference was saved → next open restores it  
  `Test: crates/sqyre-app/tests/ui_interaction.rs::data_editor_items_sort_restores_from_settings`

## Questions

_(none)_

## Out of scope

- Catalog / program data persistence (not UI chrome)
