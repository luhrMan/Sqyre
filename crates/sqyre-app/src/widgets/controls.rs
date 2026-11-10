//! Shared painted / icon controls used across panels.

use eframe::egui::{
    self, Color32, CornerRadius, Key, Modifiers, Pos2, Sense, Stroke, Vec2, WidgetInfo, WidgetType,
};

use crate::theme::{accent, accent_dim, paint_galley_centered, paint_text_centered, MACRO_STOP};

fn icon_btn_font(ui: &egui::Ui) -> egui::FontId {
    egui::TextStyle::Button.resolve(ui.style())
}

/// Square side for icon-only buttons — sized like a single-line entry box
/// (`TextEdit` = Body row height + `symmetric(4,2)` margins) so buttons and
/// fields sharing a row read at the same height at every Font size.
pub fn icon_btn_side(ui: &egui::Ui) -> f32 {
    ui.text_style_height(&egui::TextStyle::Body) + 4.0
}

/// High-contrast focus ring for custom painted controls (Tab / AccessKit focus).
pub(crate) fn paint_keyboard_focus_ring(
    ui: &egui::Ui,
    rect: egui::Rect,
    response: &egui::Response,
) {
    if !response.has_focus() {
        return;
    }
    ui.painter().rect_stroke(
        rect.expand(1.5),
        CornerRadius::same(4),
        Stroke::new(2.0, accent()),
        egui::StrokeKind::Outside,
    );
}

/// Hover and press fill for icon buttons. Resting and disabled glyphs stay bare.
fn paint_icon_button_chrome(ui: &egui::Ui, rect: egui::Rect, response: &egui::Response) {
    if !response.enabled() {
        return;
    }
    if matches!(
        response.widget_state(),
        egui::widget_style::WidgetState::Inactive | egui::widget_style::WidgetState::Noninteractive
    ) {
        return;
    }
    let visuals = ui.style().interact(response);
    ui.painter().rect(
        rect,
        CornerRadius::same(4),
        visuals.weak_bg_fill,
        visuals.bg_stroke,
        egui::StrokeKind::Inside,
    );
}

/// Icon-only button with an optically centered glyph and no resting frame.
/// Hover and press draw the same interactive fill as the brand button
/// (`frame_when_inactive(false)`).
///
/// `tip` is the AccessKit name and hover text (required for actionable icons).
pub fn icon_button(ui: &mut egui::Ui, glyph: &str, tip: &str) -> egui::Response {
    icon_button_inner(ui, glyph, tip, None)
}

/// Browse/open a picker when there is **no** entity-kind glyph on the field
/// (Settings path Choose, section chrome, …). Catalog ref rows use
/// [`entity_pick_button`] as the leading pair slot instead — never twin a
/// kind glyph with this trailing browse icon.
pub fn pick_icon_button(ui: &mut egui::Ui, tip: &str) -> egui::Response {
    icon_button(ui, crate::model_icons::PICK, tip)
}

/// Leading affordance for a catalog / entity ref field: the kind glyph **is**
/// the pick button (extreme minimalism — no trailing [`pick_icon_button`] twin).
///
/// `tip` is AccessKit name + hover ("Pick point…", …). Put this in
/// [`super::pair_leading`] / [`super::pair_icon`]'s leading slot.
pub fn entity_pick_button(ui: &mut egui::Ui, entity_icon: &str, tip: &str) -> egui::Response {
    icon_button(ui, entity_icon, tip)
}

/// Red `✕` — cancel / dismiss / abort (dialog no, close a floater, disarm a
/// picker). Field and selection clears use [`clear_icon_button`] instead.
/// `tip` names what is cancelled.
pub fn cancel_icon_button(ui: &mut egui::Ui, tip: &str) -> egui::Response {
    icon_button_inner(ui, egui_phosphor::regular::X, tip, Some(MACRO_STOP))
}

/// Red eraser — empties a field, selection, filter, or binding. `tip` names
/// what is cleared. Cancel / dismiss stays [`cancel_icon_button`].
pub fn clear_icon_button(ui: &mut egui::Ui, tip: &str) -> egui::Response {
    icon_button_inner(ui, egui_phosphor::regular::ERASER, tip, Some(MACRO_STOP))
}

/// Green `✓` — the yes side of confirm rows (`no` is [`cancel_icon_button`]).
/// `tip` keeps the action verb ("Delete", "Overwrite", …) as the AccessKit
/// name and hover text.
pub fn confirm_icon_button(ui: &mut egui::Ui, tip: &str) -> egui::Response {
    icon_button_inner(
        ui,
        egui_phosphor::regular::CHECK,
        tip,
        Some(crate::theme::MACRO_START),
    )
}

fn icon_button_inner(
    ui: &mut egui::Ui,
    glyph: &str,
    tip: &str,
    color: Option<Color32>,
) -> egui::Response {
    let enabled = ui.is_enabled();
    let font_id = icon_btn_font(ui);
    let desired = Vec2::splat(icon_btn_side(ui));
    let (rect, response) = ui.allocate_exact_size(desired, Sense::click());
    paint_icon_button_chrome(ui, rect, &response);
    let visuals = ui.style().interact(&response);
    let fg = color.unwrap_or(visuals.text_color());
    paint_text_centered(ui, rect, glyph, font_id, fg);
    paint_keyboard_focus_ring(ui, rect, &response);
    response.widget_info(|| WidgetInfo::labeled(WidgetType::Button, enabled, tip));
    response.on_hover_text(tip).on_disabled_hover_text(tip)
}

