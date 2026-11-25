# Dialogs and windows do not runaway-grow

## Rules

- Resizable dialogs/windows must not expand every frame toward the screen edge because content treated available space as a minimum.
- Scroll regions inside those surfaces must not inflate the parent by claiming more than `min(content, viewport)`.
- Right-anchored Save/Confirm/Dismiss rows must not stretch an auto-sized surface to the full constrain width.

## Examples

_(enforcement incomplete — no dedicated runaway-growth / scroll-claim / footer-inflate green yet; helpers below are the current gate)_

## Questions

- Add a headless green that a dialog body cannot ratchet toward `max_size` via available-as-min layout?
- Add a green that a right-button footer does not expand an auto-sized popup to the constrain width?

## Out of scope

- Tip hug / tip ratchet (see [tooltip-ratchet.md](tooltip-ratchet.md))
- Cell selection stroke geometry (see [cell-strokes.md](cell-strokes.md))
- Overlay picker layout in `sqyre-overlay` (cannot import `sqyre-app`; keep local helpers)

## Implementation note (not acceptance criteria)

Use `fit_dialog_popup` / `fill_resize_body` / `fill_row` (`desired_width(INFINITY)`, trailing controls first). Scroll via `dialog_scroll` / `show_scroll_edge_fit` / `popup_scroll_max_height`. Footers via `save_row` / `confirm_*_row` / `dismiss_row` (`right_button_row`) — never bare `Layout::right_to_left` in auto-sized surfaces. Do not `set_min_width(available)` / uncapped `ScrollArea` / fill `TextEdit` + siblings left-to-right.
