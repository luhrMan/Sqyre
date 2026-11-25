# Non-delete confirm prompts

## Story

Overwrite, migrate, and restore prompts reuse the same confirm chrome as deletes, with clear action verbs and Enter/Esc that match the labeled choices.

## Rules

- Non-delete irreversible or replace prompts use `confirm_window` (same chrome as deletes) — do not invent a second modal stack.
- Single-verb confirms (e.g. Data Editor **Overwrite** / overwrite variant) use `confirm_cancel_row` with that verb as the confirm label — never generic “Confirm”.
- Multi-choice confirms (e.g. settings **Move data?** Yes/No, **Import backup?** Overwrite/Merge) use `confirm_choice_row` with `ConfirmKind` (Destructive for the dangerous choice when applicable).
- Esc cancels; Enter activates the documented default choice (`enter_choice` / primary confirm) only when the confirm is the top layer and no focused widget inside it owns Enter.
- Title-bar close clears pending confirm state the same as Cancel.

## Examples

- the one where a confirm footer is right-aligned in a wide offered rect → row does not inflate the dialog  
  `Test: crates/sqyre-app/src/widgets/dialogs.rs::confirm_row_right_aligns_without_inflating_width`
- the one where Enter reaches only the top window’s save/confirm path despite background focus  
  `Test: crates/sqyre-app/src/widgets/dialogs.rs::enter_saves_top_window_despite_background_focus`  
  (same `is_top_layer` / focus rule as `poll_confirm_keys`)
- the one where a focused widget inside the window owns Enter → footer does not double-fire  
  `Test: crates/sqyre-app/src/widgets/dialogs.rs::enter_does_not_save_while_widget_in_window_is_focused`
- the one where a choice row has `enter_choice` → Enter selects that index  
  `Test: crates/sqyre-app/src/widgets/dialogs.rs::confirm_choice_row_enter_selects_enter_choice`
- the one where a Data Editor overwrite prompt is open → the confirm verb is **Overwrite**; Enter returns that overwrite and clears the prompt  
  `Test: crates/sqyre-app/src/data_editor/dialogs.rs::draw_confirm_enter_confirms_overwrite`
- the one where settings restore-backup confirm is open → **Overwrite** (destructive) and **Merge** are offered; Enter overwrites (live-only files are replaced; status says Overwrote)  
  `Test: crates/sqyre-app/tests/ui_interaction.rs::enter_on_restore_backup_confirm_overwrites`
- the one where settings move-data confirm is open → Enter defaults to **Yes** and moves the existing data  
  `Test: crates/sqyre-app/tests/ui_interaction.rs::enter_on_move_data_confirm_moves_data`

## Questions

_(none)_

## Out of scope

- Destructive **remove** / Delete-key hygiene (see [destructive-confirm.md](destructive-confirm.md))
- Footer clipping / runaway-grow (see [dialog-layout.md](dialog-layout.md))
- Variant **name** prompts and other non-confirm editor modals

## Implementation note (not acceptance criteria)

Data Editor: `PendingConfirm::Overwrite` / `OverwriteVariant` via `EditorPrompts::draw_confirm`. Settings: `PendingConfirm::MoveData` / `RestoreBackup` in `settings/data.rs` via `confirm_choice_row`. Shared keys: `poll_confirm_keys`.
