# Selection and focus strokes stay inside the cell

## Rules

- Selection/focus rings on grid or list cells must not paint outside the cell into gaps, margins, or padding (`StrokeKind::Inside`, not `Outside`).

## Examples

- the one where a grid cell is selected → selection stroke stays inside the cell  
  `Test: crates/sqyre-app/src/pickers/icon_grid.rs::selected_cell_stroke_stays_inside_cell`
- the one where a collection image cell is selected → selection stroke stays inside the cell  
  `Test: crates/sqyre-app/src/pickers/collection_cell.rs::selected_cell_stroke_stays_inside_cell`
- the one where an Add Action tile is selected → selection stroke stays inside the tile  
  `Test: crates/sqyre-app/src/add_action.rs::selected_tile_stroke_stays_inside_cell`

## Questions

_(none)_

## Out of scope

- Dialog/window growth (see [dialog-layout.md](dialog-layout.md))
- Tip geometry (see [tooltip-ratchet.md](tooltip-ratchet.md))
