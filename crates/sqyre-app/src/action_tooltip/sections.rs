//! Framed tooltip sections with wrapping rows.
//!
//! # Conventions
//!
//! - [`tip_wrapped_section`] — compact labeled fields, combos, checkboxes, DragValues.
//! - [`tip_section`] — multiline editors, icon grids, previews, repeatable list editors.
//! - [`tip_advanced`] — collapsed-by-default “Advanced” header; fill with tip_* sections.
//! - Field labels use `help::label` + control (not `DragValue.prefix`).
//! - Do not nest `ui.group` inside a tip section; list headers + rows (optional light
//!   per-item frame only for multi-line list items).

use crate::theme;
use eframe::egui::{self, Vec2};

/// Vertical gap between consecutive tip sections.
const SECTION_GAP: f32 = 4.0;

/// Sqyre-framed section (full-width vertical content).
pub fn tip_section(ui: &mut egui::Ui, add_contents: impl FnOnce(&mut egui::Ui)) {
    theme::framed_section(ui, SECTION_GAP, add_contents);
}

/// Framed section whose children flow left-to-right and wrap.
///
/// Prefer this for compact fields (pills, checkboxes, drag values, short labeled edits).
/// Multi-line editors and icon grids should use [`tip_section`] instead so they stay full-width.
pub fn tip_wrapped_section(ui: &mut egui::Ui, add_contents: impl FnOnce(&mut egui::Ui)) {
    tip_section(ui, |ui| {
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = Vec2::splat(6.0);
            add_contents(ui);
        });
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
    let response = egui::CollapsingHeader::new("Advanced")
        .default_open(false)
        .id_salt(ADVANCED_ID_SALT)
        .show(ui, |ui| {
            add_contents(ui);
        });
    // `changed` is set when the open state toggles (click or `.open(…)`).
    if response.header_response.changed() {
        let open =
            egui::collapsing_header::CollapsingState::load_with_default_open(ui.ctx(), id, false)
                .is_open();
        // Snap the open animation so body height is final next frame (otherwise
        // auto-fit chases a tween and can settle too early).
        let _ = ui.ctx().animate_bool_with_time(id, open, 0.0);
        ui.ctx()
            .data_mut(|d| d.insert_temp(edit_tip_refit_id(), true));
        ui.ctx().request_repaint();
    }
    ui.add_space(SECTION_GAP);
}
