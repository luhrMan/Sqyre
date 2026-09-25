//! Shared dialog chrome: Save/Cancel and Confirm/Cancel rows.

use eframe::egui::{self, Key, Modifiers};
use egui::containers::scroll_area::{DragScroll, ScrollBarVisibility};

/// Keep floating dialogs at least this fraction of the viewport away from each edge.
pub const DIALOG_EDGE_MARGIN_FRAC: f32 = 0.025;

/// Minimum content-rect size delta (points) before treating the OS window as resized.
const VIEWPORT_SIZE_EPSILON: f32 = 1.0;

/// Old/new [`egui::Context::content_rect`] for one frame after an OS window resize.
///
/// Floating dialogs apply this via [`fit_dialog_popup`] / [`fit_dialog_window`] so
/// their size and position keep the same fraction of the viewport.
#[derive(Debug, Clone, Copy)]
pub struct ViewportScaleEvent {
    pub old_content: egui::Rect,
    pub new_content: egui::Rect,
}

/// Track OS window size; set [`ViewportScaleEvent`] for one frame when it changes.
///
/// Steady-state (size unchanged): clear any stale pending event and return — no
/// allocations, no layer scans.
pub fn sync_viewport_window_scale(
    ctx: &egui::Context,
    last_content: &mut Option<egui::Rect>,
    pending: &mut Option<ViewportScaleEvent>,
) {
    let content = ctx.content_rect();
    let Some(prev) = *last_content else {
        *last_content = Some(content);
        *pending = None;
        return;
    };
    let dw = (content.width() - prev.width()).abs();
    let dh = (content.height() - prev.height()).abs();
    if dw <= VIEWPORT_SIZE_EPSILON && dh <= VIEWPORT_SIZE_EPSILON {
        *pending = None;
        return;
    }
    if prev.width() <= 1.0 || prev.height() <= 1.0 {
        *last_content = Some(content);
        *pending = None;
        return;
    }
    *pending = Some(ViewportScaleEvent {
        old_content: prev,
        new_content: content,
    });
    *last_content = Some(content);
}

fn scale_rect_with_viewport(rect: egui::Rect, event: &ViewportScaleEvent) -> egui::Rect {
    let old = event.old_content;
    let new = event.new_content;
    let sx = new.width() / old.width();
    let sy = new.height() / old.height();
    let min = egui::pos2(
        new.min.x + (rect.min.x - old.min.x) * sx,
        new.min.y + (rect.min.y - old.min.y) * sy,
    );
    let size = egui::vec2(rect.width() * sx, rect.height() * sy).max(egui::vec2(1.0, 1.0));
    egui::Rect::from_min_size(min, size)
}

/// One-frame clamp of Window min/max to the scaled outer rect so egui's private
/// Resize `desired_size` picks up the new size; position via `current_pos`.
fn apply_pending_viewport_scale<'a>(
    window: egui::Window<'a>,
    ctx: &egui::Context,
    id: egui::Id,
    pending: &ViewportScaleEvent,
) -> egui::Window<'a> {
    let Some(rect) = ctx.memory(|m| m.area_rect(id)) else {
        return window;
    };
    let scaled = scale_rect_with_viewport(rect, pending);
    let constrain = dialog_constrain_rect(ctx);
    let scaled = scaled.intersect(constrain);
    if scaled.width() < 1.0 || scaled.height() < 1.0 {
        return window;
    }
    let size = scaled.size();
    // min == max clamps Resize::desired_size this frame; next frame normal bounds restore.
    window.current_pos(scaled.min).min_size(size).max_size(size)
}

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

fn fit_dialog_common<'a>(
    window: egui::Window<'a>,
    ctx: &egui::Context,
    id: egui::Id,
    pending_scale: Option<&ViewportScaleEvent>,
) -> egui::Window<'a> {
    let window = window.id(id);
    let window = apply_dialog_bounds(window, ctx);
    match pending_scale {
        Some(pending) => apply_pending_viewport_scale(window, ctx, id, pending),
        None => window,
    }
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
/// and the dialog constrain rect. Prefer [`fill_resize_body`] for Window bodies
/// whose children size themselves from available space.
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

