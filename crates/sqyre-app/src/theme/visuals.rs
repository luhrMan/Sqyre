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

    // Separators + panel split lines — half-opacity structural stroke (not dim inner cards).
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

/// Floating scrollbars that stay readable when content overflows.
///
/// `floating_allocated_width = bar_width` reserves the full bar lane while a bar
/// is shown, so the thin dormant bar — and the expanded hover bar — never paint
/// or hit-test on top of widgets (egui's default of 0 draws them over content).
/// Overflow is still measured against the outer rect for floating styles, so
/// the reserved lane cannot trigger an extra H/V bar. Dormant handle opacity is
/// raised so H/V bars are visible without requiring hover — otherwise narrow
/// panels look clipped.
pub fn scroll_style() -> egui::style::ScrollStyle {
    let mut s = egui::style::ScrollStyle::floating();
    s.floating_allocated_width = s.bar_width;
    s.dormant_handle_opacity = 0.55;
    s.active_handle_opacity = 1.0;
    s.interact_handle_opacity = 1.0;
    s.dormant_background_opacity = 0.25;
    s.active_background_opacity = 0.5;
    s.interact_background_opacity = 0.8;
    s
}

/// Rounded group frame with a faint Sqyre fill + dim gold stroke.
pub fn section_frame(style: &egui::Style) -> egui::Frame {
    egui::Frame::group(style)
        .fill(frame_fill())
        .stroke(inner_stroke())
        .corner_radius(CornerRadius::same(4))
        .inner_margin(egui::Margin::same(8))
}

/// Full-width framed card, then a [`super::section_gap`] gap after it.
pub fn framed_section(ui: &mut egui::Ui, add_contents: impl FnOnce(&mut egui::Ui)) {
    // Cap width *inside* the frame. Measuring outside and then applying
    // `set_max_width` ignores inner_margin/stroke, so the right border clips.
    // Clip-anchored: the reserved scrollbar lane insets the clip, not max_rect.
    section_frame(ui.style()).show(ui, |ui| {
        crate::widgets::clamp_row_right_to_clip(ui);
        add_contents(ui);
    });
    ui.add_space(super::section_gap(ui));
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
    }

    #[test]
    fn scrollbar_lane_is_reserved_so_bars_never_cover_widgets() {
        let s = scroll_style();
        assert!(s.floating);
        // egui insets the content rect and clip by allocated_width() while a bar
        // shows; reserving the full bar width keeps even the expanded hover bar
        // (and its hit-test rect) off widgets. `floating()` defaults to 0 and
        // paints the bar on top of content.
        assert_eq!(s.floating_allocated_width, s.bar_width);
        assert_eq!(s.allocated_width(), s.bar_width);
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
        assert_eq!(panel_split_stroke().color.a(), 0x80);
    }
}
