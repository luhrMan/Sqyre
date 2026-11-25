//! Shared text-measure helpers for compact chrome rows.

use eframe::egui;

/// Unwrapped width of `text` in `style`.
pub fn text_width(ui: &egui::Ui, text: &str, style: egui::TextStyle) -> f32 {
    egui::WidgetText::from(text)
        .into_galley(ui, Some(egui::TextWrapMode::Extend), f32::INFINITY, style)
        .size()
        .x
}