/// Right-edge width covered by a floating vertical scrollbar.
///
/// egui's default `floating_allocated_width` is 0, so the bar overlays content
/// without shrinking `available_width`. Subtract this when sizing columns or
/// anchoring chrome to the visible right edge.
pub fn floating_scrollbar_overlay_width(ui: &egui::Ui) -> f32 {
    let scroll = &ui.spacing().scroll;
    if scroll.floating {
        (scroll.bar_width - scroll.floating_allocated_width).max(0.0)
    } else {
        0.0
    }
}

/// [`visible_width`] minus [`floating_scrollbar_overlay_width`] (at least 1px).
pub fn visible_content_width(ui: &egui::Ui) -> f32 {
    (visible_width(ui) - floating_scrollbar_overlay_width(ui)).max(1.0)
}

/// Fill a resizable [`egui::Window`] body without ratcheting its Resize state.
///
/// Allocates exactly the painted window region ([`visible_size`]) and runs
/// `add_body` in a [`Ui::new_child`] so child `min_size` cannot advance the
/// parent (unlike [`Ui::scope_builder`], which would push edges off-screen).
///
/// Use with [`fit_dialog_popup`] (not [`fit_dialog_window`]): an outer Window
/// scroll makes `available_size` track `max_size` and defeats this.
pub fn fill_resize_body(ui: &mut egui::Ui, add_body: impl FnOnce(&mut egui::Ui)) {
    let size = visible_size(ui).max(egui::vec2(1.0, 1.0));
    let (rect, _) = ui.allocate_exact_size(size, egui::Sense::hover());
    let mut body = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(rect)
            .layout(egui::Layout::top_down(egui::Align::Min)),
    );
    body.set_clip_rect(rect.intersect(ui.clip_rect()));
    body.set_max_size(rect.size());
    add_body(&mut body);
}

/// Keep a floating dialog inside [`dialog_constrain_rect`], scrolling overflow
/// on **both** axes when content is larger than the window.
///
/// Prefer this for simple dialogs whose body is mostly one scroll region.
/// Panels that allocate their own split/scroll layout should use
/// [`fit_dialog_popup`] plus [`fill_resize_body`] and an inner
/// [`crate::pickers::dialog_scroll`].
pub fn fit_dialog_window<'a>(
    window: egui::Window<'a>,
    ctx: &egui::Context,
    id: egui::Id,
    pending_scale: Option<&ViewportScaleEvent>,
) -> egui::Window<'a> {
    fit_dialog_common(window, ctx, id, pending_scale)
        .scroll([true, true])
        .scroll_bar_visibility(ScrollBarVisibility::VisibleWhenNeeded)
        .drag_to_scroll(DragScroll::Never)
}

/// Like [`fit_dialog_window`] but without an outer scroll area — for compact
/// auto-sized confirms / record modals, and for panes that allocate their own
/// body/footer and inner [`ScrollArea`]s (data editor, settings).
pub fn fit_dialog_popup<'a>(
    window: egui::Window<'a>,
    ctx: &egui::Context,
    id: egui::Id,
    pending_scale: Option<&ViewportScaleEvent>,
) -> egui::Window<'a> {
    fit_dialog_common(window, ctx, id, pending_scale)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SaveCancel {
    None,
    Save,
    Cancel,
}

/// Esc → cancel; Enter → save when enabled and no text field wants keys.
fn poll_save_keys(ui: &mut egui::Ui, save_enabled: bool) -> SaveCancel {
    if ui.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Escape)) {
        SaveCancel::Cancel
    } else if save_enabled
        && !ui.ctx().egui_wants_keyboard_input()
        && !ui.ctx().text_edit_focused()
        && ui.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Enter))
    {
        SaveCancel::Save
    } else {
        SaveCancel::None
    }
}

/// Right-aligned footer: **Cancel left, Save right** (tip edit header).
///
/// Drawn with `right_to_left` so Save is allocated first (rightmost). Prefer this
/// for action tips and similar chrome. For the opposite visual order (Save left,
/// Cancel right), use [`save_cancel_row_ltr`] — do not mass-migrate call sites.
///
/// Save glows while `save_enabled` (dirty pending work). `Esc` cancels; `Enter`
/// saves only when `save_enabled` and egui is not routing keys to a text field.
pub fn save_cancel_row(ui: &mut egui::Ui, save_enabled: bool) -> SaveCancel {
    let mut out = SaveCancel::None;
    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
        // First in RTL = rightmost → Save on the right.
        if crate::theme::dirty_action_button(ui, "Save", save_enabled).clicked() {
            out = SaveCancel::Save;
        }
        if ui.button("Cancel").clicked() {
            out = SaveCancel::Cancel;
        }
    });
    if out == SaveCancel::None {
        out = poll_save_keys(ui, save_enabled);
    }
    out
}

