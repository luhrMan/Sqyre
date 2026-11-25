# Dialogs and surfaces size without runaway growth

## Rules

- Dialogs use `widgets::fit_dialog_popup` (+ optional tighter `max_size`); bodies that must fill use `fill_resize_body`. Never treat `available_width()` / `available_height()` as a minimum (`set_min_width(available)`, fill `TextEdit` + siblings in a left-to-right row, uncapped `ScrollArea`).
- Fill leftover width with `desired_width(INFINITY)` and `widgets::fill_row` (trailing controls first).
- Scroll in dialogs via `pickers::dialog_scroll` / `show_scroll_edge_fit` / `popup_scroll_max_height` — claim `min(content, viewport)`, not full content height.
- Right-aligned button rows use `confirm_*_row` / `dismiss_row` / `save_row` (`right_button_row`); never `Layout::right_to_left` alone inside auto-sized surfaces.
- Selection/focus strokes on cells use `StrokeKind::Inside`, not `Outside`.
- Tips and narrow self-sizing surfaces hug content; they must not expand to fill the window.

## Examples

- the one where a view tip has narrow content → tip height stays well below the dialog constrain rect  
  `Test: crates/sqyre-app/tests/ui_interaction.rs::view_tip_hugs_narrow_content`
- the one where a grid cell is selected → selection stroke stays inside the cell  
  `Test: crates/sqyre-app/src/pickers/icon_grid.rs::selected_cell_stroke_stays_inside_cell`

## Questions

_(none)_

## Out of scope

- Overlay picker layout in `sqyre-overlay` (cannot import `sqyre-app`; keep local helpers)
