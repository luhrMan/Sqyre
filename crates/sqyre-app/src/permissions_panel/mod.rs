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

/// Card around one permission row — shared `section_frame` styling so the
/// panel matches other framed groups in the app.
fn row_frame(ui: &egui::Ui) -> egui::Frame {
    crate::theme::section_frame(ui.style())
}

/// Row title (hover shows `tip`) with the status icon right-aligned; the status
/// word it replaces rides in the icon's hover tooltip.
///
/// The title wraps in the space left of the status. A nested scroll here sat
/// inside the settings scroller and collapsed the line to a sliver.
fn row_header(
    ui: &mut egui::Ui,
    title: &str,
    tip: &str,
    status: &str,
    glyph: &'static str,
    color: Color32,
) {
    crate::widgets::fill_row(
        ui,
        |ui| {
            crate::widgets::controls::status_icon(ui, glyph, color, status);
        },
        |ui| {
            ui.add(egui::Label::new(RichText::new(title).strong()).wrap())
                .on_hover_text(tip);
        },
    );
}
