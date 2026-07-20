//! Shared header chrome for tree action edit and Add Action defaults edit.

use crate::tree_chrome;
use crate::widgets::{save_cancel_row, SaveCancel};
use eframe::egui::{self, Color32};

/// Pill + optional subtitle + Save/Cancel, then error line and separator.
pub fn paint_action_edit_header(
    ui: &mut egui::Ui,
    label: &str,
    pastel: Color32,
    subtitle: Option<&str>,
    error: Option<&str>,
) -> SaveCancel {
    let mut outcome = SaveCancel::None;
    ui.horizontal(|ui| {
        tree_chrome::paint_pill_pub(ui, label, pastel);
        if let Some(text) = subtitle {
            ui.label(text);
        }
        outcome = save_cancel_row(ui);
    });
    if let Some(err) = error {
        ui.colored_label(crate::theme::error_fg(), err);
    }
    ui.separator();
    outcome
}
