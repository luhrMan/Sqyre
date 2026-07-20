//! Shared dialog chrome: Save/Cancel and Confirm/Cancel rows.

use eframe::egui;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SaveCancel {
    None,
    Save,
    Cancel,
}

/// Right-aligned Cancel then Save (Save on the right in LTR via right_to_left).
pub fn save_cancel_row(ui: &mut egui::Ui) -> SaveCancel {
    let mut out = SaveCancel::None;
    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
        if ui.button("Cancel").clicked() {
            out = SaveCancel::Cancel;
        }
        if ui.button("Save").clicked() {
            out = SaveCancel::Save;
        }
    });
    out
}

/// Left-to-right Cancel + Save (variables / forms that prefer that order).
pub fn save_cancel_row_ltr(ui: &mut egui::Ui) -> SaveCancel {
    let mut out = SaveCancel::None;
    ui.horizontal(|ui| {
        if ui.button("Save").clicked() {
            out = SaveCancel::Save;
        }
        if ui.button("Cancel").clicked() {
            out = SaveCancel::Cancel;
        }
    });
    out
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfirmCancel {
    None,
    Confirm,
    Cancel,
}

/// Cancel + Confirm for destructive / overwrite prompts.
pub fn confirm_cancel_row(ui: &mut egui::Ui) -> ConfirmCancel {
    let mut out = ConfirmCancel::None;
    ui.horizontal(|ui| {
        if ui.button("Cancel").clicked() {
            out = ConfirmCancel::Cancel;
        }
        if ui.button("Confirm").clicked() {
            out = ConfirmCancel::Confirm;
        }
    });
    out
}
