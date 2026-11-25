# Icon chrome portrays; labels and tips explain

## Rules

- Icon-only actions expose an AccessKit / hover name (the wording the glyph replaces).
- Custom-painted interactive rows expose an accessible type/name label, not glyph-only silence.
- Static tip strings live in registries (`h::` / `ht::`); do not hand-write static hover prose at call sites.

## Examples

- the one where a tree action row is painted → AccessKit exposes the action type label  
  `Test: crates/sqyre-app/tests/ui_interaction.rs::demo_tree_action_row_exposes_accessible_type_label`
- the one where an icon button is at rest → no chrome fill until hover  
  `Test: crates/sqyre-app/src/widgets/controls.rs::icon_button_chrome_only_on_hover`

## Questions

_(none)_

## Out of scope

- Elided text disclosure tips (see [elided-tooltips.md](elided-tooltips.md))
- Tip width ratchet (see [tooltip-ratchet.md](tooltip-ratchet.md))

## Implementation note (not acceptance criteria)

Prefer `icon_button` / `help::label` / Phosphor / `VectorIcon`. Density gaps use `section_gap` / `element_gap` only (checklist in `ui-usability.mdc`).
