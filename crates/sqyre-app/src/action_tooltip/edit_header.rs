//! Shared header chrome for tree action edit and Add Action defaults edit.

use crate::tree_chrome;
use crate::widgets::{save_cancel_row, SaveCancel};
use eframe::egui::{self, Color32};

/// Pill + optional subtitle + Save/Cancel, then separator. `error` becomes a
/// hover-disclosed icon beside the header text instead of its own line.
pub fn paint_action_edit_header(
    ui: &mut egui::Ui,
    label: &str,
    pastel: Color32,
    subtitle: Option<&str>,
    error: Option<&str>,
    save_enabled: bool,
) -> SaveCancel {
    let outcome = crate::widgets::show_overflow(ui, "action_edit_header", 1.0, |ui| {
        let mut outcome = SaveCancel::None;
        ui.horizontal(|ui| {
            tree_chrome::paint_pill_pub(ui, label, pastel);
            if let Some(text) = subtitle {
                ui.label(text);
            }
            if let Some(err) = error {
                crate::widgets::error_icon(ui, err);
            }
            outcome = save_cancel_row(ui, save_enabled);
        });
        outcome
    });
    crate::widgets::section_separator(ui);
    outcome
}
