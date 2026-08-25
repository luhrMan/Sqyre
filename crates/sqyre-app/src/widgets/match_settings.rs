//! Shared Image Search match controls (tolerance, blur, method).

use crate::action_tooltip::help as h;
use crate::widgets::{combo_str, drag_field};
use eframe::egui;
use sqyre_domain::MatchMethod;

fn tolerance_help(method: MatchMethod) -> &'static str {
    if matches!(method, MatchMethod::Sqdiff | MatchMethod::SqdiffNormed) {
        h::IS_TOLERANCE_SQDIFF
    } else if method.is_normed() {
        h::IS_TOLERANCE
    } else {
        h::IS_TOLERANCE_UNNORMED
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
    let tol_help = tolerance_help(*match_method);
    if match_method.is_normed() {
        drag_field(ui, "Tolerance", tol_help, tolerance, |d| {
            d.speed(0.01).range(0.0..=1.0)
        });
    } else {
        drag_field(ui, "Tolerance", tol_help, tolerance, |d| d.speed(1.0));
    }
    drag_field(ui, "Blur", h::IS_BLUR, blur, |d| d);
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
