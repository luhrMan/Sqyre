//! User Settings → Permissions. Desktop runs the capability probe; Android lists the
//! system permissions the phone shell needs.

use eframe::egui::{self, Color32, RichText};

#[cfg(target_os = "android")]
mod android;
#[cfg(not(target_os = "android"))]
mod desktop;

#[cfg(target_os = "android")]
pub use android::PermissionsPanel;
#[cfg(not(target_os = "android"))]
pub use desktop::PermissionsPanel;

/// Card around one permission row.
fn row_frame(ui: &egui::Ui) -> egui::Frame {
    egui::Frame::NONE
        .fill(crate::theme::overlay_panel_fill())
        .stroke(egui::Stroke::new(
            1.0,
            ui.visuals().widgets.noninteractive.bg_stroke.color,
        ))
        .corner_radius(egui::CornerRadius::same(6))
        .inner_margin(egui::Margin::same(10))
}

/// Row title (hover shows `tip`) with the status right-aligned.
fn row_header(ui: &mut egui::Ui, title: &str, tip: &str, status: &str, color: Color32) {
    ui.horizontal(|ui| {
        ui.label(RichText::new(title).strong()).on_hover_text(tip);
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.colored_label(color, status);
        });
    });
}
