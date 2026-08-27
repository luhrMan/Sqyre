//! Shared Image Search match controls (tolerance, blur, method).

use crate::action_tooltip::help as h;
use crate::widgets::{combo_str, drag_field};
use eframe::egui;
use sqyre_domain::{
    clamp_match_blur, clamp_match_tolerance, MatchMethod, MAX_MATCH_BLUR,
    MAX_NORMED_MATCH_TOLERANCE, MAX_UNNORMED_MATCH_TOLERANCE, MIN_MATCH_BLUR, MIN_MATCH_TOLERANCE,
};

fn tolerance_help(method: MatchMethod) -> &'static str {
    if matches!(method, MatchMethod::Sqdiff | MatchMethod::SqdiffNormed) {
        h::IS_TOLERANCE_SQDIFF
    } else if method.is_normed() {
        h::IS_TOLERANCE
    } else {
        h::IS_TOLERANCE_UNNORMED
    }
}

pub fn configure_match_blur_drag(d: egui::DragValue<'_>) -> egui::DragValue<'_> {
    d.speed(1).range(MIN_MATCH_BLUR..=MAX_MATCH_BLUR)
}

pub fn configure_match_tolerance_drag(
    d: egui::DragValue<'_>,
    method: MatchMethod,
) -> egui::DragValue<'_> {
    if method.is_normed() {
        d.speed(0.01)
            .range(MIN_MATCH_TOLERANCE..=MAX_NORMED_MATCH_TOLERANCE)
            .max_decimals(3)
    } else {
        d.speed(1.0)
            .range(MIN_MATCH_TOLERANCE..=MAX_UNNORMED_MATCH_TOLERANCE)
    }
}

/// Tolerance + blur (+ optional method combo).
pub fn paint_match_settings(
    ui: &mut egui::Ui,
    tolerance: &mut f64,
    blur: &mut i32,
    match_method: &mut MatchMethod,
    show_method: bool,
) {
    *blur = clamp_match_blur(*blur);
    *tolerance = clamp_match_tolerance(*tolerance, *match_method);

    let tol_help = tolerance_help(*match_method);
    drag_field(ui, "Tolerance", tol_help, tolerance, |d| {
        configure_match_tolerance_drag(d, *match_method)
    });
    drag_field(ui, "Blur", h::IS_BLUR, blur, configure_match_blur_drag);
    if show_method {
        paint_match_method(ui, match_method);
    }
}

/// Match method combo only.
pub fn paint_match_method(ui: &mut egui::Ui, match_method: &mut MatchMethod) {
    let mut method_label = match_method.label().to_string();
    let method_opts: Vec<&str> = MatchMethod::ALL.iter().map(|m| m.label()).collect();
    combo_str(ui, "Method", h::IS_METHOD, &mut method_label, &method_opts);
    if let Some(m) = MatchMethod::ALL
        .iter()
        .copied()
        .find(|m| m.label() == method_label)
    {
        *match_method = m;
    }
}
