//! Sqyre brand theme (dark + Sqyre yellow accents).
//!
//! Brand/semantic colors live in [`sqyre_ui_theme`]; this module re-exports them
//! and hosts app-only visuals + widgets.

use eframe::egui::{
    self, Color32, CornerRadius, Key, Modifiers, Pos2, Sense, Stroke, Vec2, Visuals, WidgetInfo,
    WidgetType,
};

pub use sqyre_ui_theme::{
    accent_dim, chip_fill, contrast_fg, error_fg, frame_fill, inner_stroke, ok_fg,
    overlay_panel_fill, paint_galley_centered, paint_text_centered, rgba, warn_fg, MACRO_START,
    MACRO_STOP, PRIMARY,
};

/// Minimum square hit target for icon-only buttons (framed and bare).
/// Side grows with Button text so glyphs track the Font size setting.
pub const ICON_BTN_SIDE: f32 = 18.0;

fn icon_btn_font(ui: &egui::Ui) -> egui::FontId {
    egui::TextStyle::Button.resolve(ui.style())
}

fn icon_btn_side(ui: &egui::Ui) -> f32 {
    ui.text_style_height(&egui::TextStyle::Button)
        .max(ICON_BTN_SIDE)
}

/// Framed icon-only button with optically centered glyph.
pub fn icon_button(ui: &mut egui::Ui, glyph: &str) -> egui::Response {
    icon_button_inner(ui, glyph, true, None)
}

/// Frameless icon control (optically centered); used in dense tree chrome.
pub fn icon_button_bare(ui: &mut egui::Ui, glyph: &str) -> egui::Response {
    icon_button_inner(ui, glyph, false, None)
}

/// Like [`icon_button_bare`], with an optional fixed glyph color.
pub fn icon_button_bare_colored(
    ui: &mut egui::Ui,
    glyph: &str,
    color: Option<Color32>,
) -> egui::Response {
    icon_button_inner(ui, glyph, false, color)
}

/// Like [`icon_button`], with an optional fixed glyph color (e.g. record ●).
pub fn icon_button_colored(
    ui: &mut egui::Ui,
    glyph: &str,
    color: Option<Color32>,
) -> egui::Response {
    icon_button_inner(ui, glyph, true, color)
}

fn icon_button_inner(
    ui: &mut egui::Ui,
    glyph: &str,
    framed: bool,
    color: Option<Color32>,
) -> egui::Response {
    let font_id = icon_btn_font(ui);
    let desired = Vec2::splat(icon_btn_side(ui));
    let (rect, response) = ui.allocate_exact_size(desired, Sense::click());
    let visuals = ui.style().interact(&response);
    if framed {
        ui.painter().rect(
            rect,
            visuals.corner_radius,
            visuals.weak_bg_fill,
            visuals.bg_stroke,
            egui::StrokeKind::Inside,
        );
    }
    let fg = color.unwrap_or_else(|| visuals.text_color());
    paint_text_centered(ui, rect, glyph, font_id, fg);
    response
}

/// Dark scrim behind preview overlay chips / editors.
pub fn preview_scrim() -> Color32 {
    rgba([16, 16, 16, 170])
}

/// Semi-opaque black behind labels on preview imagery.
pub fn preview_label_dim() -> Color32 {
    rgba([0, 0, 0, 150])
}

/// Red grid / outline stroke on image previews.
pub fn preview_grid_stroke() -> Color32 {
    Color32::from_rgb(255, 80, 80)
}

/// Warn stroke / label on preview atlas (unresolved collections).
pub fn preview_warn_stroke() -> Color32 {
    warn_fg()
}

/// Soft blue fill for collection bounds on atlas preview.
pub fn preview_selection_fill() -> Color32 {
    rgba([60, 100, 160, 60])
}

/// Blue stroke for collection bounds on atlas preview.
pub fn preview_selection_stroke() -> Color32 {
    Color32::from_rgb(120, 180, 255)
}

/// Best / passing match marker (pixel check).
pub fn match_pass_fg() -> Color32 {
    Color32::from_rgb(80, 255, 120)
}

/// Best match below tolerance (pixel check).
pub fn match_fail_fg() -> Color32 {
    Color32::from_rgb(255, 200, 60)
}

/// Secondary match within tolerance (pixel check).
pub fn match_within_fg() -> Color32 {
    Color32::from_rgb(120, 230, 180)
}

/// Selected card / list stroke (Sqyre primary).
pub fn selection_stroke() -> Stroke {
    Stroke::new(2.0, PRIMARY)
}

/// Soft primary tint for related-row owner highlight.
pub fn highlight_owner_fill() -> Color32 {
    rgba([0xdc, 0x9d, 0x2e, 0x28])
}

/// Soft error tint behind invalid tree rows.
pub fn highlight_invalid_fill() -> Color32 {
    rgba([220, 70, 70, 45])
}

