# Tooltips and self-sizing Areas do not ratchet

## Rules

- `UiKind::Tooltip` / `Popup` / `Menu` Areas size from last frame’s painted size; layout bounds must **not** be derived from `clip_rect ∩ max_rect` (or the tip’s own painted width) or wrapped text collapses toward one-character rows.
- Inside tips/popups/menus, size with `widgets::visible_size` / `visible_width` / `visible_height` / `visible_content_width` (constrain-rect aware).
- Fixed-size preview content inside tips must use those helpers, never a hardcoded display cap that ignores the visible budget.
- Hover tips that disclose elided labels must show the full text across multiple frames (loop ≥3 frames in tests).

## Examples

- the one where an item icon hover tip paints for many frames → tip width does not shrink frame-to-frame  
  `Test: crates/sqyre-app/src/pickers/icon_grid.rs::item_tooltip_width_does_not_ratchet`
- the one where a preview error tip is hovered → width does not ratchet  
  `Test: crates/sqyre-app/src/preview_tooltip/paint.rs::preview_error_tip_width_does_not_ratchet`
- the one where a collection image file hover tip is shown → width does not ratchet  
  `Test: crates/sqyre-app/src/data_editor_preview.rs::collection_image_file_hover_tip_does_not_ratchet`
- the one where a locked name-tag tip is shown → width does not ratchet  
  `Test: crates/sqyre-app/src/widgets/tags.rs::locked_name_tag_tooltip_width_does_not_ratchet`
- the one where a program name is elided → hover tip discloses the full text  
  `Test: crates/sqyre-app/tests/ui_interaction.rs::data_editor_program_name_tooltip_discloses_elided_text`
- the one where a long macro name is elided → hover tip discloses the full text  
  `Test: crates/sqyre-app/tests/ui_interaction.rs::long_macro_name_tooltip_discloses_elided_text`

## Questions

_(none)_

## Out of scope

- Window/dialog growth ratchets (see [dialog-layout.md](dialog-layout.md))
