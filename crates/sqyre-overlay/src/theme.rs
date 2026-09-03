//! Minimal brand colors / text helpers for overlay chrome (kept out of sqyre-app).

use egui::{self, Color32, Stroke};

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

/// Soft tag-chip fill (~11% opacity).
pub fn chip_fill() -> Color32 {
    rgba([0xdc, 0x9d, 0x2e, 28])
}

/// Subtle frame fill (~5% opacity).
pub fn frame_fill() -> Color32 {
    rgba([0xdc, 0x9d, 0x2e, 13])
}

/// Dim gold stroke for inner cards and previews.
pub fn inner_stroke() -> Stroke {
    Stroke::new(1.0, accent_dim())
}

/// Place galley so its ink (mesh bounds) is centered in `rect`.
pub fn paint_galley_centered(
    ui: &mut egui::Ui,
    rect: egui::Rect,
    galley: std::sync::Arc<egui::Galley>,
    fallback: Color32,
) {
    let pos = if galley.mesh_bounds.is_positive() {
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
