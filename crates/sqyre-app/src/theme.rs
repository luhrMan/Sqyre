//! Sqyre brand theme (dark + Sqyre yellow accents).

use eframe::egui::{self, Color32, CornerRadius, Pos2, Sense, Stroke, Vec2, Visuals};

/// Sqyre gold/yellow primary (`#dc9d2e`).
pub const PRIMARY: Color32 = Color32::from_rgb(0xdc, 0x9d, 0x2e);

/// Start macro / add controls (`#36a258`).
pub const MACRO_START: Color32 = Color32::from_rgb(0x36, 0xa2, 0x58);

/// Stop macro / remove controls (`#e44134`).
pub const MACRO_STOP: Color32 = Color32::from_rgb(0xe4, 0x41, 0x34);

/// Convert `[r,g,b,a]` to egui [`Color32`] (unmultiplied).
pub fn rgba(c: [u8; 4]) -> Color32 {
    Color32::from_rgba_unmultiplied(c[0], c[1], c[2], c[3])
}

/// Dim floating panel fill used by macro / recording overlays.
pub fn overlay_panel_fill() -> Color32 {
    rgba([20, 18, 14, 230])
}

/// Dimmed primary for selection / hover (alpha `0x40`).
pub fn accent_dim() -> Color32 {
    rgba([0xdc, 0x9d, 0x2e, 0x40])
}

/// Foreground that contrasts with a pastel/solid fill (Rec.601 luminance).
pub fn contrast_fg(bg: Color32) -> Color32 {
    let lum = 0.299 * bg.r() as f32 + 0.587 * bg.g() as f32 + 0.114 * bg.b() as f32;
    if lum > 140.0 {
        Color32::from_rgb(30, 30, 30)
    } else {
        Color32::from_rgb(240, 240, 240)
    }
}

/// Place galley so its ink (mesh bounds) is centered in `rect`.
pub fn paint_galley_centered(
    ui: &mut egui::Ui,
    rect: egui::Rect,
    galley: std::sync::Arc<egui::Galley>,
    fallback: Color32,
) {
    let pos = if galley.mesh_bounds.is_positive() {
        // Optical center: baseline metrics make the layout box look top-heavy.
        rect.center() - galley.mesh_bounds.center().to_vec2()
    } else {
        egui::Align2::CENTER_CENTER
            .anchor_size(rect.center(), galley.size())
            .min
    };
    ui.painter().galley(pos, galley, fallback);
}

/// Layout and paint a single-line glyph/text optically centered in `rect`.
pub fn paint_text_centered(
    ui: &mut egui::Ui,
    rect: egui::Rect,
    text: impl Into<String>,
    font_id: egui::FontId,
    color: Color32,
) {
    let galley = ui.painter().layout_no_wrap(text.into(), font_id, color);
    paint_galley_centered(ui, rect, galley, color);
}

/// Glyph font size for all icon-only buttons.
pub const ICON_BTN_FONT: f32 = 14.0;

/// Fixed square hit target for all icon-only buttons (framed and bare).
pub const ICON_BTN_SIDE: f32 = 18.0;

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
    let font_id = egui::FontId::proportional(ICON_BTN_FONT);
    let desired = Vec2::splat(ICON_BTN_SIDE);
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

/// Soft error / failure text (Find Pixel dropper, status banners).
pub fn error_fg() -> Color32 {
    Color32::from_rgb(220, 80, 80)
}

/// Soft success text for status banners.
pub fn ok_fg() -> Color32 {
    Color32::from_rgb(80, 160, 80)
}

/// Soft tag-chip fill (~11% opacity).
pub fn chip_fill() -> Color32 {
    rgba([0xdc, 0x9d, 0x2e, 28])
}

/// Subtle frame fill (~5% opacity).
pub fn frame_fill() -> Color32 {
    rgba([0xdc, 0x9d, 0x2e, 13])
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

    // Separators / window outline — dim primary.
    v.widgets.noninteractive.bg_stroke = Stroke::new(1.0, dim);

    v.widgets.hovered.bg_stroke = Stroke::new(1.0, PRIMARY);
    v.widgets.hovered.weak_bg_fill = chip_fill();
    v.widgets.hovered.bg_fill = rgba([0xdc, 0x9d, 0x2e, 0x35]);

    v.widgets.active.bg_stroke = Stroke::new(1.0, PRIMARY);
    v.widgets.active.weak_bg_fill = rgba([0xdc, 0x9d, 0x2e, 0x50]);

    v.widgets.open.bg_stroke = Stroke::new(1.0, rgba([0xdc, 0x9d, 0x2e, 0x80]));

    v.window_stroke = Stroke::new(1.0, dim);
    v.text_cursor.stroke = Stroke::new(2.0, PRIMARY);

    v
}

/// Lock dark mode and install Sqyre visuals.
pub fn apply(ctx: &egui::Context) {
    ctx.set_theme(egui::ThemePreference::Dark);
    ctx.set_visuals_of(egui::Theme::Dark, dark_visuals());
}

/// Rounded group frame with a faint Sqyre fill + gold stroke.
pub fn section_frame(style: &egui::Style) -> egui::Frame {
    egui::Frame::group(style)
        .fill(frame_fill())
        .stroke(Stroke::new(1.0, PRIMARY))
        .corner_radius(CornerRadius::same(4))
        .inner_margin(egui::Margin::same(8))
}

/// Full-width framed card, then vertical `gap` after it.
///
/// Used by tip sections and settings panels so chrome stays consistent.
pub fn framed_section(ui: &mut egui::Ui, gap: f32, add_contents: impl FnOnce(&mut egui::Ui)) {
    section_frame(ui.style()).show(ui, |ui| {
        ui.set_min_width(ui.available_width());
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
        ui.label(egui::RichText::new(title).strong().size(16.0));
        if !subtitle.is_empty() {
            ui.label(egui::RichText::new(subtitle).weak());
        }
        ui.separator();
        add_contents(ui);
    });
}

/// Icon-only record control (danger styling).
pub fn record_icon_button(ui: &mut egui::Ui, tip: &str, enabled: bool) -> egui::Response {
    ui.add_enabled_ui(enabled, |ui| {
        icon_button_colored(ui, "●", Some(Color32::RED))
    })
    .inner
    .on_hover_text(tip)
}

/// Top-down mouse for Click button selection.
///
/// Left / right buttons, center wheel (middle), lower body (scroll).
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

    let tip = match hover_btn.unwrap_or(*button) {
        MouseButton::Left => "Left",
        MouseButton::Right => "Right",
        MouseButton::Middle => "Middle",
        MouseButton::Scroll => "Scroll",
    };
    response.on_hover_text(tip)
}

/// Vertical up / tap / down switch for Click/Key press state.
///
/// Top = up, middle = tap, bottom = down. Click toggles; click a zone to set.
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

    response.on_hover_text(match *state {
        PressState::Up => "Up",
        PressState::Tap => "Tap",
        PressState::Down => "Down",
    })
}

/// Vertical up↔down switch for Click/Key button state (`true` = down).
///
/// Top of the track is up; bottom is down. Click toggles; click a half to set.
pub fn up_down_toggle(ui: &mut egui::Ui, down: &mut bool) -> egui::Response {
    const TRACK_W: f32 = 18.0;
    const TRACK_H: f32 = 36.0;
    const PAD: f32 = 2.0;

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

    response.on_hover_text(if *down { "Down" } else { "Up" })
}

#[cfg(test)]
mod tests {
    use super::*;

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
    }
}
