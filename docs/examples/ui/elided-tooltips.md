# Elided labels disclose full text on hover

## Rules

- When a list/row name is elided in the UI, hovering it must show the full text in a tip (readable across multiple frames).

## Examples

- the one where a program name is elided → hover tip discloses the full text  
  `Test: crates/sqyre-app/tests/ui_interaction.rs::data_editor_program_name_tooltip_discloses_elided_text`
- the one where a long macro name is elided → hover tip discloses the full text  
  `Test: crates/sqyre-app/tests/ui_interaction.rs::long_macro_name_tooltip_discloses_elided_text`
- the one where a stress-tree type summary is elided at UI scale 2 → tip discloses the full type text  
  `Test: crates/sqyre-app/tests/ui_interaction.rs::stress_tree_summary_tooltip_discloses_full_type_text_at_scale_two`

## Questions

_(none — confirmed for list/row names that already elide; extend the same rule when adding new eliding name surfaces)_

## Out of scope

- Tip width ratchet / self-sizing Area layout (see [tooltip-ratchet.md](tooltip-ratchet.md))
- Icon-only chrome tips that are not disclosing elided content (see [usability.md](usability.md))
