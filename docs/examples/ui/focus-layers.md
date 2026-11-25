# Focus and keys respect the live top layer

## Rules

- Foreground surfaces (palette, confirm, modal pickers) may leave a closed `Area` in egui’s layer order; “may this surface take keys?” uses `focus_nav::is_top_layer`, never `top_layer_id(Foreground).is_none()`.
- Esc closes the topmost layer only (`widgets::consume_escape`); do not raw-`consume_key(Escape)` across the app.
- After a foreground dialog closes, underlying surfaces must accept activation keys again (e.g. Enter to save).
- Custom focusable widgets must `request_focus()` on pointer down/click (`ui.interact` does not) and paint a visible focus state when `has_focus()`.
- Prefer in-panel keys (↑↓ / Enter / Esc); do not steal keys while recording macros/hotkeys/chords.
- Search-first surfaces focus the search field on open when that is the main job.

## Examples

- the one where Enter saves a dirty program after a foreground dialog was Esc-closed → save still works  
  `Test: crates/sqyre-app/tests/ui_interaction.rs::data_editor_enter_saves_after_foreground_dialog_closed`
- the one where the add-action picker opens → search field has focus  
  `Test: crates/sqyre-app/tests/ui_interaction.rs::add_action_picker_focuses_search_on_open`
- the one where Shift+F10 is pressed on a focused row → context menu opens  
  `Test: crates/sqyre-app/src/widgets/context_menu.rs::shift_f10_opens_the_menu_for_the_focused_row`
- the one where the command palette is reopened after click-outside → rows still respond to mouse  
  `Test: crates/sqyre-app/tests/ui_interaction.rs::command_palette_reopened_after_click_outside_responds_to_mouse_click`

## Questions

_(none)_

## Out of scope

- New global application hotkey schemes
