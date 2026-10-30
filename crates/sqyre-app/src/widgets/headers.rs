//! List/section headers with a right-aligned item count.

use eframe::egui;

/// The `(n)` count string [`count_suffix`] paints — also for width pre-measure.
pub fn count_text(count: usize) -> String {
    format!("({count})")
}

/// Weak `(count)` label.
pub fn count_suffix(ui: &mut egui::Ui, count: usize) {
    ui.label(egui::RichText::new(count_text(count)).weak());
}

fn count_row(
    ui: &mut egui::Ui,
    add_title: impl FnOnce(&mut egui::Ui) -> egui::Response,
    count: usize,
) -> egui::Response {
    // Justified layout fills the parent without raising min_size above it.
    // `allocate_ui(available_width)` would claim that width as min_size and
    // ratchet Windows toward max_size (see `crate::widgets::visible_width`).
    ui.with_layout(egui::Layout::top_down_justified(egui::Align::LEFT), |ui| {
        crate::widgets::clamp_row_right_to_clip(ui);
        ui.horizontal(|ui| {
            let resp = add_title(ui);
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                count_suffix(ui, count);
            });
            resp
        })
        .inner
    })
    .inner
}

/// Title with a weak `(count)` right after it. Returns the title response.
///
/// Sized to its text so content-sized sections (framed tips, cards) do not
/// stretch to full width; panel headers use [`heading_with_count`].
pub fn title_with_count(
    ui: &mut egui::Ui,
    title: impl Into<egui::WidgetText>,
    count: usize,
) -> egui::Response {
    ui.horizontal(|ui| {
        let resp = ui.add(egui::Label::new(title).selectable(false));
        count_suffix(ui, count);
        resp
    })
    .inner
}

/// Heading on the left, `(count)` right-aligned across the panel.
pub fn heading_with_count(ui: &mut egui::Ui, title: &str, count: usize) -> egui::Response {
    count_row(
        ui,
        |ui| ui.add(egui::Label::new(egui::RichText::new(title).heading()).selectable(false)),
        count,
    )
}