/// Soft blue fill for execution cursor row.
pub fn highlight_cursor_fill() -> Color32 {
    rgba([90, 160, 240, 70])
}

/// Soft green fill for execution progress overlay.
pub fn highlight_progress_fill() -> Color32 {
    rgba([90, 200, 130, 90])
}

/// Icon-grid selected cell fill.
pub fn picker_selected_fill() -> Color32 {
    rgba([80, 160, 100, 60])
}

/// Icon-grid selected cell stroke.
pub fn picker_selected_stroke() -> Color32 {
    Color32::from_rgb(60, 140, 80)
}

/// DnD drop-target hover stroke on icon grid.
pub fn picker_drop_stroke() -> Color32 {
    Color32::from_rgb(80, 140, 200)
}

/// Remove-badge hover fill on icon grid.
pub fn picker_remove_hover() -> Color32 {
    Color32::from_rgb(180, 60, 60)
}

/// Collection cell selection fill.
pub fn cell_selection_fill() -> Color32 {
    rgba([60, 160, 255, 70])
}

/// Collection cell selection stroke.
pub fn cell_selection_stroke() -> Color32 {
    Color32::from_rgb(40, 140, 255)
}

/// Selected-text stroke — light cream readable on dim gold fill.
const SELECTION_FG: Color32 = Color32::from_rgb(0xf5, 0xe6, 0xc0);

/// Dark visuals with Sqyre yellow for primary accents (selection, hover, links).
pub fn dark_visuals() -> Visuals {
    let mut v = Visuals::dark();
    let dim = accent_dim();

    v.hyperlink_color = PRIMARY;
    v.warn_fg_color = PRIMARY;
    v.selection.bg_fill = dim;
    v.selection.stroke = Stroke::new(1.0, SELECTION_FG);

    // Separators / inner group outlines — dim primary.
    v.widgets.noninteractive.bg_stroke = Stroke::new(1.0, dim);

    v.widgets.hovered.bg_stroke = Stroke::new(1.0, PRIMARY);
    v.widgets.hovered.weak_bg_fill = chip_fill();
    v.widgets.hovered.bg_fill = rgba([0xdc, 0x9d, 0x2e, 0x35]);

    v.widgets.active.bg_stroke = Stroke::new(1.0, PRIMARY);
    v.widgets.active.weak_bg_fill = rgba([0xdc, 0x9d, 0x2e, 0x50]);

    v.widgets.open.bg_stroke = Stroke::new(1.0, rgba([0xdc, 0x9d, 0x2e, 0x80]));

    v.window_stroke = Stroke::new(1.0, PRIMARY);
    v.text_cursor.stroke = Stroke::new(2.0, PRIMARY);

    v
}

/// Lock dark mode and install Sqyre visuals.
pub fn apply(ctx: &egui::Context) {
    ctx.set_theme(egui::ThemePreference::Dark);
    ctx.set_visuals_of(egui::Theme::Dark, dark_visuals());
}

/// Rounded group frame with a faint Sqyre fill + dim gold stroke.
pub fn section_frame(style: &egui::Style) -> egui::Frame {
    egui::Frame::group(style)
        .fill(frame_fill())
        .stroke(inner_stroke())
        .corner_radius(CornerRadius::same(4))
        .inner_margin(egui::Margin::same(8))
}

/// Full-width framed card, then vertical `gap` after it.
pub fn framed_section(ui: &mut egui::Ui, gap: f32, add_contents: impl FnOnce(&mut egui::Ui)) {
    // Cap width *inside* the frame. Measuring outside and then applying
    // `set_max_width` ignores inner_margin/stroke, so the right border clips.
    section_frame(ui.style()).show(ui, |ui| {
        ui.set_max_width(crate::widgets::visible_width(ui));
        add_contents(ui);
    });
    ui.add_space(gap);
}

/// [`framed_section`] with a strong title, optional weak subtitle, and separator.
pub fn titled_section(
    ui: &mut egui::Ui,
    title: &str,
    subtitle: &str,
    gap: f32,
    add_contents: impl FnOnce(&mut egui::Ui),
) {
    framed_section(ui, gap, |ui| {
        ui.label(egui::RichText::new(title).strong().heading());
        if !subtitle.is_empty() {
            ui.label(egui::RichText::new(subtitle).weak());
        }
        ui.separator();
        add_contents(ui);
    });
}

