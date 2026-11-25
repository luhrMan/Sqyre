# Hover tips stay readable and hug their content

## Rules

- Hover tips, popup bodies, and menus must not shrink their layout budget frame-to-frame in a way that collapses wrapped text toward one-character rows.
- Tips with narrow content must hug that content — they must not expand to fill the window or dialog constrain rect.
- Fixed media inside tips (previews, images) must fit the visible budget — not a hardcoded display size that ignores it.

## Examples

- the one where an item icon hover tip paints for many frames → tip width does not shrink frame-to-frame  
  `Test: crates/sqyre-app/src/pickers/icon_grid.rs::item_tooltip_width_does_not_ratchet`
- the one where a preview error tip is hovered → width does not ratchet  
  `Test: crates/sqyre-app/src/preview_tooltip/paint.rs::preview_error_tip_width_does_not_ratchet`
- the one where a collection image file hover tip is shown → width does not ratchet  
  `Test: crates/sqyre-app/src/data_editor_preview.rs::collection_image_file_hover_tip_does_not_ratchet`
- the one where a locked name-tag tip is shown → width does not ratchet  
  `Test: crates/sqyre-app/src/widgets/tags.rs::locked_name_tag_tooltip_width_does_not_ratchet`
- the one where a view tip has narrow content → tip height stays well below the dialog constrain rect  
  `Test: crates/sqyre-app/tests/ui_interaction.rs::view_tip_hugs_narrow_content`

## Questions

_(none — confirmed: no-ratchet + hug-content for tip/popup/menu surfaces)_

## Out of scope

- Window/dialog growth (see [dialog-layout.md](dialog-layout.md))
- Elided label disclosure on hover (see [elided-tooltips.md](elided-tooltips.md))
- Cell selection strokes (see [cell-strokes.md](cell-strokes.md))

## Implementation note (not acceptance criteria)

Self-sizing `Area`s (`UiKind::Tooltip` / `Popup` / `Menu`) size from last frame’s paint. Prefer `widgets::visible_size` / `visible_width` / `visible_height` / `visible_content_width` over tip-local `clip ∩ max_rect` math. Cookbook: `egui-interaction-testing` Patterns 1–2.