/// [`icon_button`] / [`cancel_icon_button`] / [`clear_icon_button`] as a
/// [`egui::Widget`] — the form [`egui::Ui::add_enabled`] requires.
/// (`add_enabled_ui` wraps content in a scope child that owns the rest of a
/// wrapping row, so a single disabled icon could never move to the next row
/// and painted past the clip.)
pub struct IconButton<'a> {
    pub glyph: &'a str,
    pub tip: &'a str,
    pub color: Option<Color32>,
}

impl<'a> IconButton<'a> {
    pub fn new(glyph: &'a str, tip: &'a str, color: Option<Color32>) -> Self {
        Self { glyph, tip, color }
    }
}

impl egui::Widget for IconButton<'_> {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        icon_button_inner(ui, self.glyph, self.tip, self.color)
    }
}

/// Small glyph badge inside the top-right corner of `rect` (e.g. a button's
/// response rect). Sized to a fraction of the host so it reads as a badge,
/// never covers the control; kept inside the bounds so the glyph is never cut
/// by the row clip.
///
/// Color alone is not enough: callers also change the hover text to say what is wrong.
fn corner_badge(ui: &mut egui::Ui, rect: egui::Rect, glyph: &str, color: Color32) {
    let font_h = ui.text_style_height(&egui::TextStyle::Small);
    let side = (rect.height() * 0.45).min(font_h).max(6.0);
    let font_id = egui::FontId::proportional(side);
    let badge = egui::Rect::from_center_size(
        egui::pos2(rect.right() - side * 0.7, rect.top() + side * 0.5),
        Vec2::splat(side),
    );
    ui.painter()
        .circle_filled(badge.center(), side * 0.55, ui.visuals().panel_fill);
    paint_text_centered(ui, badge, glyph, font_id, color);
}

/// `⚠` corner badge for warnings on a host widget.
pub fn warn_badge(ui: &mut egui::Ui, rect: egui::Rect) {
    corner_badge(ui, rect, "⚠", crate::theme::warn_fg());
}

/// `⊗` corner badge for errors on a host widget.
pub fn error_badge(ui: &mut egui::Ui, rect: egui::Rect) {
    corner_badge(
        ui,
        rect,
        egui_phosphor::regular::X_CIRCLE,
        crate::theme::error_fg(),
    );
}

/// Severity glyph with the message it replaces in the tooltip / AccessKit
/// name — the icon-first replacement for an always-visible status line. `tip`
/// is required: an unexplained icon is dead chrome.
///
/// Hover shows the message. A press or tap does too, and the tap keeps it
/// open: a finger is not a hover, and egui will not open a hover tooltip for
/// a [`Sense::hover`] widget while a button is down, then hides it on release.
///
/// Uses Phosphor glyphs (Proportional fallback) — emoji ✕/⚠ often render as
/// missing-glyph tofu with the Ubuntu/Hack chain.
pub fn status_icon(ui: &mut egui::Ui, glyph: &str, color: Color32, tip: &str) -> egui::Response {
    let enabled = ui.is_enabled();
    let resp = ui.add(
        egui::Label::new(egui::RichText::new(glyph).color(color))
            .sense(Sense::CLICK)
            .selectable(false),
    );
    resp.widget_info(|| WidgetInfo::labeled(WidgetType::Label, enabled, tip));
    reveal_status_tip(ui, &resp, tip);
    resp
}

/// Show `tip` for `resp` on hover, while it is pressed, and after a tap until
/// another click. The press path does not wait out `tooltip_delay` and does
/// not use the hover-only early-out that drops a finger press.
pub(crate) fn reveal_status_tip(ui: &egui::Ui, resp: &egui::Response, tip: &str) {
    let id = resp.id.with("status_tip_open");
    if resp.clicked() {
        let open = ui.ctx().data(|d| d.get_temp::<bool>(id).unwrap_or(false));
        ui.ctx().data_mut(|d| d.insert_temp(id, !open));
    } else if resp.clicked_elsewhere() {
        ui.ctx().data_mut(|d| d.insert_temp(id, false));
    }
    let pinned = ui.ctx().data(|d| d.get_temp::<bool>(id).unwrap_or(false));
    if pinned || resp.is_pointer_button_down_on() {
        // The first frame only sizes the tip. Ask for the frame that paints it,
        // then stop so a tip left open does not repaint forever.
        if !resp.is_tooltip_open() {
            resp.ctx.request_repaint();
        }
        show_status_tip(resp, tip);
    } else {
        let _ = resp.clone().on_hover_text(tip);
    }
}

fn show_status_tip(resp: &egui::Response, tip: &str) {
    let screen_w = resp.ctx.content_rect().width();
    let max_w = (screen_w.min(resp.ctx.global_style().spacing.tooltip_width) - 24.0).max(80.0);
    egui::Tooltip::for_widget(resp).show(|ui| {
        ui.set_max_width(max_w);
        ui.label(tip);
    });
}

/// `⊗` error marker — hover, press, or tap shows the error text.
pub fn error_icon(ui: &mut egui::Ui, tip: &str) -> egui::Response {
    status_icon(
        ui,
        egui_phosphor::regular::X_CIRCLE,
        crate::theme::error_fg(),
        tip,
    )
}

