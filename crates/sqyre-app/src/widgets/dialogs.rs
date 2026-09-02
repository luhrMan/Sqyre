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

fn apply_dialog_bounds<'a>(window: egui::Window<'a>, ctx: &egui::Context) -> egui::Window<'a> {
    let rect = dialog_constrain_rect(ctx);
    window.constrain_to(rect).max_size(rect.size())
}

/// Visible layout budget for a Ui — not leftover room toward a Window `max_size`.
///
/// egui [`Window`]s auto-expand to content `min_size` and do not shrink. Dialogs
/// use [`fit_dialog_window`] / [`fit_dialog_popup`], which set `max_size` to the
/// screen constrain rect. Widgets that treat `available_width()` as a **minimum**
/// (`set_width` / `set_min_width` / `TextEdit::desired_width` / column counts /
/// `ScrollArea::auto_shrink([false, _])` without a max) therefore grow the window
/// every frame until it hits the screen edge.
///
/// Prefer the painted region (`clip_rect` ∩ `max_rect`), then clamp to available
/// and the dialog constrain rect. Call [`pin_visible`] at the start of a Window
/// body when children size themselves from available space.
pub fn visible_size(ui: &egui::Ui) -> egui::Vec2 {
    let constrain = dialog_constrain_rect(ui.ctx()).size();
    let clip = ui.clip_rect().size();
    let max_rect = ui.max_rect().size();
    let avail = ui.available_size();
    egui::vec2(
        visible_axis(avail.x, clip.x, max_rect.x, constrain.x),
        visible_axis(avail.y, clip.y, max_rect.y, constrain.y),
    )
}

fn visible_axis(avail: f32, clip: f32, max_rect: f32, constrain: f32) -> f32 {
    let cap = constrain.max(1.0);
    // Prefer the current painted region. `available_*` can include leftover room
    // toward Window::max_size and must not drive min_size.
    let current = clip.min(max_rect);
    let v = if current.is_finite() && current > 1.0 {
        current.min(avail).min(cap)
    } else if avail.is_finite() && avail > 1.0 {
        avail.min(cap)
    } else {
        (cap * 0.4).max(1.0)
    };
    v.clamp(1.0, cap)
}

/// [`visible_size`].x
pub fn visible_width(ui: &egui::Ui) -> f32 {
    visible_size(ui).x
}

/// [`visible_size`].y
pub fn visible_height(ui: &egui::Ui) -> f32 {
    visible_size(ui).y
}

/// Cap this Ui so children cannot ratchet a Window toward `max_size`.
///
/// Prefer fixing the child instead when possible: `ScrollArea::auto_shrink([false, _])`
/// and `set_width(available)` set `min_size` equal to the current window size, which
/// both grows the window and blocks shrinking. Use `auto_shrink([true, …])` and
/// `TextEdit::desired_width(f32::INFINITY)` for fill-without-lock layouts.
pub fn pin_visible(ui: &mut egui::Ui) {
    ui.set_max_size(visible_size(ui));
}

/// Fill a resizable [`egui::Window`] body without ratcheting its Resize state.
///
/// Allocates exactly the current Resize region (`max_rect`) and runs `add_body`
/// clipped inside it. Child fill layouts then cannot report a `min_size` larger
/// than the window — which is what makes egui Windows grow every frame and
/// refuse to shrink.
///
/// Use with [`fit_dialog_popup`] (not [`fit_dialog_window`]): an outer Window
/// scroll makes `available_size` track `max_size` and defeats this.
pub fn fill_resize_body(ui: &mut egui::Ui, add_body: impl FnOnce(&mut egui::Ui)) {
    let size = ui.max_rect().size().max(egui::vec2(1.0, 1.0));
    let (rect, _) = ui.allocate_exact_size(size, egui::Sense::hover());
    ui.scope_builder(egui::UiBuilder::new().max_rect(rect), |ui| {
        ui.set_clip_rect(rect);
        ui.set_max_size(rect.size());
        add_body(ui);
    });
}

/// Keep a floating dialog inside [`dialog_constrain_rect`], scrolling overflow.
///
/// egui allows windows larger than their constrain rect to be panned (overhang
/// drag), which looks like the contents slide while the frame stays put. Cap
/// size and scroll overflow inside the frame so that cannot happen.
///
/// Vertical scroll only — horizontal outer bars fight nested layouts that claim
/// `available_width` (clipped buttons, phantom H scrollbars). Panels that fully
/// manage their own scroll/split layout should use [`fit_dialog_popup`] instead.
///
/// Use for resizable / default-sized panels. For small auto-sized popups use
/// [`fit_dialog_popup`] so they still shrink to their content.
pub fn fit_dialog_window<'a>(window: egui::Window<'a>, ctx: &egui::Context) -> egui::Window<'a> {
    apply_dialog_bounds(window, ctx)
        .scroll([false, true])
        .scroll_bar_visibility(ScrollBarVisibility::VisibleWhenNeeded)
        .drag_to_scroll(DragScroll::Never)
}

/// Like [`fit_dialog_window`] but without an outer scroll area — for compact
/// auto-sized confirms / record modals, and for panes that allocate their own
/// body/footer and inner [`ScrollArea`]s (data editor, settings).
pub fn fit_dialog_popup<'a>(window: egui::Window<'a>, ctx: &egui::Context) -> egui::Window<'a> {
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
