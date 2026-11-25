# Destructive remove confirms

## Story

Removing catalog entities or macros asks for confirm; Delete while typing never removes the selection; macro-tree actions stay one-key delete.

## Rules

- Catalog / Data Editor deletes (program, entity, icon variant, mask image) open `confirm_window` + `confirm_cancel_row` with an action verb (**Delete** / **Remove**) — never one-shot from the footer, context menu, or Delete key.
- Macro-list delete opens the same confirm chrome (`Delete Macro`).
- Macro-tree **actions**: Delete removes the selection immediately (no confirm).
- Delete key must not remove a selection while a text field is focused, a popup/prompt owns keys, or the surface is not the top layer.
- Confirm footer: Enter confirms and Esc cancels when the confirm is the top layer and no focused widget inside it owns Enter (`poll_confirm_keys`).
- Use shared helpers (`confirm_window`, `confirm_cancel_row`, `ConfirmKind::Destructive` where the primary is red) — do not invent a parallel confirm stack.

## Examples

- the one where Delete is pressed on a selected catalog program → confirm opens; program stays until Enter confirms  
  `Test: crates/sqyre-app/tests/ui_interaction.rs::delete_key_confirms_removal_of_selected_model`
- the one where Delete is pressed in the program name field → no confirm; program remains  
  `Test: crates/sqyre-app/tests/ui_interaction.rs::delete_key_in_model_name_field_does_not_remove_model`
- the one where Delete is pressed on a selected macro-tree action → action is removed immediately (no confirm)  
  `Test: crates/sqyre-app/tests/ui_interaction.rs::delete_key_removes_selected_macro_action`
- the one where a confirm footer is right-aligned in a wide offered rect → row does not inflate the dialog  
  `Test: crates/sqyre-app/src/widgets/dialogs.rs::confirm_row_right_aligns_without_inflating_width`
- the one where macro-list Delete opens the confirm and Enter removes the macro  
  `Test: crates/sqyre-app/tests/ui_interaction.rs::enter_confirms_macro_list_delete`

## Questions

_(none)_

## Out of scope

- Overwrite / migrate / restore multi-choice prompts (see [choice-confirms.md](choice-confirms.md))
- Confirm footer layout geometry beyond the shared helper contract (see [dialog-layout.md](dialog-layout.md))
- Esc / top-layer routing details (see [focus-layers.md](focus-layers.md))

## Implementation note (not acceptance criteria)

Data Editor: `request_delete_selected` / `PendingConfirm::Delete*` → `EditorPrompts::draw_confirm`. Macro list: `pending_delete`. Tree: `tree_delete_key` / `tree_owns_delete` (immediate remove). Delete gating: `text_edit_focused`, `owns_delete_key`, `any_popup_open`.