/// `⚠` warning marker — hover, press, or tap shows the warning text.
pub fn warn_icon(ui: &mut egui::Ui, tip: &str) -> egui::Response {
    status_icon(
        ui,
        egui_phosphor::regular::WARNING,
        crate::theme::warn_fg(),
        tip,
    )
}

/// Named font family for Phosphor Fill (outline Regular stays on Proportional).
pub const PHOSPHOR_FILL_FAMILY: &str = "phosphor-fill";

/// Icon-only on/off toggle: outline glyph when off, filled glyph when on.
///
/// `tip` is the AccessKit name and hover text. Returns a response whose
/// `changed()` is true when the user toggled `selected`.
pub fn icon_toggle(
    ui: &mut egui::Ui,
    selected: &mut bool,
    tip: &str,
    outline_glyph: &str,
    filled_glyph: &str,
) -> egui::Response {
    let enabled = ui.is_enabled();
    let desired = Vec2::splat(icon_btn_side(ui));
    let (rect, mut response) = ui.allocate_exact_size(desired, Sense::click());
    if enabled && response.clicked() {
        *selected = !*selected;
        response.mark_changed();
    }
    paint_icon_button_chrome(ui, rect, &response);
    let visuals = ui.style().interact(&response);
    let glyph = if *selected {
        filled_glyph
    } else {
        outline_glyph
    };
    let font_id = if *selected {
        egui::FontId::new(
            icon_btn_font(ui).size,
            egui::FontFamily::Name(PHOSPHOR_FILL_FAMILY.into()),
        )
    } else {
        icon_btn_font(ui)
    };
    let fg = if *selected {
        accent()
    } else if enabled {
        visuals.text_color()
    } else {
        visuals.text_color().gamma_multiply(0.55)
    };
    paint_text_centered(ui, rect, glyph, font_id, fg);
    paint_keyboard_focus_ring(ui, rect, &response);
    let selected_now = *selected;
    response.widget_info(|| WidgetInfo::selected(WidgetType::Checkbox, enabled, selected_now, tip));
    response.on_hover_text(tip).on_disabled_hover_text(tip)
}

/// [`icon_toggle`] as a [`egui::Widget`] — see [`IconButton`].
pub struct IconToggle<'a> {
    pub selected: &'a mut bool,
    pub tip: &'a str,
    pub outline_glyph: &'a str,
    pub filled_glyph: &'a str,
}

impl<'a> IconToggle<'a> {
    pub fn new(
        selected: &'a mut bool,
        tip: &'a str,
        outline_glyph: &'a str,
        filled_glyph: &'a str,
    ) -> Self {
        Self {
            selected,
            tip,
            outline_glyph,
            filled_glyph,
        }
    }
}

impl egui::Widget for IconToggle<'_> {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        icon_toggle(
            ui,
            self.selected,
            self.tip,
            self.outline_glyph,
            self.filled_glyph,
        )
    }
}

/// Repaint interval for pulsing glows (~20 fps; the pulse is slow enough to stay smooth).
const GLOW_FRAME: std::time::Duration = std::time::Duration::from_millis(50);

/// Soft 0..1 glow phase (~0.4 Hz) shared by pulsing controls.
///
/// Schedules the next frame only while the viewport is focused and not minimized;
/// otherwise the glow freezes until something else repaints.
pub fn glow_pulse(ui: &egui::Ui) -> f32 {
    let (t, visible) = ui.input(|i| {
        let vp = i.viewport();
        (
            i.time as f32,
            vp.focused != Some(false) && vp.minimized != Some(true),
        )
    });
    if visible {
        ui.ctx().request_repaint_after(GLOW_FRAME);
    }
    (t * std::f32::consts::TAU * 0.4).sin().mul_add(0.5, 0.5)
}

/// [`glow_pulse`] while `animate`, else a steady full glow that schedules no repaints.
pub fn glow_pulse_if(ui: &egui::Ui, animate: bool) -> f32 {
    if animate {
        glow_pulse(ui)
    } else {
        1.0
    }
}

/// Stacked filled halos around `rect`; paint *under* an opaque body.
///
/// Filled (not concentric `rect_stroke`s) so epaint stroke tessellation cannot
/// leave a midline seam.
pub fn glow_halo_shapes(
    rect: egui::Rect,
    rounding: CornerRadius,
    color: Color32,
    pulse: f32,
) -> Vec<egui::Shape> {
    (1..=5u8)
        .rev()
        .map(|i| {
            let i = f32::from(i);
            let expand = i * (1.1 + pulse * 1.6);
            let alpha = ((22.0 / i) * (0.35 + 0.65 * pulse)).round() as u8;
            let expand_u8 = expand.round().clamp(0.0, 255.0) as u8;
            let glow_round = CornerRadius {
                nw: rounding.nw.saturating_add(expand_u8),
                ne: rounding.ne.saturating_add(expand_u8),
                sw: rounding.sw.saturating_add(expand_u8),
                se: rounding.se.saturating_add(expand_u8),
            };
            egui::Shape::rect_filled(
                rect.expand(expand),
                glow_round,
                Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), alpha),
            )
        })
        .collect()
}

