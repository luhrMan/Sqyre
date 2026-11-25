# Recording input

## Story

Macro-action, hotkey-chord, single-key, and screen-click capture each own their keys; macro hotkeys stay suspended until that capture finishes, and shell shortcuts stay quiet while a recorder is open.

## Rules

- Macro-action (`MacroRecordBridge` / `MacroRecordUi`): Esc **stops** capture. Esc is absent from the event list and from the review actions. Hotkeys stay suspended while capture is armed; Esc finish and Cancel both resume.
- Hotkey chord (`HotkeyRecordUi`): Esc dismisses **only when no keys are held**. A held Esc joins the chord. Cancel (the button) dismisses while keys are held. Suspend lasts from open through `WaitingRelease` until the chord is released or Cancel.
- Single-key (`KeyRecordUi`): Esc **is recordable**. Cancel is button-only (`dismiss_row(..., false)`). Suspend lasts through `WaitingRelease` until that key is released or Cancel; Cancel saves nothing.
- Screen-click (`ScreenClickBridge`): Esc cancels the armed session (point, color, or search area) and yields no result. With `hide_app_during_recording`, the main window may hide while screen-click or macro-action capture is armed; `recording_suppresses_shell` then skips normal shell input. The overlay/grab owns the click (`set_grab_owns_input` also blocks hook clicks).
- Global shell shortcuts stay off while `hotkey_record`, `key_record`, or `macro_record` is open: command palette (Ctrl+K), zoom, tree clipboard / add-action chords (`handle_shortcuts`), and Delete on the macro tree (`tree_owns_delete`).
- Overlay macro buttons are cleared and the quick-access fan snaps shut while `screen_click` or `macro_record_bridge` is armed.
- Key and hotkey dialogs use shared chrome: `chord_record::record_modal` and `dismiss_row`.

## Examples

- the one where macro-action capture receives `a` then Esc → recording stops, the event list has no Esc, and the review actions include `a` but not Esc  
  `Test: crates/sqyre-hotkeys/src/macro_record.rs::records_keys_until_esc`  
  `Test: crates/sqyre-hotkeys/src/macro_record.rs::esc_stops_recording`  
  `Test: crates/sqyre-app/src/macro_record.rs::macro_record_esc_finishes_resumes_and_omits_esc`
- the one where macro-action capture is cancelled → hotkeys resume, the UI closes, and no events remain  
  `Test: crates/sqyre-app/src/macro_record.rs::macro_record_cancel_resumes_hotkeys`
- the one where the hotkey dialog is open with an empty chord and Esc is pressed → the dialog closes and hotkeys resume  
  `Test: crates/sqyre-app/src/hotkey_record.rs::esc_with_empty_chord_dismisses_and_resumes`
- the one where Esc is held in the hotkey dialog → Esc joins the chord (`esc`) and a further Escape press does not dismiss  
  `Test: crates/sqyre-app/src/hotkey_record.rs::esc_held_joins_chord_without_dismiss`
- the one where a chord is held and Cancel is clicked → the dialog closes and hotkeys resume  
  `Test: crates/sqyre-app/src/hotkey_record.rs::cancel_while_chord_held_dismisses_and_resumes`
- the one where hotkey record is in WaitingRelease with `a` still down → hotkeys stay suspended until `a` is released  
  `Test: crates/sqyre-app/src/hotkey_record.rs::waiting_release_stays_suspended_until_release`
- the one where single-key record sees Esc → the saved key is `esc`, Escape does not dismiss, and hotkeys resume only after release  
  `Test: crates/sqyre-app/src/key_record.rs::esc_is_recorded_and_release_resumes`
- the one where single-key record is waiting and Escape is pressed, then Cancel → nothing is saved and hotkeys resume  
  `Test: crates/sqyre-app/src/key_record.rs::escape_key_does_not_dismiss_cancel_does`
- the one where screen-click is armed (point, color, or search area) and Esc is pressed → the session cancels with no point, color, or area  
  `Test: crates/sqyre-hotkeys/src/screen_click.rs::esc_cancels_armed_session`
- the one where hide-during-recording is on and screen-click or macro-action capture is armed → the main window hides and the shell suppresses normal input; disarm shows it again; the setting off leaves the window up  
  `Test: crates/sqyre-app/src/app_run.rs::armed_recording_may_hide_main_window`
- the one where the selection grab owns input → hook clicks stay blocked until explicitly allowed  
  `Test: crates/sqyre-hotkeys/src/screen_click.rs::grab_owns_input_flag_roundtrip`  
  `Test: crates/sqyre-hotkeys/src/screen_click.rs::allow_hook_clicks_keeps_grab_flag`
- the one where Ctrl+K, zoom, or Ctrl+A is pressed with a recorder open → the palette stays closed, scale is unchanged, and Add Action stays closed; the same chords fire when every recorder is closed  
  `Test: crates/sqyre-app/src/ui_overlays.rs::shortcuts_do_not_fire_while_recorders_open`
- the one where the macro tree would own Delete, then a recorder opens → Delete is no longer owned  
  `Test: crates/sqyre-app/src/ui_macro_tree.rs::recorders_block_tree_delete`
- the one where an enabled overlay button exists and screen-click or macro-action capture is armed → the hosted button list is cleared and the quick-access fan closes  
  `Test: crates/sqyre-app/src/ui_overlays.rs::armed_recording_clears_overlay_buttons`

## Questions

_(none)_

## Out of scope

- Wait / move / tap / snap thresholds (Settings → Recording; runtime)
- Wayland / X11 grab implementation and overlay drawing
- WASM editor recording surfaces — future `wasm-editor-surface` map
- Layer-stack Esc / Enter (see [focus-layers.md](focus-layers.md))

## Implementation note (not acceptance criteria)

Shared modal: `chord_record::record_modal`, `poll_waiting_release`, `widgets::dismiss_row`. Shell gate: `ui_overlays::handle_shortcuts` and `tree_owns_delete`. Window hide: `update_recording_visibility` (`hide_app_during_recording`) and `recording_suppresses_shell` (early return in `SqyreApp::ui` while hidden and still armed). Overlay gate: `sync_macro_overlay` syncs an empty button list when `screen_click` or `macro_record_bridge` is armed. Grab ownership: `ScreenClickBridge::set_grab_owns_input` / `block_hook_clicks`, polled from `RecordingOverlay`.
