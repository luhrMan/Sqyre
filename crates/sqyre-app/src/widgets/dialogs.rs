//! Shared dialog chrome: Save/Cancel and Confirm/Cancel rows.

use eframe::egui::{self, Key, Modifiers};

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

/// Esc → cancel, Enter → submit for top-level confirm popups.
///
/// Keys are consumed so they do not leak to the UI under the dialog.
pub fn poll_confirm_keys(ui: &mut egui::Ui) -> ConfirmCancel {
    if ui.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Escape)) {
        ConfirmCancel::Cancel
    } else if ui.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Enter)) {
        ConfirmCancel::Confirm
    } else {
        ConfirmCancel::None
    }
}

/// Cancel + Confirm for destructive / overwrite prompts.
///
/// `Enter` confirms and `Esc` cancels.
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
    if out == ConfirmCancel::None {
        out = poll_confirm_keys(ui);
    }
    out
}
