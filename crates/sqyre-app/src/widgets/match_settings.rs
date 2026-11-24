//! Shared Image Search match controls (tolerance, blur, scale, method, plane flip).

use crate::action_tooltip::help as h;
use crate::widgets::{combo_enum, combo_str, drag_field, NumEntry};
use eframe::egui;
use sqyre_domain::{
    clamp_match_blur, clamp_match_scale_percent, clamp_match_tolerance, MatchMethod, PlaneFlip,
    MAX_MATCH_BLUR, MAX_MATCH_SCALE_PERCENT, MAX_NORMED_MATCH_TOLERANCE,
    MAX_UNNORMED_MATCH_TOLERANCE, MIN_MATCH_BLUR, MIN_MATCH_SCALE_PERCENT, MIN_MATCH_TOLERANCE,
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

pub fn configure_match_blur_drag<Num: egui::emath::Numeric>(
    d: NumEntry<'_, Num>,
) -> NumEntry<'_, Num> {
    d.speed(1).range(MIN_MATCH_BLUR..=MAX_MATCH_BLUR)
}

pub fn configure_match_scale_drag<Num: egui::emath::Numeric>(
    d: NumEntry<'_, Num>,
) -> NumEntry<'_, Num> {
    d.speed(1)
        .range(MIN_MATCH_SCALE_PERCENT..=MAX_MATCH_SCALE_PERCENT)
        .suffix("%")
}

pub fn configure_match_tolerance_drag<Num: egui::emath::Numeric>(
    d: NumEntry<'_, Num>,
    method: MatchMethod,
) -> NumEntry<'_, Num> {
    if method.is_normed() {
        d.speed(0.01)
            .range(MIN_MATCH_TOLERANCE..=MAX_NORMED_MATCH_TOLERANCE)
            .max_decimals(3)
    } else {
        d.speed(1.0)
            .range(MIN_MATCH_TOLERANCE..=MAX_UNNORMED_MATCH_TOLERANCE)
    }
}

/// Tolerance + blur + match scale (+ optional method combo).
pub fn paint_match_settings(
    ui: &mut egui::Ui,
    tolerance: &mut f64,
    blur: &mut i32,
    match_scale_percent: &mut i32,
    match_method: &mut MatchMethod,
    show_method: bool,
) {
    *blur = clamp_match_blur(*blur);
    *match_scale_percent = clamp_match_scale_percent(*match_scale_percent);
    *tolerance = clamp_match_tolerance(*tolerance, *match_method);

    let tol_help = tolerance_help(*match_method);
    drag_field(
        ui,
        egui_phosphor::regular::FADERS,
        "Tolerance",
        tol_help,
        tolerance,
        |d| configure_match_tolerance_drag(d, *match_method),
    );
    drag_field(
        ui,
        egui_phosphor::regular::DROP,
        "Blur",
        h::IS_BLUR,
        blur,
        configure_match_blur_drag,
    );
    drag_field(
        ui,
        egui_phosphor::regular::ARROWS_IN_SIMPLE,
        "Match scale",
        h::IS_MATCH_SCALE,
        match_scale_percent,
        configure_match_scale_drag,
    );
    if show_method {
        paint_match_method(ui, match_method);
    }
}

/// Tolerance, blur, and scale, then `advanced` for the method (and any siblings).
///
/// The caller owns the disclosure: image search uses `tip_advanced`; forms use
/// `advanced_section`. `advanced` receives `match_method` so it is not borrowed
/// twice.
pub fn paint_match_settings_advanced(
    ui: &mut egui::Ui,
    tolerance: &mut f64,
    blur: &mut i32,
    match_scale_percent: &mut i32,
    match_method: &mut MatchMethod,
    advanced: impl FnOnce(&mut egui::Ui, &mut MatchMethod),
) {
    paint_match_settings(
        ui,
        tolerance,
        blur,
        match_scale_percent,
        match_method,
        false,
    );
    advanced(ui, match_method);
}

/// Match method combo only (everyday [`MatchMethod::ui_label`] names).
pub fn paint_match_method(ui: &mut egui::Ui, match_method: &mut MatchMethod) {
    let mut method_label = match_method.ui_label().to_string();
    let method_opts: Vec<&str> = MatchMethod::UI_ORDER.iter().map(|m| m.ui_label()).collect();
    combo_str(
        ui,
        egui_phosphor::regular::FUNCTION,
        "Method",
        h::IS_METHOD,
        &mut method_label,
        &method_opts,
    );
    if let Some(m) = MatchMethod::UI_ORDER
        .iter()
        .copied()
        .find(|m| m.ui_label() == method_label)
    {
        *match_method = m;
    }
}

/// Plane flip combo (Image Search Advanced + Match Check).
pub fn paint_plane_flip(ui: &mut egui::Ui, plane_flip: &mut PlaneFlip) {
    combo_enum(
        ui,
        egui_phosphor::regular::ARROWS_LEFT_RIGHT,
        "Plane flip",
        h::IS_PLANE_FLIP,
        plane_flip,
        &PlaneFlip::ALL,
        PlaneFlip::ui_label,
    );
}
