//! Wrapping rows whose widget groups (label + control, icon + pill, …) wrap as one unit.
//!
//! # Conventions
//!
//! - [`wrapped_row`] — left-to-right row that wraps, with the standard field gap.
//! - [`wrap_unit`] — one group that must stay on a single row; use for every
//!   label + control pair (field helpers in [`super::fields`] already do).
//! - Do not put a bare `ui.horizontal` / `ui.vertical` group inside a wrapping row:
//!   it is sized to the space left on the current row, so it overflows the pane
//!   instead of moving to the next row.

use eframe::egui;

/// Left-to-right row that wraps, spaced like tip sections and form rows.
pub fn wrapped_row<R>(
    ui: &mut egui::Ui,
    add_contents: impl FnOnce(&mut egui::Ui) -> R,
) -> egui::InnerResponse<R> {
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing = egui::Vec2::splat(crate::theme::element_gap(ui));
        add_contents(ui)
    })
}

/// Lays out `add_contents` left-to-right as one unit that never splits across rows.
///
/// Inside a wrapping parent the unit starts a new row when its width does not fit
/// the space left on the current row. An unmeasured unit that is not at the row
/// start is measured in an invisible probe (instead of discarding the frame — a
/// discarded frame three passes in a row paints egui's PERF WARNING overlay), so
/// neither a split nor an overflow is ever shown. The group is placed at its
/// own measured size: `ui.horizontal` is only `interact_size` tall, and a
/// wrapping row centers that short box, so each later unit starts lower.
///
/// A non-wrapping horizontal row (left-to-right or right-to-left) has the same
/// centering, so each unit requests its measured height. Width still follows
/// the content on this frame. A vertical parent has no sibling to align and
/// stays a plain `ui.horizontal`.
pub fn wrap_unit<R>(
    ui: &mut egui::Ui,
    mut add_contents: impl FnMut(&mut egui::Ui) -> R,
) -> egui::InnerResponse<R> {
    let layout = ui.layout();
    let ltr = layout.main_dir == egui::Direction::LeftToRight;
    let horizontal = ltr || layout.main_dir == egui::Direction::RightToLeft;
    if !horizontal {
        return ui.horizontal(add_contents);
    }
    if !(layout.main_wrap && ltr) {
        return place_in_horizontal_row(ui, add_contents);
    }
    let size_id = ui.next_auto_id().with("wrap_unit_size");
    let at_row_start = ui.cursor().min.x <= ui.max_rect().left() + 0.5;
    let mut size = ui.ctx().data(|d| d.get_temp::<[f32; 2]>(size_id));
    if !at_row_start {
        let measured = size.unwrap_or_else(|| {
            let mut probe = ui.new_child(egui::UiBuilder::new().invisible());
            let rect = probe.horizontal(&mut add_contents).response.rect;
            let measured = [rect.width(), rect.height()];
            ui.ctx().data_mut(|d| d.insert_temp(size_id, measured));
            measured
        });
        size = Some(measured);
        if measured[0] > ui.available_size_before_wrap().x + 0.5 {
            ui.end_row();
        }
    }
    let place = egui::Layout::left_to_right(egui::Align::Center);
    // A caller can raise `interact_size.y` to put every unit on one band
    // (summary pills). Content taller than that still keeps its own height,
    // so a short interact size cannot pull the next unit down.
    let band_h = ui.spacing().interact_size.y;
    let inner = match size.filter(|s| s[0] > 0.0 && s[1] > 0.0) {
        Some([w, h]) => {
            let h = h.max(band_h);
            ui.allocate_ui_with_layout(egui::vec2(w, h), place, |ui| {
                ui.set_min_height(h);
                add_contents(ui)
            })
        }
        None => ui.with_layout(place, &mut add_contents),
    };
    let rect = inner.response.rect;
    ui.ctx()
        .data_mut(|d| d.insert_temp(size_id, [rect.width(), rect.height()]));
    inner
}

/// Place one unit in a non-wrapping horizontal row at its real height.
///
/// `ui.horizontal` asks only for `interact_size.y`. The row then centers that
/// short box in the band grown by the previous unit, so each next unit starts
/// lower. Requesting the measured height keeps every unit on one baseline
/// without freezing the width (a name field has to grow on the same frame).
fn place_in_horizontal_row<R>(
    ui: &mut egui::Ui,
    mut add_contents: impl FnMut(&mut egui::Ui) -> R,
) -> egui::InnerResponse<R> {
    let size_id = ui.next_auto_id().with("wrap_unit_size");
    let known = ui.ctx().data(|d| d.get_temp::<f32>(size_id));
    let slot_h = known.unwrap_or_else(|| {
        let mut probe = ui.new_child(egui::UiBuilder::new().invisible());
        let height = probe.horizontal(&mut add_contents).response.rect.height();
        ui.ctx().data_mut(|d| d.insert_temp(size_id, height));
        height
    });
    let saved = ui.spacing().interact_size;
    let mut interact = saved;
    interact.y = interact.y.max(slot_h);
    ui.spacing_mut().interact_size = interact;
    let inner = ui.horizontal(|ui| {
        ui.spacing_mut().interact_size = saved;
        add_contents(ui)
    });
    ui.spacing_mut().interact_size = saved;
    let height = inner.response.rect.height();
    ui.ctx().data_mut(|d| d.insert_temp(size_id, height));
    inner
}

