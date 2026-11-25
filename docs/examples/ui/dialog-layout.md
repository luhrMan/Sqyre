# Dialogs and windows do not runaway-grow

## Rules

- Resizable dialogs/windows must not expand every frame toward the screen edge because content treated available space as a minimum.
- Scroll regions inside those surfaces must not inflate the parent by claiming more than `min(content, viewport)`.
- Right-anchored Save/Confirm/Dismiss rows must not stretch an auto-sized surface to the full constrain width.

## Examples

- the one where a dialog body tries `set_min_width(available)` inside `fill_resize_body` → window width stays stable and far below the constrain rect  
  `Test: crates/sqyre-app/src/widgets/dialogs.rs::fill_resize_body_stops_available_as_min_from_growing_window`
- the one where a confirm footer is right-aligned in a wide offered rect → row min width does not reach the right edge  
  `Test: crates/sqyre-app/src/widgets/dialogs.rs::confirm_row_right_aligns_without_inflating_width`
- the one where an edge-expanded scroll paints into frame margin → parent claim stays inside the original bound  
  `Test: crates/sqyre-app/src/widgets/scroll.rs::expanded_viewport_does_not_inflate_parent_size`
- the one where a fill-row hosts TextEdit + trailing button → parent pane is not widened past its max  
  `Test: crates/sqyre-app/src/widgets/fields.rs::fill_row_keeps_trailing_inside_pane_and_parent_unwidened`

## Questions

_(none)_

## Out of scope

- Tip hug / tip ratchet (see [tooltip-ratchet.md](tooltip-ratchet.md))
- Cell selection stroke geometry (see [cell-strokes.md](cell-strokes.md))
- Overlay picker layout in `sqyre-overlay` (cannot import `sqyre-app`; keep local helpers)
- When Delete / overwrite must confirm (see [destructive-confirm.md](destructive-confirm.md), [choice-confirms.md](choice-confirms.md))

## Implementation note (not acceptance criteria)

Use `fit_dialog_popup` / `fill_resize_body` / `fill_row` (`desired_width(INFINITY)`, trailing controls first). Scroll via `dialog_scroll` / `show_scroll_edge_fit` / `popup_scroll_max_height`. Footers via `save_row` / `confirm_*_row` / `dismiss_row` (`right_button_row`) — never bare `Layout::right_to_left` in auto-sized surfaces. Do not `set_min_width(available)` / uncapped `ScrollArea` / fill `TextEdit` + siblings left-to-right.
