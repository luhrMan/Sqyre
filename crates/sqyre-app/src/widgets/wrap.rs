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
        ui.spacing_mut().item_spacing = egui::Vec2::splat(crate::theme::SPACE_8);
        add_contents(ui)
    })
}

/// Lays out `add_contents` left-to-right as one unit that never splits across rows.
///
/// Inside a wrapping parent the unit starts a new row when its width (remembered
/// from the previous pass) does not fit the space left on the current row. An
/// unmeasured unit that is not at the row start goes to a new row and the pass is
/// discarded, so neither a split nor an overflow is ever shown. Outside wrapping
/// layouts this is a plain `ui.horizontal`.
pub fn wrap_unit<R>(
    ui: &mut egui::Ui,
    add_contents: impl FnOnce(&mut egui::Ui) -> R,
) -> egui::InnerResponse<R> {
    let layout = ui.layout();
    if !(layout.main_wrap && layout.main_dir == egui::Direction::LeftToRight) {
        return ui.horizontal(add_contents);
    }
    let width_id = ui.next_auto_id().with("wrap_unit_width");
    let at_row_start = ui.cursor().min.x <= ui.max_rect().left() + 0.5;
    if !at_row_start {
        match ui.ctx().data(|d| d.get_temp::<f32>(width_id)) {
            Some(w) if w <= ui.available_size_before_wrap().x + 0.5 => {}
            Some(_) => ui.end_row(),
            None => {
                ui.end_row();
                ui.ctx().request_discard("new wrap_unit");
            }
        }
    }
    let inner = ui.horizontal(add_contents);
    let w = inner.response.rect.width();
    ui.ctx().data_mut(|d| d.insert_temp(width_id, w));
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
