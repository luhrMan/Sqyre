//! Shared row / pill sizing for tree chrome and var pills.
//!
//! Kept free of `tree_chrome` / `var_pills` imports so those modules can both
//! depend on this leaf without forming a cycle.

use eframe::egui::{self, Color32};

/// Icon badge edge length (tree type column / chrome).
pub(crate) const ICON_SIZE: f32 = 18.0;

/// Outer display padding Y for summary pills (matches var-pill chrome).
const OUTER_MARGIN_Y: f32 = 2.0;
/// Nested chip padding Y inside a summary pill.
const NESTED_MARGIN_Y: f32 = 1.0;

/// Tallest summary pill (outer chrome around a nested chip) at the current Small font.
///
/// Measured from a laid-out galley, not `text_style_height`, so pixel-snapped
/// glyph bounds at fractional/HiDPI scales are included. Rounded up to a whole
/// physical pixel so it can serve as a layout band height.
pub fn summary_pill_height(ui: &egui::Ui) -> f32 {
    let font = egui::TextStyle::Small.resolve(ui.style());
    let text_h = ui
        .painter()
        .layout_no_wrap("Ag".to_owned(), font, Color32::WHITE)
        .size()
        .y;
    let h = text_h + (OUTER_MARGIN_Y + NESTED_MARGIN_Y) * 2.0;
    let ppp = ui.ctx().pixels_per_point();
    (h * ppp).ceil() / ppp
}

/// Default tree-row height (icon column + chrome).
pub fn default_row_height(interact_y: f32) -> f32 {
    interact_y.max(ICON_SIZE)
}

fn type_badge_side(ui: &egui::Ui) -> f32 {
    ui.text_style_height(&egui::TextStyle::Small).max(ICON_SIZE)
}

/// Row height including type-badge, summary pills, and icon-button glyphs that track Font size.
pub fn row_height(ui: &egui::Ui) -> f32 {
    let btn_h = crate::widgets::icon_btn_side(ui);
    default_row_height(ui.spacing().interact_size.y)
        .max(type_badge_side(ui))
        .max(btn_h)
        .max(summary_pill_height(ui))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_row_height_is_at_least_icon() {
        assert_eq!(default_row_height(10.0), ICON_SIZE);
        assert_eq!(default_row_height(24.0), 24.0);
    }
}
