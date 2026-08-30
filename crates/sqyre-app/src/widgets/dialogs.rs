//! Shared dialog chrome: Save/Cancel and Confirm/Cancel rows.

use eframe::egui::{self, Key, Modifiers};
use egui::containers::scroll_area::{DragScroll, ScrollBarVisibility};

/// Keep floating dialogs at least this fraction of the viewport away from each edge.
pub const DIALOG_EDGE_MARGIN_FRAC: f32 = 0.025;

/// [`egui::Context::content_rect`] inset by [`DIALOG_EDGE_MARGIN_FRAC`] on each side.
pub fn dialog_constrain_rect(ctx: &egui::Context) -> egui::Rect {
    let rect = ctx.content_rect();
    rect.shrink2(egui::vec2(
        rect.width() * DIALOG_EDGE_MARGIN_FRAC,
        rect.height() * DIALOG_EDGE_MARGIN_FRAC,
    ))
}

fn apply_dialog_bounds<'a>(
    window: egui::Window<'a>,
    ctx: &egui::Context,
) -> egui::Window<'a> {
    let rect = dialog_constrain_rect(ctx);
    window.constrain_to(rect).max_size(rect.size())
}

/// Keep a floating dialog inside [`dialog_constrain_rect`], scrolling overflow.
///
/// egui allows windows larger than their constrain rect to be panned (overhang
/// drag), which looks like the contents slide while the frame stays put. Cap
/// size and scroll overflow inside the frame so that cannot happen.
///
/// Use for resizable / default-sized panels. For small auto-sized popups use
/// [`fit_dialog_popup`] so they still shrink to their content.
pub fn fit_dialog_window<'a>(
    window: egui::Window<'a>,
    ctx: &egui::Context,
) -> egui::Window<'a> {
    apply_dialog_bounds(window, ctx)
        .scroll([true, true])
        .scroll_bar_visibility(ScrollBarVisibility::VisibleWhenNeeded)
        .drag_to_scroll(DragScroll::Never)
}

/// Like [`fit_dialog_window`] but without an outer scroll area — for compact
/// auto-sized confirms / record modals that must size to their content.
pub fn fit_dialog_popup<'a>(
    window: egui::Window<'a>,
    ctx: &egui::Context,
) -> egui::Window<'a> {
    apply_dialog_bounds(window, ctx)
}

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
    fit_dialog_popup(
        egui::Window::new(title)
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .order(egui::Order::Foreground)
            .open(&mut open),
        ctx,
    )
    .show(ctx, body);
    open
}
