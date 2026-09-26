//! Shared Sqyre brand and semantic color tokens.
//!
//! Single source of truth for `sqyre-app` and `sqyre-overlay`. Widgets stay in
//! the app; this crate holds colors, strokes, and minimal paint helpers both
//! surfaces need.

use egui::{self, Color32, Stroke};

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

/// Soft error / failure text (Find Pixel dropper, status banners).
///
/// App status chrome and egui `Visuals::error_fg_color` must use this — not
/// ad-hoc RGB or `Color32::RED`. PixelCheck heatmap markers are separate viz
/// tokens in `sqyre-app::theme` (`match_*_fg`).
pub fn error_fg() -> Color32 {
    Color32::from_rgb(220, 80, 80)
}

/// Soft warning text (platform/session advisories).
pub fn warn_fg() -> Color32 {
    Color32::from_rgb(220, 160, 60)
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

/// Dim gold stroke for inner cards and previews (weaker than window chrome).
pub fn inner_stroke() -> Stroke {
    Stroke::new(1.0, accent_dim())
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn primary_is_sqyre_yellow() {
        assert_eq!(PRIMARY, Color32::from_rgb(220, 157, 46));
        assert_eq!(PRIMARY.to_array(), [0xdc, 0x9d, 0x2e, 0xff]);
    }

    #[test]
    fn semantic_fg_roles_are_distinct() {
        assert_ne!(error_fg(), warn_fg());
        assert_ne!(warn_fg(), ok_fg());
        assert_ne!(error_fg(), ok_fg());
    }

    #[test]
    fn brand_fills_derive_from_primary() {
        assert_eq!(accent_dim(), rgba([0xdc, 0x9d, 0x2e, 0x40]));
        assert_eq!(chip_fill(), rgba([0xdc, 0x9d, 0x2e, 28]));
        assert_eq!(frame_fill(), rgba([0xdc, 0x9d, 0x2e, 13]));
        assert_eq!(inner_stroke().color, accent_dim());
    }
}
