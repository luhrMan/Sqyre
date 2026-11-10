//! Dark visuals apply + framed section chrome.

use eframe::egui::{self, Color32, CornerRadius, Stroke, Visuals};

use super::tokens::{
    accent, accent_dim, chip_fill, density, error_fg, frame_fill, inner_stroke, panel_split_stroke,
    rgba, warn_fg, SPACE_2, SPACE_4, WINDOW_RESIZE_CORNER_SIZE,
};

/// Selected-text stroke — light cream readable on dim gold fill.
const SELECTION_FG: Color32 = Color32::from_rgb(0xf5, 0xe6, 0xc0);

/// Dark visuals with Sqyre yellow for primary accents (selection, hover, links).
pub fn dark_visuals() -> Visuals {
    let mut v = Visuals::dark();
    let dim = accent_dim();
    let acc = accent();
    let acc_rgba = |a: u8| rgba([acc.r(), acc.g(), acc.b(), a]);

    v.hyperlink_color = acc;
    v.error_fg_color = error_fg();
    v.warn_fg_color = warn_fg();
    v.selection.bg_fill = dim;
    v.selection.stroke = Stroke::new(1.0, SELECTION_FG);

    // Separators + panel split lines — half-opacity structural stroke (not dim inner cards).
    v.widgets.noninteractive.bg_stroke = panel_split_stroke();

    v.widgets.hovered.bg_stroke = Stroke::new(1.0, acc);
    v.widgets.hovered.weak_bg_fill = chip_fill();
    v.widgets.hovered.bg_fill = acc_rgba(0x35);

    v.widgets.active.bg_stroke = Stroke::new(1.0, acc);
    v.widgets.active.weak_bg_fill = acc_rgba(0x50);

    v.widgets.open.bg_stroke = Stroke::new(1.0, acc_rgba(0x80));

    v.window_stroke = Stroke::new(1.0, acc);
    v.text_cursor.stroke = Stroke::new(2.0, acc);

    // Small corner grip: egui's 12pt `///` glyph covers too much of whatever
    // sits at a dialog's bottom-right. The hit zone is tightened in
    // `SettingsUi::apply_appearance` (`resize_grab_radius_corner`).
    v.resize_corner_size = WINDOW_RESIZE_CORNER_SIZE;

    v
}

/// Lock dark mode and install Sqyre visuals.
pub fn apply(ctx: &egui::Context) {
    ctx.set_theme(egui::ThemePreference::Dark);
    ctx.set_visuals_of(egui::Theme::Dark, dark_visuals());
}

/// Floating scrollbars that stay readable when content overflows.
///
/// `floating_allocated_width` stays 0: bars are a pure overlay at the viewport
/// edge, so content keeps its full width/height and no lane is reserved. The
/// thin dormant bar and the expanded hover bar (plus its `bar_width` hit rect)
/// sit on top of the content edge — [`crate::widgets::visible_content_width`]
/// subtracts that overlay where chrome must stay clear of it.
/// Dormant handle opacity is raised so H/V bars are visible without requiring
/// hover — otherwise narrow panels look clipped.
pub fn scroll_style() -> egui::style::ScrollStyle {
    let mut s = egui::style::ScrollStyle::floating();
    s.dormant_handle_opacity = 0.55;
    s.active_handle_opacity = 1.0;
    s.interact_handle_opacity = 1.0;
    s.dormant_background_opacity = 0.25;
    s.active_background_opacity = 0.5;
    s.interact_background_opacity = 0.8;
    s
}

/// Inner margin for popup / tip / HUD chrome — density-aware SSOT.
///
/// Compact/Normal: [`SPACE_4`]×[`SPACE_2`]. Spaced: element×half-element.
/// Replaces one-off `(8,6)` / `(10,6)` / `(10,8)` / `same(6)` popup pads.
pub fn chrome_inner_margin(ui: &egui::Ui) -> egui::Margin {
    chrome_inner_margin_ctx(ui.ctx())
}

/// [`chrome_inner_margin`] when only a [`egui::Context`] is available (Modal /
/// Window builders before the body `Ui` exists).
pub fn chrome_inner_margin_ctx(ctx: &egui::Context) -> egui::Margin {
    let e = density(ctx).element;
    let h = e.max(SPACE_4);
    let v = (e * 0.5).max(SPACE_2);
    egui::Margin::symmetric(h as i8, v as i8)
}