#[cfg(test)]
mod tests {
    use super::*;

    const PANE_W: f32 = 260.0;

    /// Two passes so every unit has a remembered width.
    fn layout_pairs(ctx: &egui::Context) -> Vec<(egui::Rect, egui::Rect)> {
        let mut pairs = Vec::new();
        for _ in 0..2 {
            ctx.run_ui(egui::RawInput::default(), |ui| {
                pairs.clear();
                let pane = egui::Rect::from_min_size(ui.max_rect().min, egui::vec2(PANE_W, 400.0));
                let mut pane_ui = ui.new_child(egui::UiBuilder::new().max_rect(pane));
                wrapped_row(&mut pane_ui, |ui| {
                    for label in ["Tolerance", "Blur", "Min threshold", "Resize factor"] {
                        let mut v = 0.0_f32;
                        let (l, c) = wrap_unit(ui, |ui| {
                            let l = ui.label(label).rect;
                            let c = ui.add(egui::DragValue::new(&mut v)).rect;
                            (l, c)
                        })
                        .inner;
                        pairs.push((l, c));
                    }
                });
                assert!(
                    pane_ui.min_rect().width() <= PANE_W + 0.5,
                    "wrapped row widened pane to {}",
                    pane_ui.min_rect().width()
                );
            })
            .drop_without_applying_deltas();
        }
        pairs
    }

    #[test]
    fn pairs_wrap_together_inside_pane() {
        let ctx = egui::Context::default();
        let pairs = layout_pairs(&ctx);
        let origin_x = pairs[0].0.left();
        let mut rows = 1;
        for (i, (label, control)) in pairs.iter().enumerate() {
            assert!(
                (label.center().y - control.center().y).abs() < 1.0,
                "pair {i} split: {label:?} / {control:?}"
            );
            assert!(
                control.right() <= origin_x + PANE_W + 0.5,
                "pair {i} overflows pane: {control:?}"
            );
            if i > 0 && label.top() > pairs[i - 1].0.bottom() {
                rows += 1;
            }
        }
        assert!(rows > 1, "narrow pane should wrap pairs onto more rows");
    }

    #[test]
    fn units_on_one_row_share_a_baseline() {
        let ctx = egui::Context::default();
        let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(900.0, 200.0));
        let mut rects = Vec::new();
        for pass in 0..2 {
            ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(screen),
                    ..Default::default()
                },
                |ui| {
                    rects.clear();
                    wrapped_row(ui, |ui| {
                        for _ in 0..3 {
                            // Taller than `interact_size.y`, which is what made
                            // each next unit step down.
                            let r = wrap_unit(ui, |ui| {
                                ui.allocate_exact_size(egui::vec2(80.0, 24.0), egui::Sense::hover())
                                    .1
                            });
                            rects.push(r.response.rect);
                        }
                    });
                },
            )
            .drop_without_applying_deltas();
            let top = rects[0].top();
            for (i, rect) in rects.iter().enumerate() {
                assert!(
                    (rect.top() - top).abs() < 1.0,
                    "unit {i} stepped down on pass {pass}: {rects:?}"
                );
            }
        }
    }

    #[test]
    fn units_in_a_plain_horizontal_share_a_baseline() {
        let ctx = egui::Context::default();
        let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(900.0, 200.0));
        let mut rects = Vec::new();
        for pass in 0..2 {
            ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(screen),
                    ..Default::default()
                },
                |ui| {
                    rects.clear();
                    ui.vertical(|ui| {
                        ui.horizontal(|ui| {
                            for _ in 0..3 {
                                let r = wrap_unit(ui, |ui| {
                                    ui.allocate_exact_size(
                                        egui::vec2(80.0, 24.0),
                                        egui::Sense::hover(),
                                    )
                                    .1
                                });
                                rects.push(r.response.rect);
                            }
                        });
                    });
                },
            )
            .drop_without_applying_deltas();
            let top = rects[0].top();
            for (i, rect) in rects.iter().enumerate() {
                assert!(
                    (rect.top() - top).abs() < 1.0,
                    "unit {i} stepped down on pass {pass}: {rects:?}"
                );
            }
        }
    }

    #[test]
    fn units_in_a_right_to_left_row_share_a_baseline() {
        let ctx = egui::Context::default();
        let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(900.0, 200.0));
        let mut rects = Vec::new();
        for pass in 0..2 {
            ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(screen),
                    ..Default::default()
                },
                |ui| {
                    rects.clear();
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        for _ in 0..3 {
                            let r = wrap_unit(ui, |ui| {
                                ui.allocate_exact_size(egui::vec2(80.0, 24.0), egui::Sense::hover())
                                    .1
                            });
                            rects.push(r.response.rect);
                        }
                    });
                },
            )
            .drop_without_applying_deltas();
            let top = rects[0].top();
            for (i, rect) in rects.iter().enumerate() {
                assert!(
                    (rect.top() - top).abs() < 1.0,
                    "unit {i} stepped down on pass {pass}: {rects:?}"
                );
            }
        }
    }

    #[test]
    fn plain_horizontal_outside_wrapping_layout() {
        let ctx = egui::Context::default();
        ctx.run_ui(egui::RawInput::default(), |ui| {
            let before = ui.cursor().min;
            let r = wrap_unit(ui, |ui| ui.label("x").rect).response.rect;
            assert!((r.top() - before.y).abs() < 0.5);
        })
        .drop_without_applying_deltas();
    }
}
