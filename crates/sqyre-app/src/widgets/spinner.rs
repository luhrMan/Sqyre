//! Brand loading spinner: the Sqyre S drawn from its real upper/lower
//! strokes, with one gap sweeping down the S's own figure-8.
//! Drop-in replacement for [`egui::Spinner`]; geometry and timing are
//! shared with the native overlay's busy indicator via
//! [`sqyre_overlay::icons::paint_trace_spinner`].

use eframe::egui::{self, Response, Sense, Ui, WidgetInfo, WidgetType};

/// Brand loading spinner sized like [`egui::Spinner`] (`interact_size.y`).
pub fn brand_spinner(ui: &mut Ui) -> Response {
    brand_spinner_sized(ui, ui.spacing().interact_size.y)
}

/// Brand loading spinner at an explicit square side (points).
pub fn brand_spinner_sized(ui: &mut Ui, side: f32) -> Response {
    let (rect, response) = ui.allocate_exact_size(egui::vec2(side, side), Sense::hover());
    response.widget_info(|| WidgetInfo::new(WidgetType::ProgressIndicator));
    if !ui.is_rect_visible(rect) {
        return response;
    }
    let glyph = egui::Rect::from_center_size(rect.center(), rect.size() * 0.92);
    sqyre_overlay::icons::paint_trace_spinner(ui, glyph, sqyre_ui_theme::accent());
    // Animated — request_repaint_after, not a bare request_repaint, which can
    // be dropped on an idle window and freeze the sweep (see ui_toolbar).
    ui.ctx()
        .request_repaint_after(sqyre_overlay::icons::SPINNER_TICK);
    response
}

/// Brand spinner beside a loading message. The spinner keeps the UI repainting.
pub fn loading_line(ui: &mut Ui, text: impl Into<egui::WidgetText>) -> Response {
    ui.horizontal(|ui| {
        brand_spinner(ui);
        ui.label(text)
    })
    .inner
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spinner_keeps_requesting_repaints() {
        let mut h = egui_kittest::Harness::builder()
            .with_size([64.0, 64.0])
            .build_ui(|ui| {
                brand_spinner(ui);
            });
        h.input_mut().time = Some(0.0);
        h.run_steps(1);
        let delay = h.output().viewport_output[&egui::ViewportId::ROOT].repaint_delay;
        assert!(
            delay < std::time::Duration::MAX,
            "an animated spinner must keep scheduling repaints"
        );
    }

    #[test]
    fn loading_line_shows_the_message_beside_the_spinner() {
        use egui_kittest::kittest::Queryable;
        let mut h = egui_kittest::Harness::builder()
            .with_size([240.0, 48.0])
            .build_ui(|ui| {
                loading_line(ui, "Loading windows…");
            });
        h.run_steps(1);
        h.get_by_label("Loading windows…");
        h.get_by_role(egui::accesskit::Role::ProgressIndicator);
    }
}
