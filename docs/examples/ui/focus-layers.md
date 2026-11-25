# The live top surface owns keys

## Rules

- Only the live top surface may handle Esc / Enter / in-panel activation keys.
- Closed foreground Areas may linger in egui’s layer order; that must not permanently block or steal keys from the surface underneath.
- Esc closes the topmost layer only — never a raw app-wide Escape consume that skips the stack.

## Examples

- the one where Enter saves a dirty program after a foreground dialog was Esc-closed → save still works  
  `Test: crates/sqyre-app/tests/ui_interaction.rs::data_editor_enter_saves_after_foreground_dialog_closed`

## Questions

_(none — confirmed: layer-safe key routing + Esc topmost-only are non-negotiable)_

## Out of scope

- Search field focus on open (see [search-on-open.md](search-on-open.md))
- Context-menu keyboard open (see [nested-popups.md](nested-popups.md))
- In-panel ↑↓ (see [usability.md](usability.md))
- Recording Esc, suspend, and shell shortcuts while a recorder is open (see [recording-input.md](recording-input.md))
- Custom widget `request_focus` mechanics (implementation note below)

## Implementation note (not acceptance criteria)

Gate “may this surface take keys?” with `focus_nav::is_top_layer`, not `top_layer_id(Foreground).is_none()`. Prefer `widgets::consume_escape`. Arrow focus is locked to the top window via `focus_nav::lock_to_window`. Custom controls that need keys must `request_focus()` on pointer down/click and paint a visible focus state — `ui.interact` does not opt in. Cookbook: `egui-interaction-testing` Pattern 4.