/// `Frame::popup` + [`chrome_inner_margin`] — the one popup chrome frame.
pub fn popup_frame(ui: &egui::Ui) -> egui::Frame {
    egui::Frame::popup(ui.style()).inner_margin(chrome_inner_margin(ui))
}

/// [`popup_frame`] from a context (permissions Modal, edit-tip Window frame).
pub fn popup_frame_ctx(ctx: &egui::Context) -> egui::Frame {
    egui::Frame::popup(ctx.global_style().as_ref()).inner_margin(chrome_inner_margin_ctx(ctx))
}

/// Rounded group frame with a faint Sqyre fill + dim gold stroke.
///
/// Inner margin tracks density element gap, clamped to [`SPACE_2`]…[`SPACE_4`]
/// so Compact cards tighten and Spaced cards do not inflate past Normal.
pub fn section_frame(ui: &egui::Ui) -> egui::Frame {
    let m = density(ui.ctx()).element.clamp(SPACE_2, SPACE_4) as i8;
    egui::Frame::group(ui.style())
        .fill(frame_fill())
        .stroke(inner_stroke())
        .corner_radius(CornerRadius::same(4))
        .inner_margin(egui::Margin::same(m))
}

/// Full-width framed card, then a [`super::section_gap`] gap after it.
pub fn framed_section(ui: &mut egui::Ui, add_contents: impl FnOnce(&mut egui::Ui)) {
    // Cap width *inside* the frame. Measuring outside and then applying
    // `set_max_width` ignores inner_margin/stroke, so the right border clips.
    // Clip-anchored: the reserved scrollbar lane insets the clip, not max_rect.
    section_frame(ui).show(ui, |ui| {
        crate::widgets::clamp_row_right_to_clip(ui);
        add_contents(ui);
    });
    super::add_section_gap(ui);
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
        assert_eq!(v.resize_corner_size, WINDOW_RESIZE_CORNER_SIZE);
    }

    #[test]
    fn scrollbars_overlay_the_edge_without_reserving_a_lane() {
        let s = scroll_style();
        assert!(s.floating);
        // Zero allocated width: egui leaves the content rect and clip
        // untouched and paints the bar over the viewport edge, so scrollable
        // content keeps the full extent and bars consume no space.
        assert_eq!(s.floating_allocated_width, 0.0);
        assert_eq!(s.allocated_width(), 0.0);
    }

    #[test]
    fn window_chrome_is_strong_outer_weak_inner() {
        let style = egui::Style {
            visuals: dark_visuals(),
            ..Default::default()
        };
        assert_eq!(style.visuals.window_stroke.color, PRIMARY);
        assert_eq!(inner_stroke().color, accent_dim());
        assert_eq!(panel_split_stroke().color.a(), 0x80);
    }

    #[test]
    fn chrome_inner_margin_tracks_density() {
        let ctx = egui::Context::default();
        // Default density = Normal (element 4) → 4×2.
        let m = chrome_inner_margin_ctx(&ctx);
        assert_eq!(m.left, SPACE_4 as i8);
        assert_eq!(m.top, SPACE_2 as i8);

        crate::theme::set_density(&ctx, crate::theme::DensityGaps::new(4.0, 2.0));
        let compact = chrome_inner_margin_ctx(&ctx);
        assert_eq!(compact.left, SPACE_4 as i8);
        assert_eq!(compact.top, SPACE_2 as i8);

        crate::theme::set_density(&ctx, crate::theme::DensityGaps::new(12.0, 8.0));
        let spaced = chrome_inner_margin_ctx(&ctx);
        assert_eq!(spaced.left, crate::theme::SPACE_8 as i8);
        assert_eq!(spaced.top, SPACE_4 as i8);
    }

    #[test]
    fn popup_and_section_frames_use_shared_margins() {
        let ctx = egui::Context::default();
        ctx.run_ui(egui::RawInput::default(), |ui| {
            let popup = popup_frame(ui);
            assert_eq!(popup.inner_margin, chrome_inner_margin(ui));
            assert_eq!(section_frame(ui).stroke.color, accent_dim());
            let m = section_frame(ui).inner_margin;
            assert!(m.left >= SPACE_2 as i8 && m.left <= SPACE_4 as i8);
        })
        .drop_without_applying_deltas();
    }
}
