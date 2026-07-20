//! Sqyre brand theme (dark + Sqyre yellow accents).

use eframe::egui::{self, Color32, CornerRadius, Pos2, Sense, Stroke, Vec2, Visuals};

/// Sqyre gold/yellow primary (`#dc9d2e`).
pub const PRIMARY: Color32 = Color32::from_rgb(0xdc, 0x9d, 0x2e);

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
    ui.add_enabled(
        enabled,
        egui::Button::new(egui::RichText::new("●").size(16.0).color(Color32::RED)),
    )
    .on_hover_text(tip)
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
