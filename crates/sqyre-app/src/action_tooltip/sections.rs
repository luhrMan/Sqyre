//! Framed tooltip sections with wrapping rows.

use crate::theme;
use eframe::egui::{self, Vec2};

/// Vertical gap between consecutive tip sections.
const SECTION_GAP: f32 = 4.0;

/// Sqyre-framed section.
pub fn tip_section(ui: &mut egui::Ui, add_contents: impl FnOnce(&mut egui::Ui)) {
    theme::section_frame(ui.style()).show(ui, |ui| {
        ui.set_min_width(ui.available_width());
        add_contents(ui);
    });
    ui.add_space(SECTION_GAP);
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
