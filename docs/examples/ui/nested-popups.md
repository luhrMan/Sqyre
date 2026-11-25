# Nested parent popups stay open over child menus

## Rules

- A parent popup that opens nested layers (context menus, suggestion lists) must stay open while the user interacts inside those children.
- Close the parent only on a click outside the parent **and** every registered nested rect, or via Esc / explicit close.
- Parents that need this behavior use `widgets::parent_popup` with open state in temp data (`id.with("open")`), not egui’s single `Memory::popups` slot.
- Nested menus/dropdowns call `widgets::register_nested_popup(ctx, rect)` each frame while open.
- Leaf suggestion popups with **no** children may use `egui::Popup` directly; do not wrap every leaf in `parent_popup`.

## Examples

- the one where a chip context menu Remove runs inside a parent popup → parent stays open and the tag is removed  
  `Test: crates/sqyre-app/src/widgets/context_menu.rs::chip_menu_inside_parent_popup_stays_open_and_removes`
- the one where a suggestion row is clicked inside a parent popup → parent stays open and the value is applied  
  `Test: crates/sqyre-app/src/widgets/context_menu.rs::suggestion_click_inside_parent_popup_stays_open`
- the one where the user clicks outside the parent and all nested rects → parent closes  
  `Test: crates/sqyre-app/src/widgets/context_menu.rs::outside_click_closes_parent_popup`
- the one where an edit tip sits above a foreground popup click → tip stays open  
  `Test: crates/sqyre-app/src/action_tooltip/mod.rs::edit_tip_click_on_foreground_popup_stays_open`
- the one where Shift+F10 is pressed on a focused row → context menu opens  
  `Test: crates/sqyre-app/src/widgets/context_menu.rs::shift_f10_opens_the_menu_for_the_focused_row`

## Questions

_(none — confirmed: keep `parent_popup` / `register_nested_popup` for parents with children; leaves may stay raw)_

## Out of scope

- Tray / OS-native menus
- Changing egui’s global `Memory::popups` semantics
- Layer-safe Esc/Enter after foreground dialogs (see [focus-layers.md](focus-layers.md))
