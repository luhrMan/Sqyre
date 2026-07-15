//! Sqyre brand theme — mirrors Go `ui/theme.go` (dark + Sqyre yellow accents).

use eframe::egui::{self, Color32, CornerRadius, Pos2, Sense, Stroke, Vec2, Visuals};

/// Sqyre gold/yellow primary (`#dc9d2e`), same as Go `sqyrePrimary`.
pub const PRIMARY: Color32 = Color32::from_rgb(0xdc, 0x9d, 0x2e);

/// Dimmed primary for selection / hover (alpha `0x40`), Go `sqyreSelection` / `sqyreHover`.
pub fn accent_dim() -> Color32 {
    Color32::from_rgba_unmultiplied(0xdc, 0x9d, 0x2e, 0x40)
}

/// Soft tag-chip fill (~11% opacity), Go `WrapTagChip`.
pub fn chip_fill() -> Color32 {
    Color32::from_rgba_unmultiplied(0xdc, 0x9d, 0x2e, 28)
}

/// Subtle frame fill (~5% opacity), Go `WrapSqyreFrame`.
pub fn frame_fill() -> Color32 {
    Color32::from_rgba_unmultiplied(0xdc, 0x9d, 0x2e, 13)
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

    // Separators / window outline — Go maps `ColorNameSeparator` to dim primary.
    v.widgets.noninteractive.bg_stroke = Stroke::new(1.0, dim);

    v.widgets.hovered.bg_stroke = Stroke::new(1.0, PRIMARY);
    v.widgets.hovered.weak_bg_fill = chip_fill();
    v.widgets.hovered.bg_fill = Color32::from_rgba_unmultiplied(0xdc, 0x9d, 0x2e, 0x35);

    v.widgets.active.bg_stroke = Stroke::new(1.0, PRIMARY);
    v.widgets.active.weak_bg_fill = Color32::from_rgba_unmultiplied(0xdc, 0x9d, 0x2e, 0x50);

    v.widgets.open.bg_stroke =
        Stroke::new(1.0, Color32::from_rgba_unmultiplied(0xdc, 0x9d, 0x2e, 0x80));

    v.window_stroke = Stroke::new(1.0, dim);
    v.text_cursor.stroke = Stroke::new(2.0, PRIMARY);

    v
}

/// Lock dark mode (Go uses fixed dark variant) and install Sqyre visuals.
pub fn apply(ctx: &egui::Context) {
    ctx.set_theme(egui::ThemePreference::Dark);
    ctx.set_visuals_of(egui::Theme::Dark, dark_visuals());
}

/// Rounded group frame with a faint Sqyre fill + gold stroke (Go `WrapSqyreFrame`).
pub fn section_frame(style: &egui::Style) -> egui::Frame {
    egui::Frame::group(style)
        .fill(frame_fill())
        .stroke(Stroke::new(1.0, PRIMARY))
        .corner_radius(CornerRadius::same(4))
        .inner_margin(egui::Margin::same(8))
}

/// Icon-only record control (Go `MediaRecordIcon` + `DangerImportance`).
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
    let track_fill = if *down {
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
    fn primary_matches_go_sqyre_yellow() {
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
