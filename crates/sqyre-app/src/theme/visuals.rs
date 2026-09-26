//! Dark visuals apply + framed section chrome.

use eframe::egui::{self, Color32, CornerRadius, Stroke, Visuals};

use super::tokens::{
    accent_dim, chip_fill, error_fg, frame_fill, inner_stroke, panel_split_stroke, rgba, warn_fg,
    PRIMARY,
};

/// Selected-text stroke — light cream readable on dim gold fill.
const SELECTION_FG: Color32 = Color32::from_rgb(0xf5, 0xe6, 0xc0);

/// Dark visuals with Sqyre yellow for primary accents (selection, hover, links).
pub fn dark_visuals() -> Visuals {
    let mut v = Visuals::dark();
    let dim = accent_dim();

    v.hyperlink_color = PRIMARY;
    v.error_fg_color = error_fg();
    v.warn_fg_color = warn_fg();
    v.selection.bg_fill = dim;
    v.selection.stroke = Stroke::new(1.0, SELECTION_FG);

    // Separators + panel split lines — opaque structural stroke (not dim inner cards).
    v.widgets.noninteractive.bg_stroke = panel_split_stroke();

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::tokens::{accent_dim, inner_stroke, panel_split_stroke, PRIMARY};

    #[test]
    fn dark_visuals_use_sqyre_accents() {
        let v = dark_visuals();
        assert!(v.dark_mode);
        assert_eq!(v.hyperlink_color, PRIMARY);
        assert_eq!(v.error_fg_color, error_fg());
        assert_eq!(v.warn_fg_color, warn_fg());
        assert_eq!(v.selection.bg_fill, accent_dim());
        assert_eq!(v.widgets.hovered.bg_stroke.color, PRIMARY);
        assert_eq!(v.window_stroke.color, PRIMARY);
        assert_eq!(v.widgets.noninteractive.bg_stroke, panel_split_stroke());
        assert_eq!(v.widgets.noninteractive.bg_stroke.color.a(), 255);
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
        assert_eq!(panel_split_stroke().color, PRIMARY);
    }
}
