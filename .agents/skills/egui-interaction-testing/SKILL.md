---
name: egui-interaction-testing
description: >-
  Headlessly test Sqyre UI dynamic states that layout_sweep cannot see — hover
  tooltips, nested popups/menus, keyboard focus, drag/split gestures, and
  persisted UI state. Use when fixing or preventing regressions in sqyre-app
  tooltips, popups, focus/keys, splitters, dialogs, or persisted sizes; when a
  user reports something "reverted" or "still broken" after gates pass; or when
  adding egui_kittest / ctx.run_ui coverage.
---

# egui interaction testing

`layout_sweep` only proves static scenes don't clip. Every regression class below is invisible to it — each has a cheap headless repro pattern. Add one test **per dynamic state you touch** when editing `sqyre-app` UI (required by `verify-new-code`).

## Where tests live

- `crates/sqyre-app/tests/ui_interaction.rs` — full-app flows via `common::build_docs_harness([w, h], |app| app.open_*_for_docs())` → `egui_kittest::Harness<'_, SqyreApp>`. Drive with `get_by_label(..).click()`, `key_press(Key::Enter/Escape)`, `query_all_by_role(..)` + `.focus()` / `.type_text(..)`, then `harness.run()` / `run_steps(n)` and assert on `harness.state().docs_*()` accessors.
- Inline `#[cfg(test)] mod tests` beside the widget — for `egui::Context`-level checks that need no full app (tooltip areas, painted shapes, area rects).
- `tests/macro_list_split.rs` — persisted-state round-trips.
- `docs_screenshots` / `ui_interaction` run in the nextest `gpu` group (lavapipe); `ctx.run_ui` tests need no GPU.

## Pattern 1 — hover tooltips (single-char wrap, overflow, missing)

Self-sizing `Area`s misbehave over **multiple frames** — always loop ≥3 frames.

```rust
let ctx = egui::Context::default();
for frame in 0..10 {
    let input = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1024.0, 768.0))),
        time: Some(frame as f64 * 0.25),
        predicted_dt: 0.25,
        events: if frame == 0 {
            vec![egui::Event::PointerMoved(cell.center())] // hover opens the tip
        } else { Vec::new() },
        ..Default::default()
    };
    let out = ctx.run_ui(input, |ui| {
        let resp = ui.put(cell, egui::Button::new(""));
        attach_item_icon_tooltip(&resp, &catalog, &mut icons, target); // site under test
    });
    // assert on painted text galley widths across frames — catches the
    // per-frame width ratchet (regression: tooltips collapsing to 1-char rows)
    let tip_w = out.shapes.iter().find_map(|c| match &c.shape {
        egui::Shape::Text(t) if t.galley.text().contains("needle") => Some(t.galley.size().x),
        _ => None,
    });
    out.drop_without_applying_deltas(); // discard frame, keep memory
}
```

Precedent: `pickers/icon_grid.rs` `item_tooltip_width_does_not_ratchet` — compares first-frame vs last-frame galley width.

## Pattern 2 — surface geometry (popups that fill the window, bleeds)

```rust
let rect = ctx.memory(|m| m.area_rect(egui::Id::new("sqyre_permissions_intro"))).unwrap();
let bounds = crate::widgets::dialog_constrain_rect(&ctx);
assert!(rect.height() < bounds.height() * 0.4); // hugs content, not fills window
```

- **Strokes:** scan `out.shapes` for the selection color and assert no `StrokeKind::Outside` (precedent: `icon_grid` `selected_cell_stroke_stays_inside_cell`).
- **Horizontal overflow:** assert painted shapes stay within the tip/popup `area_rect` (precedent: sequoia-ship tooltip-image fix).

## Pattern 3 — popups/menus that must stay open

Nested popups live on separate layers — the bug is dismissal, not paint. Drive via the Harness (`ui_interaction.rs`): open the parent popup, click a nested control (tag suggestion, right-click a chip), `run_steps(2)`, assert the parent's widget still exists via `query_all_by_role`/`get_by_label`. `Memory::popups` holds one slot — assert parent **and** child both visible.

## Pattern 4 — keyboard / focus

```rust
harness.get_by_label("Demo Program").click(); harness.run();
name_node.focus(); name_node.type_text(" Renamed"); harness.run();
harness.key_press(egui::Key::Enter); harness.run();
assert!(harness.state().docs_catalog().get("Demo Program Renamed").is_some());
```

- For **focus widgets** (splitters, custom controls): `harness.key_press(Tab)` or directional arrows, then assert `has_focus` behavior took effect. Remember `ui.interact` doesn't request focus — the widget must `request_focus()` itself (see `egui-layout-gotchas`).
- For **layer-gated keys**: reproduce the stale-`Area` case — open a Foreground dialog (palette/confirm), Esc it, `run_steps(4)`, then press the gated key. Areas linger in egui's order after closing (precedent: `data_editor_enter_saves_after_foreground_dialog_closed`).

## Pattern 5 — persisted UI state

`macro_list_split.rs` pattern: drive the gesture (splitter drag / toggle), trigger save, re-create the widget state from the store, assert restored. The second half of every persistence test: **run idle frames under a different window size and assert the stored value did not drift** — per-frame derived write-back is the regression this guards.

## Rules of thumb

- Loop frames — most egui bugs are cross-frame (ratchets, stale areas, settling).
- Assert on **painted output** (`out.shapes`, `area_rect`) for layout claims and on **app state** (`harness.state()`) for behavior claims.
- Keep the regression test next to the fix commit so `git bisect` lands on a commit whose test fails.