/// Persist / commit button (floppy-disk glyph) with a Sqyre-yellow glow while
/// enabled (dirty + valid). `tip` is the AccessKit name and hover text.
///
/// Disabled state matches a normal `add_enabled(false, …)` icon button. The glow
/// holds steady and only pulses (see [`glow_pulse`]) while hovered.
pub fn dirty_action_button(ui: &mut egui::Ui, tip: &str, enabled: bool) -> egui::Response {
    let glyph = egui_phosphor::regular::FLOPPY_DISK;
    if !enabled {
        return ui.add_enabled(false, IconButton::new(glyph, tip, None));
    }

    let visuals = ui.style().visuals.widgets.inactive;
    let rounding = visuals.corner_radius;
    let base_fill = visuals.weak_bg_fill;
    let pad = ui.spacing().button_padding;
    let font_id = icon_btn_font(ui);
    let galley = ui
        .painter()
        .layout_no_wrap(glyph.to_owned(), font_id, accent());
    let size = Vec2::new(
        galley.size().x + 2.0 * pad.x,
        (galley.size().y + 2.0 * pad.y).max(ui.spacing().interact_size.y),
    );

    let (rect, response) = ui.allocate_exact_size(size, Sense::click());
    let pulse = glow_pulse_if(ui, response.hovered());
    let painter = ui.painter();

    painter.extend(glow_halo_shapes(rect, rounding, accent(), pulse));

    // Opaque body covers the halo center so nothing shows through the label.
    let tint = 0.12 + pulse * 0.22;
    let fill = Color32::from_rgb(
        ((1.0 - tint) * base_fill.r() as f32 + tint * accent().r() as f32).round() as u8,
        ((1.0 - tint) * base_fill.g() as f32 + tint * accent().g() as f32).round() as u8,
        ((1.0 - tint) * base_fill.b() as f32 + tint * accent().b() as f32).round() as u8,
    );
    // Keep stroke width fixed — animating Inside stroke width can tessellate a midline.
    let stroke = Stroke::new(1.0, accent());
    painter.rect(rect, rounding, fill, stroke, egui::StrokeKind::Inside);
    paint_galley_centered(ui, rect, galley, accent());
    paint_keyboard_focus_ring(ui, rect, &response);
    response.widget_info(|| WidgetInfo::labeled(WidgetType::Button, true, tip));
    response.on_hover_text(tip)
}

/// Icon-only record control (danger styling).
pub fn record_icon_button(ui: &mut egui::Ui, tip: &str, enabled: bool) -> egui::Response {
    ui.add_enabled(enabled, IconButton::new("●", tip, Some(MACRO_STOP)))
}

/// Icon-only add control (`+` in start styling).
pub fn add_button(ui: &mut egui::Ui, tip: &str, enabled: bool) -> egui::Response {
    ui.add_enabled(
        enabled,
        IconButton::new(
            egui_phosphor::regular::PLUS,
            tip,
            Some(crate::theme::MACRO_START),
        ),
    )
}

/// Icon-only remove/delete control (`✕` in danger styling).
pub fn remove_icon_button(ui: &mut egui::Ui, tip: &str, enabled: bool) -> egui::Response {
    ui.add_enabled(
        enabled,
        IconButton::new(egui_phosphor::regular::X, tip, Some(MACRO_STOP)),
    )
}

/// Visual / keyboard order for [`press_state_toggle`] (top → bottom).
const PRESS_STATE_ORDER: &[sqyre_domain::PressState] = &[
    sqyre_domain::PressState::Up,
    sqyre_domain::PressState::Tap,
    sqyre_domain::PressState::Down,
];

fn press_state_label(state: sqyre_domain::PressState) -> &'static str {
    match state {
        sqyre_domain::PressState::Up => "Up",
        sqyre_domain::PressState::Tap => "Tap",
        sqyre_domain::PressState::Down => "Down",
    }
}

/// Wrap-step index into `options` by `delta` (−1 / +1).
fn cycle_index(len: usize, idx: usize, delta: i32) -> usize {
    debug_assert!(len > 0);
    ((idx as i32 + delta).rem_euclid(len as i32)) as usize
}

/// Next value from `options` when focused and an arrow key is pressed.
///
/// Right/Down advance; Left/Up retreat. Returns `true` when the value changed.
fn cycle_focused_option<T: Copy + PartialEq>(
    ui: &mut egui::Ui,
    focused: bool,
    value: &mut T,
    options: &[T],
) -> bool {
    if !focused || options.is_empty() {
        return false;
    }
    let delta = ui.input_mut(|i| {
        if i.consume_key(Modifiers::NONE, Key::ArrowRight)
            || i.consume_key(Modifiers::NONE, Key::ArrowDown)
        {
            Some(1)
        } else if i.consume_key(Modifiers::NONE, Key::ArrowLeft)
            || i.consume_key(Modifiers::NONE, Key::ArrowUp)
        {
            Some(-1)
        } else {
            None
        }
    });
    let Some(delta) = delta else {
        return false;
    };
    let idx = options.iter().position(|o| *o == *value).unwrap_or(0);
    let next = options[cycle_index(options.len(), idx, delta)];
    if next == *value {
        return false;
    }
    *value = next;
    true
}

/// AccessKit name + current value (ComboBox role, matching egui's closed combo).
fn valued_control_info(enabled: bool, name: &str, value: &str) -> WidgetInfo {
    WidgetInfo {
        enabled,
        label: Some(name.to_owned()),
        current_text_value: Some(value.to_owned()),
        ..WidgetInfo::new(WidgetType::ComboBox)
    }
}