/// Persist / commit button that pulses a Sqyre-yellow glow while enabled (dirty + valid).
///
/// Disabled state matches a normal `add_enabled(false, …)` button. While glowing,
/// requests continuous repaint so the loop stays smooth.
///
/// Glow is soft filled halos painted *under* an opaque button body (not concentric
/// `rect_stroke`s) so epaint stroke tessellation cannot leave a midline seam.
pub fn dirty_action_button(ui: &mut egui::Ui, label: &str, enabled: bool) -> egui::Response {
    if !enabled {
        return ui.add_enabled(false, egui::Button::new(label));
    }

    let t = ui.input(|i| i.time) as f32;
    // ~0.4 Hz full cycle; soft ease so it reads as a glow, not a blink.
    let pulse = (t * std::f32::consts::TAU * 0.4).sin().mul_add(0.5, 0.5);

    let visuals = ui.style().visuals.widgets.inactive;
    let rounding = visuals.corner_radius;
    let base_fill = visuals.weak_bg_fill;
    let pad = ui.spacing().button_padding;
    let font_id = egui::TextStyle::Button.resolve(ui.style());
    let galley = ui
        .painter()
        .layout_no_wrap(label.to_owned(), font_id, PRIMARY);
    let size = Vec2::new(
        galley.size().x + 2.0 * pad.x,
        (galley.size().y + 2.0 * pad.y).max(ui.spacing().interact_size.y),
    );

    let (rect, response) = ui.allocate_exact_size(size, Sense::click());
    let painter = ui.painter();

    // Soft bloom under the button — stacked fills, no stroked rings.
    for i in (1..=5).rev() {
        let expand = i as f32 * (1.1 + pulse * 1.6);
        let alpha = ((22.0 / i as f32) * (0.35 + 0.65 * pulse)).round() as u8;
        let expand_u8 = expand.round().clamp(0.0, 255.0) as u8;
        let glow_round = CornerRadius {
            nw: rounding.nw.saturating_add(expand_u8),
            ne: rounding.ne.saturating_add(expand_u8),
            sw: rounding.sw.saturating_add(expand_u8),
            se: rounding.se.saturating_add(expand_u8),
        };
        painter.rect_filled(
            rect.expand(expand),
            glow_round,
            Color32::from_rgba_unmultiplied(PRIMARY.r(), PRIMARY.g(), PRIMARY.b(), alpha),
        );
    }

    // Opaque body covers the halo center so nothing shows through the label.
    let tint = 0.12 + pulse * 0.22;
    let fill = Color32::from_rgb(
        ((1.0 - tint) * base_fill.r() as f32 + tint * PRIMARY.r() as f32).round() as u8,
        ((1.0 - tint) * base_fill.g() as f32 + tint * PRIMARY.g() as f32).round() as u8,
        ((1.0 - tint) * base_fill.b() as f32 + tint * PRIMARY.b() as f32).round() as u8,
    );
    // Keep stroke width fixed — animating Inside stroke width can tessellate a midline.
    let stroke = Stroke::new(1.0, PRIMARY);
    painter.rect(rect, rounding, fill, stroke, egui::StrokeKind::Inside);
    paint_galley_centered(ui, rect, galley, PRIMARY);

    ui.ctx().request_repaint();
    response
}

/// Icon-only record control (danger styling).
pub fn record_icon_button(ui: &mut egui::Ui, tip: &str, enabled: bool) -> egui::Response {
    ui.add_enabled_ui(enabled, |ui| icon_button_colored(ui, "●", Some(MACRO_STOP)))
        .inner
        .on_hover_text(tip)
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
            Some(PRIMARY)
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
        PRIMARY
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

    let tip = press_state_label(*state);
    let enabled = ui.is_enabled();
    response.widget_info(|| valued_control_info(enabled, "Press state", tip));
    response.on_hover_text(tip)
}

/// Vertical up↔down switch for Click/Key button state (`true` = down).
///
/// Top of the track is up; bottom is down. Click a half to set; arrows cycle when focused.
pub fn up_down_toggle(ui: &mut egui::Ui, down: &mut bool) -> egui::Response {
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
    let track_fill = if *down { PRIMARY } else { visuals.bg_fill };
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
    fn primary_is_sqyre_yellow() {
        assert_eq!(PRIMARY, Color32::from_rgb(220, 157, 46));
        assert_eq!(PRIMARY.to_array(), [0xdc, 0x9d, 0x2e, 0xff]);
    }

    #[test]
    fn dark_visuals_use_sqyre_accents() {
        let v = dark_visuals();
        assert!(v.dark_mode);
        assert_eq!(v.hyperlink_color, PRIMARY);
        assert_eq!(v.selection.bg_fill, accent_dim());
        assert_eq!(v.widgets.hovered.bg_stroke.color, PRIMARY);
        assert_eq!(v.window_stroke.color, PRIMARY);
    }

    #[test]
    fn window_chrome_is_strong_outer_weak_inner() {
        let style = egui::Style {
            visuals: dark_visuals(),
            ..Default::default()
        };
        assert_eq!(style.visuals.window_stroke.color, PRIMARY);
        assert_eq!(section_frame(&style).stroke.color, accent_dim());
        assert_eq!(inner_stroke().color, accent_dim());
    }

    #[test]
    fn cycle_index_wraps() {
        assert_eq!(cycle_index(4, 0, 1), 1);
        assert_eq!(cycle_index(4, 3, 1), 0);
        assert_eq!(cycle_index(4, 0, -1), 3);
        assert_eq!(cycle_index(3, 1, -1), 0);
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
