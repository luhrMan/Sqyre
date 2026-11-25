# Variable-reference fields and pills

## Story

Authors edit and read `${var}` / `{var}` through one helper path: chip overlays when idle, autocomplete for open `${…` / `{…`, unknown names warn without blocking; display pills share the same nested-chip language.

## Rules

- Value fields that accept variable refs use `validated_var_ref_edit` / `validated_var_ref_multiline_edit` / `paint_validated_var_ref` (or place a TextEdit and call `show_var_ref_autocomplete`) — do not invent a parallel editor.
- Var-name fields use `var_name_text_edit`.
- Unfocused non-empty **value** with a ref paints a chip overlay; click focuses the real editor.
- Unfocused non-empty **var-name** paints a name chip the same way.
- Incomplete `${prefix` **or** bare `{prefix` at the caret opens autocomplete; ↑↓ / Enter|Tab / Esc are consumed while it is open. Completing preserves the opener style (`${…}` vs `{…}`).
- Unknown refs are a **warning** (visual + tip), not a submit blocker.
- Neighbor chip click focuses that field only.
- Tree / tip / summary pills that show values with refs use the shared nested-chip paint (`paint_value_pill` / `paint_summary_pill_*` / `paint_var_ref_content`) — same unknown-warning color language as editors.

## Examples

### Edit / focus / autocomplete

- the one where a value is unfocused and contains a ref → chip overlay shows; focused or plain text → no overlay  
  `Test: crates/sqyre-app/src/var_pills.rs::var_ref_overlay_only_when_unfocused_with_ref`
- the one where a var-name is nonempty and unfocused → name chip overlay  
  `Test: crates/sqyre-app/src/var_pills.rs::var_name_overlay_when_nonempty_unfocused`
- the one where the left chip is clicked beside a neighbor → only the left editor focuses  
  `Test: crates/sqyre-app/src/var_pills.rs::click_on_left_chip_focuses_left_editor_not_neighbor`
- the one where the caret is in incomplete `${co` or `{co` → suggestions filter; apply keeps dollar vs bare style  
  `Test: crates/sqyre-app/src/var_pills.rs::incomplete_var_ref_at_caret`  
  `Test: crates/sqyre-app/src/var_pills.rs::completion_filters_and_applies`

### Display pills

- the one where summary pills with refs paint at HiDPI → height stays within the tree row  
  `Test: crates/sqyre-app/src/var_pills.rs::summary_pills_fit_tree_row_at_hidpi`
- the one where plain and `${}` value pills share a wrap row → same band height  
  `Test: crates/sqyre-app/src/var_pills.rs::plain_value_and_name_pills_share_row_height`
- the one where a tip wrap row hosts an expression pill → stays single-line  
  `Test: crates/sqyre-app/src/var_pills.rs::tip_expression_pills_stay_single_line_on_wrap_row`

## Questions

_(none)_

## Out of scope

- Confirm-to-autocreate unknown variables (warn-only for this map)
- `sqyre-varref` escape / rename / parse edge cases (crate tests)
- YAML / code editors and other non-field TextEdit surfaces

## Implementation note (not acceptance criteria)

Primary module: `crates/sqyre-app/src/var_pills.rs`. Preview coord chips may place TextEdit manually but must call `var_ref_autocomplete_keys` + `show_var_ref_autocomplete`. Extend opts / public AC API instead of forking.