/// Top-down mouse for Click button selection.
///
/// Left / right buttons, center wheel (middle), lower body (scroll).
/// Arrow keys cycle when focused; AccessKit announces name + current button.
pub fn mouse_button_picker(
    ui: &mut egui::Ui,
    button: &mut sqyre_domain::MouseButton,
) -> egui::Response {
    use sqyre_domain::MouseButton;

    const W: f32 = 36.0;
    const H: f32 = 54.0;

    let desired = Vec2::new(W, H);
    let (rect, mut response) = ui.allocate_exact_size(desired, Sense::click());

    let body = rect.shrink2(Vec2::new(1.0, 1.0));
    let button_h = body.height() * 0.42;
    let buttons = egui::Rect::from_min_size(body.min, Vec2::new(body.width(), button_h));
    let mid_x = buttons.center().x;
    let left_btn = egui::Rect::from_min_max(buttons.min, Pos2::new(mid_x, buttons.bottom()));
    let right_btn = egui::Rect::from_min_max(
        Pos2::new(mid_x, buttons.top()),
        Pos2::new(buttons.right(), buttons.bottom()),
    );
    let wheel = egui::Rect::from_center_size(
        Pos2::new(mid_x, buttons.top() + button_h * 0.55),
        Vec2::new(body.width() * 0.22, button_h * 0.55),
    );
    let scroll_body = egui::Rect::from_min_max(Pos2::new(body.left(), buttons.bottom()), body.max);

    let hit = |pos: Pos2| -> MouseButton {
        if wheel.contains(pos) {
            MouseButton::Middle
        } else if left_btn.contains(pos) {
            MouseButton::Left
        } else if right_btn.contains(pos) {
            MouseButton::Right
        } else {
            MouseButton::Scroll
        }
    };

    if response.clicked() {
        if let Some(pos) = response.interact_pointer_pos() {
            *button = hit(pos);
            response.mark_changed();
        }
    }
    if cycle_focused_option(ui, response.has_focus(), button, MouseButton::ALL) {
        response.mark_changed();
    }

    let hover_btn = response.hover_pos().map(hit).filter(|_| response.hovered());

    let visuals = ui.style().interact(&response);
    let painter = ui.painter();
    let rounding = CornerRadius::same(body.width() as u8 / 2);
    let stroke = Stroke::new(1.0, visuals.bg_stroke.color);

    painter.rect_filled(body, rounding, visuals.bg_fill);
    painter.rect_stroke(body, rounding, stroke, egui::StrokeKind::Inside);

    let highlight = |b: MouseButton| -> Option<Color32> {
        if *button == b {
            Some(accent())
        } else if hover_btn == Some(b) {
            Some(accent_dim())
        } else {
            None
        }
    };

    // Clip button fills to the upper body shape via a slightly inset paint.
    if let Some(c) = highlight(MouseButton::Left) {
        painter.rect_filled(
            left_btn.intersect(body).shrink(1.0),
            CornerRadius {
                nw: rounding.nw,
                ne: 0,
                sw: 0,
                se: 0,
            },
            c,
        );
    }
    if let Some(c) = highlight(MouseButton::Right) {
        painter.rect_filled(
            right_btn.intersect(body).shrink(1.0),
            CornerRadius {
                nw: 0,
                ne: rounding.ne,
                sw: 0,
                se: 0,
            },
            c,
        );
    }
    if let Some(c) = highlight(MouseButton::Scroll) {
        painter.rect_filled(
            scroll_body.intersect(body).shrink(1.0),
            CornerRadius {
                nw: 0,
                ne: 0,
                sw: rounding.sw,
                se: rounding.se,
            },
            c,
        );
    }

    // Seam between left/right and buttons/body.
    painter.line_segment(
        [
            Pos2::new(mid_x, buttons.top() + 2.0),
            Pos2::new(mid_x, buttons.bottom()),
        ],
        stroke,
    );
    painter.line_segment(
        [
            Pos2::new(body.left() + 2.0, buttons.bottom()),
            Pos2::new(body.right() - 2.0, buttons.bottom()),
        ],
        stroke,
    );

    let wheel_fill = highlight(MouseButton::Middle).unwrap_or(visuals.weak_bg_fill);
    let wheel_r = CornerRadius::same((wheel.width() / 2.0) as u8);
    painter.rect_filled(wheel, wheel_r, wheel_fill);
    painter.rect_stroke(wheel, wheel_r, stroke, egui::StrokeKind::Inside);

    // Scroll affordance on the lower body (stronger when selected / hovered).
    {
        let cx = scroll_body.center().x;
        let cy = scroll_body.center().y;
        let active =
            matches!(*button, MouseButton::Scroll) || hover_btn == Some(MouseButton::Scroll);
        let chevron = if active {
            visuals.fg_stroke.color
        } else {
            visuals.bg_stroke.color
        };
        let s = 3.5;
        painter.line_segment(
            [Pos2::new(cx - s, cy - 2.0), Pos2::new(cx, cy - 6.0)],
            Stroke::new(1.5, chevron),
        );
        painter.line_segment(
            [Pos2::new(cx + s, cy - 2.0), Pos2::new(cx, cy - 6.0)],
            Stroke::new(1.5, chevron),
        );
        painter.line_segment(
            [Pos2::new(cx - s, cy + 2.0), Pos2::new(cx, cy + 6.0)],
            Stroke::new(1.5, chevron),
        );
        painter.line_segment(
            [Pos2::new(cx + s, cy + 2.0), Pos2::new(cx, cy + 6.0)],
            Stroke::new(1.5, chevron),
        );
    }

    paint_keyboard_focus_ring(ui, rect, &response);
    let tip = hover_btn.unwrap_or(*button).label();
    let enabled = ui.is_enabled();
    let value = button.label();
    response.widget_info(|| valued_control_info(enabled, "Mouse button", value));
    response.on_hover_text(tip)
}

