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
    save_enabled: bool,
) -> SaveCancel {
    let outcome = crate::widgets::show_overflow(ui, "action_edit_header", 1.0, |ui| {
        let mut outcome = SaveCancel::None;
        ui.horizontal(|ui| {
            tree_chrome::paint_pill_pub(ui, label, pastel);
            if let Some(text) = subtitle {
                ui.label(text);
            }
            outcome = save_cancel_row(ui, save_enabled);
        });
        outcome
    });
    if let Some(err) = error {
        // Wrap to the painted width: inside `fill_resize_body`, `available_width`
        // reaches toward `max_size`, so a long one-line error would clip right.
        let w = crate::widgets::visible_width(ui);
        ui.horizontal(|ui| {
            ui.set_max_width(w);
            ui.add(
                egui::Label::new(egui::RichText::new(err).color(crate::theme::error_fg())).wrap(),
            );
        });
    }
    crate::widgets::section_separator(ui);
    outcome
}
