//! Sqyre brand theme — mirrors Go `ui/theme.go` (dark + Sqyre yellow accents).

use eframe::egui::{self, Color32, CornerRadius, Stroke, Visuals};

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