/// Vertical up / tap / down switch for Click/Key press state.
///
/// Top = up, middle = tap, bottom = down. Click a zone to set; arrows cycle when focused.
pub fn press_state_toggle(
    ui: &mut egui::Ui,
    state: &mut sqyre_domain::PressState,
) -> egui::Response {
    use sqyre_domain::PressState;

    const TRACK_W: f32 = 18.0;
    const TRACK_H: f32 = 54.0;
    const PAD: f32 = 2.0;

    let desired = Vec2::new(TRACK_W, TRACK_H);
    let (rect, mut response) = ui.allocate_exact_size(desired, Sense::click());

    if response.clicked() {
        if let Some(pos) = response.interact_pointer_pos() {
            let third = rect.height() / 3.0;
            let y = pos.y - rect.top();
            *state = if y < third {
                PressState::Up
            } else if y < third * 2.0 {
                PressState::Tap
            } else {
                PressState::Down
            };
        } else {
            *state = match *state {
                PressState::Up => PressState::Tap,
                PressState::Tap => PressState::Down,
                PressState::Down => PressState::Up,
            };
        }
        response.mark_changed();
    }
    if cycle_focused_option(ui, response.has_focus(), state, PRESS_STATE_ORDER) {
        response.mark_changed();
    }

    let visuals = ui.style().interact(&response);
    let track_fill = if matches!(*state, PressState::Down) {
        accent()
    } else {
        visuals.bg_fill
    };
    let painter = ui.painter();
    let rounding = CornerRadius::same((TRACK_W / 2.0) as u8);
    painter.rect_filled(rect, rounding, track_fill);
    painter.rect_stroke(
        rect,
        rounding,
        Stroke::new(1.0, visuals.bg_stroke.color),
        egui::StrokeKind::Inside,
    );

    let knob_d = TRACK_W - PAD * 2.0;
    let knob_x = rect.center().x;
    let knob_y = match *state {
        PressState::Up => rect.top() + PAD + knob_d / 2.0,
        PressState::Tap => rect.center().y,
        PressState::Down => rect.bottom() - PAD - knob_d / 2.0,
    };
    painter.circle_filled(
        Pos2::new(knob_x, knob_y),
        knob_d / 2.0,
        visuals.fg_stroke.color,
    );

    paint_keyboard_focus_ring(ui, rect, &response);
    let tip = press_state_label(*state);
    let enabled = ui.is_enabled();
    response.widget_info(|| valued_control_info(enabled, "Press state", tip));
    response.on_hover_text(tip)
}