/// Left-to-right footer: **Save left, Cancel right** (variables panel / forms).
///
/// Opposite button order from [`save_cancel_row`]. Keep both; pick the helper that
/// matches the surrounding panel rather than migrating for consistency alone.
///
/// Save glows while `save_enabled` (dirty pending work). Same `Esc` / `Enter`
/// rules as [`save_cancel_row`].
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
    if out == SaveCancel::None {
        out = poll_save_keys(ui, save_enabled);
    }
    out
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfirmCancel {
    None,
    Confirm,
    Cancel,
}

/// Visual role for a confirm / choice primary button.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfirmKind {
    /// Neutral primary (Overwrite, Yes, Add, …).
    Primary,
    /// Destructive primary styled with [`crate::theme::MACRO_STOP`] (Delete, …).
    Destructive,
}

fn confirm_button(ui: &mut egui::Ui, label: &str, kind: ConfirmKind) -> egui::Response {
    match kind {
        ConfirmKind::Primary => ui.button(label),
        ConfirmKind::Destructive => {
            ui.button(egui::RichText::new(label).color(crate::theme::MACRO_STOP))
        }
    }
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

/// Cancel left + labeled primary right for destructive / overwrite prompts.
///
/// `confirm_label` is the action verb (**Delete**, **Overwrite**, …) — never
/// generic "Confirm". `Enter` confirms and `Esc` cancels.
pub fn confirm_cancel_row(
    ui: &mut egui::Ui,
    confirm_label: &str,
    kind: ConfirmKind,
) -> ConfirmCancel {
    let mut out = ConfirmCancel::None;
    ui.horizontal(|ui| {
        if ui.button("Cancel").clicked() {
            out = ConfirmCancel::Cancel;
        }
        if confirm_button(ui, confirm_label, kind).clicked() {
            out = ConfirmCancel::Confirm;
        }
    });
    if out == ConfirmCancel::None {
        out = poll_confirm_keys(ui);
    }
    out
}

/// Outcome of [`confirm_choice_row`] (Cancel + multiple labeled choices).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfirmChoice {
    None,
    Cancel,
    /// Index into the `choices` slice passed to [`confirm_choice_row`].
    Choice(usize),
}

/// Cancel left + one or more labeled choice buttons.
///
/// `Esc` → Cancel. `Enter` → `enter_choice` when `Some`.
pub fn confirm_choice_row(
    ui: &mut egui::Ui,
    choices: &[(&str, ConfirmKind)],
    enter_choice: Option<usize>,
) -> ConfirmChoice {
    let mut out = ConfirmChoice::None;
    ui.horizontal(|ui| {
        if ui.button("Cancel").clicked() {
            out = ConfirmChoice::Cancel;
        }
        for (i, (label, kind)) in choices.iter().enumerate() {
            if confirm_button(ui, label, *kind).clicked() {
                out = ConfirmChoice::Choice(i);
            }
        }
    });
    if out == ConfirmChoice::None {
        match poll_confirm_keys(ui) {
            ConfirmCancel::Cancel => out = ConfirmChoice::Cancel,
            ConfirmCancel::Confirm => {
                if let Some(i) = enter_choice.filter(|i| *i < choices.len()) {
                    out = ConfirmChoice::Choice(i);
                }
            }
            ConfirmCancel::None => {}
        }
    }
    out
}

/// Centered, non-resizable popup chrome shared by delete/overwrite confirm prompts.
///
/// Runs `body` inside the window and returns `false` once the user has closed
/// it via the titlebar close control (callers should clear their pending-confirm
/// state in that case, same as an explicit Cancel).
pub fn confirm_window(
    ctx: &egui::Context,
    title: &str,
    pending_scale: Option<&ViewportScaleEvent>,
    body: impl FnOnce(&mut egui::Ui),
) -> bool {
    let mut open = true;
    let id = egui::Id::new(("sqyre_confirm", title));
    fit_dialog_popup(
        egui::Window::new(title)
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .order(egui::Order::Foreground)
            .open(&mut open),
        ctx,
        id,
        pending_scale,
    )
    .show(ctx, body);
    open
}
