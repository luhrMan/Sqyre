//! Framed tooltip sections with wrapping rows.
//!
//! # Conventions
//!
//! - [`tip_wrapped_section`] — compact labeled fields, combos, checkboxes, numeric
//!   entries ([`crate::widgets::num_entry`]). Each label + control pair is one
//!   [`crate::widgets::wrap_unit`] so wrapping never splits it; shared field
//!   helpers already do this.
//! - [`tip_section`] — multiline editors, icon grids, previews, repeatable list editors.
//! - [`tip_advanced`] — collapsed-by-default “Advanced” header; fill with tip_* sections.
//! - Field labels use `help::label` + control (not `NumEntry.prefix`).
//! - Do not nest `ui.group` inside a tip section; list headers + rows (optional light
//!   per-item frame only for multi-line list items).

use crate::theme;
use eframe::egui;

/// Sqyre-framed section (full-width vertical content).
pub fn tip_section(ui: &mut egui::Ui, add_contents: impl FnOnce(&mut egui::Ui)) {
    theme::framed_section(ui, add_contents);
}

/// Framed section whose children flow left-to-right and wrap.
///
/// Prefer this for compact fields (pills, checkboxes, drag values, short labeled edits).
/// Multi-line editors and icon grids should use [`tip_section`] instead so they stay full-width.
pub fn tip_wrapped_section(ui: &mut egui::Ui, add_contents: impl FnOnce(&mut egui::Ui)) {
    tip_section(ui, |ui| {
        crate::widgets::wrapped_row(ui, add_contents);
    });
}

const ADVANCED_ID_SALT: &str = "tip_advanced";

fn edit_tip_refit_id() -> egui::Id {
    egui::Id::new("sqyre_action_edit_tip_refit")
}

/// Consume a pending edit-tooltip height re-fit request (set by [`tip_advanced`]).
pub(crate) fn take_edit_tip_refit(ctx: &egui::Context) -> bool {
    ctx.data_mut(|d| d.remove_temp::<bool>(edit_tip_refit_id()).unwrap_or(false))
}

/// Collapsed-by-default Advanced header. Prefer tip_* sections inside for framing.
///
/// Clicking the header requests an edit-tooltip height re-fit so newly shown
/// (or hidden) widgets resize the window instead of only scrolling.
pub fn tip_advanced(ui: &mut egui::Ui, add_contents: impl FnOnce(&mut egui::Ui)) {
    let id = ui.make_persistent_id(ADVANCED_ID_SALT);
    let response = crate::widgets::advanced_section(ui, ADVANCED_ID_SALT, add_contents);
    // `changed` is set when the open state toggles (click or `.open(…)`).
    let open =
        egui::collapsing_header::CollapsingState::load_with_default_open(ui.ctx(), id, false)
            .is_open();
    if response.header_response.changed() {
        // Snap the open animation so body height is final next frame (otherwise
        // auto-fit chases a tween and can settle too early).
        let _ = ui.ctx().animate_bool_with_time(id, open, 0.0);
        ui.ctx()
            .data_mut(|d| d.insert_temp(edit_tip_refit_id(), true));
        ui.ctx().request_repaint();
    }
    // Open bodies already end with `framed_section` / tip_* trailing gaps;
    // pairing another `add_section_gap` here double-spaces the tip (A3).
    // Collapsed headers still need one gap before the next sibling.
    if !open {
        theme::add_section_gap(ui);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::{section_gap, SPACE_8};

    fn empty_tip_advanced_height(ctx: &egui::Context, open: bool) -> f32 {
        let mut height = 0.0;
        // Two frames: first stores open state via make_persistent_id path, second measures.
        for frame in 0..2 {
            ctx.run_ui(egui::RawInput::default(), |ui| {
                let id = ui.make_persistent_id(ADVANCED_ID_SALT);
                let mut state = egui::collapsing_header::CollapsingState::load_with_default_open(
                    ui.ctx(),
                    id,
                    false,
                );
                state.set_open(open);
                state.store(ui.ctx());
                let y0 = ui.cursor().min.y;
                tip_advanced(ui, |_| {});
                if frame == 1 {
                    height = ui.cursor().min.y - y0;
                }
            })
            .drop_without_applying_deltas();
        }
        height
    }

    #[test]
    fn tip_advanced_skips_trailing_gap_when_open() {
        let ctx = egui::Context::default();
        crate::theme::set_density(&ctx, crate::theme::DensityGaps::new(SPACE_8, 4.0));
        // `add_section_gap` tops up past `item_spacing`, so measure that pad — not the raw token.
        let pad = {
            let mut p = 0.0;
            ctx.run_ui(egui::RawInput::default(), |ui| {
                ui.label("prior");
                let y0 = ui.cursor().min.y;
                crate::theme::add_section_gap(ui);
                p = ui.cursor().min.y - y0;
                let _ = section_gap(ui);
            })
            .drop_without_applying_deltas();
            p
        };
        let closed = empty_tip_advanced_height(&ctx, false);
        let open = empty_tip_advanced_height(&ctx, true);
        // Empty open body: header only (no trailing gap). Closed: header + section pad.
        assert!(
            (closed - open - pad).abs() < 1.0,
            "closed tip_advanced should add exactly one section pad (closed={closed}, open={open}, pad={pad})"
        );
    }
}