/// Vertical up↔down switch for Click/Key button state (`true` = down).
///
/// Top of the track is up; bottom is down. Click a half to set; arrows cycle when focused.
#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "shared painted control; unit-tested, reserved for Key/Click chrome"
    )
)]
pub(crate) fn up_down_toggle(ui: &mut egui::Ui, down: &mut bool) -> egui::Response {
    const TRACK_W: f32 = 18.0;
    const TRACK_H: f32 = 36.0;
    const PAD: f32 = 2.0;
    const ORDER: &[bool] = &[false, true];

    let desired = Vec2::new(TRACK_W, TRACK_H);
    let (rect, mut response) = ui.allocate_exact_size(desired, Sense::click());

    if response.clicked() {
        if let Some(pos) = response.interact_pointer_pos() {
            *down = pos.y > rect.center().y;
        } else {
            *down = !*down;
        }
        response.mark_changed();
    }
    if cycle_focused_option(ui, response.has_focus(), down, ORDER) {
        response.mark_changed();
    }

    let visuals = ui.style().interact(&response);
    let track_fill = if *down { accent() } else { visuals.bg_fill };
    let painter = ui.painter();
    let rounding = CornerRadius::same((TRACK_W / 2.0) as u8);
    painter.rect_filled(rect, rounding, track_fill);
    painter.rect_stroke(
        rect,
        rounding,
        Stroke::new(1.0, visuals.bg_stroke.color),
        egui::StrokeKind::Inside,
    );

    let knob_d = TRACK_W - PAD * 2.0;
    let knob_x = rect.center().x;
    let knob_y = if *down {
        rect.bottom() - PAD - knob_d / 2.0
    } else {
        rect.top() + PAD + knob_d / 2.0
    };
    painter.circle_filled(
        Pos2::new(knob_x, knob_y),
        knob_d / 2.0,
        visuals.fg_stroke.color,
    );

    paint_keyboard_focus_ring(ui, rect, &response);
    let tip = if *down { "Down" } else { "Up" };
    let enabled = ui.is_enabled();
    response.widget_info(|| valued_control_info(enabled, "Up/Down", tip));
    response.on_hover_text(tip)
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui::{Event, RawInput};
    use sqyre_domain::{MouseButton, PressState};

    #[test]
    fn cycle_index_wraps() {
        assert_eq!(cycle_index(4, 0, 1), 1);
        assert_eq!(cycle_index(4, 3, 1), 0);
        assert_eq!(cycle_index(4, 0, -1), 3);
        assert_eq!(cycle_index(3, 1, -1), 0);
    }

    /// A disabled `IconButton` added via `Ui::add_enabled` must still wrap like a
    /// normal widget inside `horizontal_wrapped` — a scope child (e.g.
    /// `add_enabled_ui`) owns the rest of the row and paints past the clip.
    #[test]
    fn disabled_icon_button_wraps_in_wrapped_row() {
        let ctx = egui::Context::default();
        run_ui(&ctx, RawInput::default(), |ui| {
            let pane = egui::Rect::from_min_size(ui.max_rect().min, egui::vec2(50.0, 300.0));
            let mut pane_ui = ui.new_child(egui::UiBuilder::new().max_rect(pane));
            pane_ui.horizontal_wrapped(|ui| {
                for _ in 0..4 {
                    ui.add_enabled(false, IconButton::new("↻", "icon", None));
                }
            });
            assert!(
                pane_ui.min_rect().width() <= 50.0 + 0.5,
                "disabled icons overflowed a wrapped row: {:?}",
                pane_ui.min_rect()
            );
        });
    }

    /// Icon buttons are bare glyphs at rest. Hover paints the interactive fill.
    #[test]
    fn icon_button_chrome_only_on_hover() {
        let ctx = egui::Context::default();
        let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(400.0, 300.0));

        let mut idle_rect = egui::Rect::NOTHING;
        let mut toggle_rect = egui::Rect::NOTHING;
        let idle = ctx.run_ui(
            RawInput {
                screen_rect: Some(screen),
                ..Default::default()
            },
            |ui| {
                idle_rect = icon_button(ui, "↻", "Refresh").rect;
                let mut on = false;
                toggle_rect = icon_toggle(ui, &mut on, "Pin", "☆", "★").rect;
            },
        );
        assert!(
            !button_fill_on(&idle.shapes, idle_rect),
            "resting icon button painted a fill"
        );
        assert!(
            !button_fill_on(&idle.shapes, toggle_rect),
            "resting icon toggle painted a fill"
        );
        idle.drop_without_applying_deltas();

        let mut hover_rect = egui::Rect::NOTHING;
        let hover = ctx.run_ui(
            RawInput {
                screen_rect: Some(screen),
                events: vec![egui::Event::PointerMoved(idle_rect.center())],
                ..Default::default()
            },
            |ui| {
                hover_rect = icon_button(ui, "↻", "Refresh").rect;
            },
        );
        assert!(
            button_fill_on(&hover.shapes, hover_rect),
            "hovered icon button painted no fill"
        );
        hover.drop_without_applying_deltas();
    }

    /// A finger is not a hover, and the hover delay must not hide the message.
    #[test]
    fn error_icon_shows_its_message_on_tap() {
        let ctx = egui::Context::default();
        ctx.all_styles_mut(|style| {
            style.interaction.tooltip_delay = 30.0;
        });
        let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(360.0, 640.0));
        let tip = "Image unavailable";
        let mut icon = egui::Pos2::ZERO;
        run_frame(&ctx, screen, 0.0, vec![], |ui| {
            icon = error_icon(ui, tip).rect.center();
        })
        .drop_without_applying_deltas();

        run_frame(
            &ctx,
            screen,
            0.2,
            vec![egui::Event::PointerMoved(icon), pointer_button(icon, true)],
            |ui| {
                let _ = error_icon(ui, tip);
            },
        )
        .drop_without_applying_deltas();
        // Opening frame is a sizing pass; the held frame is what the user sees.
        let pressed = run_frame(&ctx, screen, 0.4, vec![], |ui| {
            let _ = error_icon(ui, tip);
        });
        let press_ok = text_on_screen(&pressed.shapes, screen, tip);
        pressed.drop_without_applying_deltas();
        assert!(press_ok, "holding the error icon did not show its message");

        let stayed = run_frame(
            &ctx,
            screen,
            0.6,
            vec![pointer_button(icon, false), egui::Event::PointerGone],
            |ui| {
                let _ = error_icon(ui, tip);
            },
        );
        let stay_ok = text_on_screen(&stayed.shapes, screen, tip);
        stayed.drop_without_applying_deltas();
        assert!(
            stay_ok,
            "the error icon message disappeared when the tap ended"
        );
    }

    fn pointer_button(pos: egui::Pos2, pressed: bool) -> Event {
        Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: Modifiers::NONE,
        }
    }

    fn run_frame(
        ctx: &egui::Context,
        screen: egui::Rect,
        time: f64,
        events: Vec<Event>,
        add: impl FnMut(&mut egui::Ui),
    ) -> egui::FullOutput {
        ctx.run_ui(
            RawInput {
                screen_rect: Some(screen),
                time: Some(time),
                predicted_dt: 0.1,
                events,
                ..Default::default()
            },
            add,
        )
    }

    fn text_on_screen(
        shapes: &[egui::epaint::ClippedShape],
        screen: egui::Rect,
        needle: &str,
    ) -> bool {
        fn walk(shape: &egui::Shape, clip: egui::Rect, screen: egui::Rect, needle: &str) -> bool {
            match shape {
                egui::Shape::Text(text) if text.galley.text().contains(needle) => {
                    let painted =
                        egui::Rect::from_min_size(text.pos, text.galley.size()).intersect(clip);
                    let visible = painted.intersect(screen);
                    visible.width() > 8.0 && visible.height() > 8.0
                }
                egui::Shape::Vec(shapes) => shapes.iter().any(|s| walk(s, clip, screen, needle)),
                _ => false,
            }
        }
        shapes
            .iter()
            .any(|clipped| walk(&clipped.shape, clipped.clip_rect, screen, needle))
    }

    fn button_fill_on(shapes: &[egui::epaint::ClippedShape], button: egui::Rect) -> bool {
        shapes.iter().any(|clipped| match &clipped.shape {
            egui::Shape::Rect(rect) => {
                (rect.rect.center() - button.center()).length() < 2.0
                    && (rect.rect.width() - button.width()).abs() < 4.0
                    && rect.fill.a() > 0
            }
            _ => false,
        })
    }

    /// Icon buttons share rows with entry boxes — [`icon_btn_side`] must track
    /// the `TextEdit` height (Body row + 2×2 margin) at every Font size or the
    /// row reads lopsided.
    #[test]
    fn icon_button_matches_text_edit_height() {
        let ctx = egui::Context::default();
        for font_px in [10.0f32, 12.0, 14.0, 16.0] {
            let mut style = (*ctx.global_style()).clone();
            for ts in [egui::TextStyle::Body, egui::TextStyle::Button] {
                style
                    .text_styles
                    .insert(ts, egui::FontId::proportional(font_px));
            }
            ctx.set_global_style(style);
            let mut s = String::new();
            run_ui(&ctx, RawInput::default(), |ui| {
                let btn = icon_button(ui, "↻", "Refresh");
                let edit = ui.add(egui::TextEdit::singleline(&mut s));
                assert!(
                    (btn.rect.height() - edit.rect.height()).abs() < 0.5,
                    "font={font_px}: icon button {} vs entry box {}",
                    btn.rect.height(),
                    edit.rect.height()
                );
            });
        }
    }

    #[test]
    fn valued_control_info_exposes_name_and_value() {
        let info = valued_control_info(true, "Mouse button", "Left");
        assert_eq!(info.typ, WidgetType::ComboBox);
        assert_eq!(info.label.as_deref(), Some("Mouse button"));
        assert_eq!(info.current_text_value.as_deref(), Some("Left"));
        assert!(info.enabled);
    }

    fn key_press(key: Key) -> Event {
        Event::Key {
            key,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: Modifiers::NONE,
        }
    }

    /// Headless tests discard paint output; clear texture deltas so Drop does not panic (egui 0.36+).
    fn run_ui(ctx: &egui::Context, input: RawInput, add_contents: impl FnMut(&mut egui::Ui)) {
        ctx.run_ui(input, add_contents)
            .drop_without_applying_deltas();
    }

    #[test]
    fn mouse_button_picker_cycles_with_arrows_when_focused() {
        let ctx = egui::Context::default();
        let mut button = MouseButton::Left;

        run_ui(&ctx, RawInput::default(), |ui| {
            let r = mouse_button_picker(ui, &mut button);
            r.request_focus();
        });

        let mut input = RawInput::default();
        input.events.push(key_press(Key::ArrowRight));
        run_ui(&ctx, input, |ui| {
            let r = mouse_button_picker(ui, &mut button);
            assert!(r.has_focus(), "picker must keep focus for arrow cycling");
            assert!(r.changed());
        });
        assert_eq!(button, MouseButton::Right);

        let mut input = RawInput::default();
        input.events.push(key_press(Key::ArrowLeft));
        run_ui(&ctx, input, |ui| {
            let r = mouse_button_picker(ui, &mut button);
            assert!(r.changed());
        });
        assert_eq!(button, MouseButton::Left);
    }

    #[test]
    fn press_state_toggle_cycles_with_arrows_when_focused() {
        let ctx = egui::Context::default();
        let mut state = PressState::Up;

        run_ui(&ctx, RawInput::default(), |ui| {
            press_state_toggle(ui, &mut state).request_focus();
        });

        let mut input = RawInput::default();
        input.events.push(key_press(Key::ArrowDown));
        run_ui(&ctx, input, |ui| {
            let r = press_state_toggle(ui, &mut state);
            assert!(r.has_focus());
            assert!(r.changed());
        });
        assert_eq!(state, PressState::Tap);

        let mut input = RawInput::default();
        input.events.push(key_press(Key::ArrowDown));
        run_ui(&ctx, input, |ui| {
            assert!(press_state_toggle(ui, &mut state).changed());
        });
        assert_eq!(state, PressState::Down);
    }

    #[test]
    fn up_down_toggle_cycles_with_arrows_when_focused() {
        let ctx = egui::Context::default();
        let mut down = false;

        run_ui(&ctx, RawInput::default(), |ui| {
            up_down_toggle(ui, &mut down).request_focus();
        });

        let mut input = RawInput::default();
        input.events.push(key_press(Key::ArrowRight));
        run_ui(&ctx, input, |ui| {
            let r = up_down_toggle(ui, &mut down);
            assert!(r.has_focus());
            assert!(r.changed());
        });
        assert!(down);
    }
}
