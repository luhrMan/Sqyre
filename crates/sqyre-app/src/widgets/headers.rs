//! List/section headers with a right-aligned item count.

use eframe::egui;

/// Weak `(count)` painted flush-right in the remaining row width.
fn paint_count(ui: &mut egui::Ui, count: usize) {
    ui.label(egui::RichText::new(format!("({count})")).weak());
}

fn count_row(
    ui: &mut egui::Ui,
    add_title: impl FnOnce(&mut egui::Ui) -> egui::Response,
    count: usize,
) -> egui::Response {
    // Justified layout fills the parent without raising min_size above it.
    // `allocate_ui(available_width)` would claim that width as min_size and
    // ratchet Windows toward max_size (see `crate::widgets::pin_visible`).
    let row_w = crate::widgets::visible_width(ui);
    ui.with_layout(egui::Layout::top_down_justified(egui::Align::LEFT), |ui| {
        ui.set_max_width(row_w);
        ui.horizontal(|ui| {
            let resp = add_title(ui);
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                paint_count(ui, count);
            });
            resp
        })
        .inner
    })
    .inner
}

/// Title on the left, `(count)` right-aligned. Returns the title response.
pub fn title_with_count(
    ui: &mut egui::Ui,
    title: impl Into<egui::WidgetText>,
    count: usize,
) -> egui::Response {
    count_row(
        ui,
        |ui| ui.add(egui::Label::new(title).selectable(false)),
        count,
    )
}

/// Selectable title on the left, `(count)` right-aligned. Returns the title response.
pub fn selectable_title_with_count(
    ui: &mut egui::Ui,
    selected: bool,
    title: impl Into<egui::WidgetText>,
    count: usize,
) -> egui::Response {
    count_row(ui, |ui| ui.selectable_label(selected, title), count)
}

/// Heading on the left, `(count)` right-aligned.
pub fn heading_with_count(ui: &mut egui::Ui, title: &str, count: usize) -> egui::Response {
    title_with_count(ui, egui::RichText::new(title).heading(), count)
}
