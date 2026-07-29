//! Shared dialog chrome: Save/Cancel and Confirm/Cancel rows.

use eframe::egui::{self, Key, Modifiers};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SaveCancel {
    None,
    Save,
    Cancel,
}

/// Right-aligned Cancel then Save (Save on the right in LTR via right_to_left).
///
/// Save glows while `save_enabled` (dirty pending work).
pub fn save_cancel_row(ui: &mut egui::Ui, save_enabled: bool) -> SaveCancel {
    let mut out = SaveCancel::None;
    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
        if ui.button("Cancel").clicked() {
            out = SaveCancel::Cancel;
        }
        if crate::theme::dirty_action_button(ui, "Save", save_enabled).clicked() {
            out = SaveCancel::Save;
        }
    });
    out
}

/// Left-to-right Cancel + Save (variables / forms that prefer that order).
///
/// Save glows while `save_enabled` (dirty pending work).
pub fn save_cancel_row_ltr(ui: &mut egui::Ui, save_enabled: bool) -> SaveCancel {
    let mut out = SaveCancel::None;
    ui.horizontal(|ui| {
        if crate::theme::dirty_action_button(ui, "Save", save_enabled).clicked() {
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

/// Centered, non-resizable popup chrome shared by delete/overwrite confirm prompts.
///
/// Runs `body` inside the window and returns `false` once the user has closed
/// it via the titlebar close control (callers should clear their pending-confirm
/// state in that case, same as an explicit Cancel).
pub fn confirm_window(ctx: &egui::Context, title: &str, body: impl FnOnce(&mut egui::Ui)) -> bool {
    let mut open = true;
    egui::Window::new(title)
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .order(egui::Order::Foreground)
        .open(&mut open)
        .show(ctx, body);
    open
}
