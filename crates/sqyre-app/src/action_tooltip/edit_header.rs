//! Shared header chrome for tree action edit and Add Action defaults edit.

use crate::tree_chrome;
use crate::widgets::{save_row, SaveCancel};
use eframe::egui::{self, Color32};

/// Pill + optional subtitle + right-aligned Save, then separator. `error` becomes a
/// hover-disclosed icon beside the header text instead of its own line.
///
/// When `show_close` is set, a Close button sits at the right edge of the row.
pub fn paint_action_edit_header(
    ui: &mut egui::Ui,
    label: &str,
    pastel: Color32,
    subtitle: Option<&str>,
    error: Option<&str>,
    save_enabled: bool,
    show_close: bool,
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
            if show_close {
                // Close is the rightmost control; Save sits just to its left.
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if crate::widgets::cancel_icon_button(ui, crate::help_text::EDIT_TIP_CLOSE)
                        .clicked()
                    {
                        outcome = SaveCancel::Cancel;
                    }
                    let row = save_row(ui, save_enabled);
                    if outcome == SaveCancel::None {
                        outcome = row;
                    }
                });
            } else {
                outcome = save_row(ui, save_enabled);
            }
        });
        outcome
    });
    crate::widgets::section_separator(ui);
    outcome
}
