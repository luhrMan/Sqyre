//! Shared scaffold for the detection actions (Image Search, OCR, Find Pixel).
//!
//! Primary fields stay visible; output coords / wait+order (and Image Search
//! Method, OCR preprocess flags) live under a collapsed Advanced section.

use super::var_ref_field;
use super::{coords_editor, detection_branch_editor, search_area_section, targets_editor};
use crate::action_tooltip::help as h;
use crate::action_tooltip::sections::{tip_advanced, tip_section, tip_wrapped_section};
use crate::paint_ctx::{CatalogPaint, VarTheme};
use crate::pickers::ActivePicker;
use crate::theme;
use crate::tree_chrome;
use crate::var_pills;
use crate::widgets::{combo_str, drag_field, text_field, W_VAR};
use eframe::egui;
use sqyre_domain::{parse_hex_color, CoordinateRef, DetectionBranch, Macro, MatchMethod};
use sqyre_hotkeys::ScreenClickBridge;

/// Name + search area — the always-visible detection header.
fn detection_primary_header(
    ui: &mut egui::Ui,
    paint: &mut CatalogPaint<'_>,
    picker: &mut ActivePicker,
    name: &mut String,
    search_area: &mut CoordinateRef,
) {
    tip_wrapped_section(ui, |ui| {
        text_field(ui, "Name", h::NAME, name);
    });
    search_area_section(ui, paint, search_area, picker);
}

/// Output coords + wait/order — shared Advanced contents for detection actions.
fn detection_advanced_fields(
    ui: &mut egui::Ui,
    theme: VarTheme<'_>,
    detection: &mut DetectionBranch,
) {
    tip_wrapped_section(ui, |ui| {
        coords_editor(ui, &mut detection.coords, theme.known_vars, theme.is_dark);
    });
    detection_branch_editor(ui, detection);
}

fn match_method_editor(ui: &mut egui::Ui, match_method: &mut MatchMethod) {
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

#[allow(clippy::too_many_arguments)]
pub(super) fn paint_image_search_fields(
    ui: &mut egui::Ui,
    paint: &mut CatalogPaint<'_>,
    picker: &mut ActivePicker,
    theme: VarTheme<'_>,
    name: &mut String,
    targets: &mut Vec<String>,
    search_area: &mut CoordinateRef,
    tolerance: &mut f64,
    blur: &mut i32,
    match_method: &mut MatchMethod,
    detection: &mut DetectionBranch,
) {
    detection_primary_header(ui, paint, picker, name, search_area);
    tip_section(ui, |ui| {
        targets_editor(ui, paint.catalog, paint.icons, targets, picker);
    });
    tip_wrapped_section(ui, |ui| {
        let tol_help = if matches!(
            *match_method,
            MatchMethod::Sqdiff | MatchMethod::SqdiffNormed
        ) {
            h::IS_TOLERANCE_SQDIFF
        } else if match_method.is_normed() {
            h::IS_TOLERANCE
        } else {
            h::IS_TOLERANCE_UNNORMED
        };
        if match_method.is_normed() {
            drag_field(ui, "Tolerance", tol_help, tolerance, |d| {
                d.speed(0.01).range(0.0..=1.0)
            });
        } else {
            drag_field(ui, "Tolerance", tol_help, tolerance, |d| d.speed(1.0));
        }
        drag_field(ui, "Blur", h::IS_BLUR, blur, |d| d);
    });
    tip_advanced(ui, |ui| {
        detection_advanced_fields(ui, theme, detection);
        tip_wrapped_section(ui, |ui| {
            match_method_editor(ui, match_method);
        });
    });
}

#[allow(clippy::too_many_arguments)]
pub(super) fn paint_ocr_fields(
    ui: &mut egui::Ui,
    paint: &mut CatalogPaint<'_>,
    picker: &mut ActivePicker,
    theme: VarTheme<'_>,
    active_macro: Option<&Macro>,
    name: &mut String,
    target: &mut String,
    search_area: &mut CoordinateRef,
    output_variable: &mut String,
    blur: &mut i32,
    min_threshold: &mut i32,
    resize: &mut f64,
    grayscale: &mut bool,
    threshold_otsu: &mut bool,
    threshold_invert: &mut bool,
    detection: &mut DetectionBranch,
) {
    detection_primary_header(ui, paint, picker, name, search_area);
    tip_wrapped_section(ui, |ui| {
        var_pills::var_name_text_edit(
            ui,
            "Output variable",
            output_variable,
            theme.known_vars,
            theme.is_dark,
            W_VAR,
            h::OCR_OUTPUT,
        );
        var_ref_field(
            ui,
            "Target",
            h::OCR_TARGET,
            target,
            theme.known_vars,
            theme.is_dark,
            W_VAR,
            active_macro,
        );
    });
    tip_wrapped_section(ui, |ui| {
        drag_field(ui, "Blur", h::OCR_BLUR, blur, |d| d);
        drag_field(
            ui,
            "Min threshold",
            h::OCR_MIN_THRESHOLD,
            min_threshold,
            |d| d,
        );
    });
    tip_advanced(ui, |ui| {
        detection_advanced_fields(ui, theme, detection);
        tip_wrapped_section(ui, |ui| {
            drag_field(ui, "Resize", h::OCR_RESIZE, resize, |d| d.speed(0.01));
            h::tip(ui.checkbox(grayscale, "Grayscale"), h::OCR_GRAYSCALE);
            h::tip(ui.checkbox(threshold_otsu, "Threshold Otsu"), h::OCR_OTSU);
            h::tip(
                ui.checkbox(threshold_invert, "Threshold invert"),
                h::OCR_INVERT,
            );
        });
    });
}

#[allow(clippy::too_many_arguments)]
pub(super) fn paint_find_pixel_fields(
    ui: &mut egui::Ui,
    paint: &mut CatalogPaint<'_>,
    picker: &mut ActivePicker,
    theme: VarTheme<'_>,
    active_macro: Option<&Macro>,
    screen_click: &ScreenClickBridge,
    name: &mut String,
    search_area: &mut CoordinateRef,
    target_color: &mut String,
    color_tolerance: &mut i32,
    detection: &mut DetectionBranch,
) {
    detection_primary_header(ui, paint, picker, name, search_area);
    tip_wrapped_section(ui, |ui| {
        ui.horizontal(|ui| {
            var_ref_field(
                ui,
                "Target color",
                h::PIXEL_COLOR,
                target_color,
                theme.known_vars,
                theme.is_dark,
                W_VAR,
                active_macro,
            );
            if let Some(rgba) = parse_hex_color(target_color) {
                let size = egui::vec2(16.0, 16.0);
                let (rect, _) = ui.allocate_exact_size(size, egui::Sense::hover());
                ui.painter().rect(
                    rect,
                    3.0,
                    tree_chrome::rgba_pub(rgba),
                    egui::Stroke::new(1.0, egui::Color32::from_gray(80)),
                    egui::StrokeKind::Outside,
                );
            }
            if theme::record_icon_button(
                ui,
                "Click on screen to sample pixel color",
                !screen_click.is_armed(),
            )
            .clicked()
            {
                screen_click.arm_color();
            }
        });
        drag_field(
            ui,
            "Color tolerance",
            h::PIXEL_TOLERANCE,
            color_tolerance,
            |d| d,
        );
    });
    tip_advanced(ui, |ui| {
        detection_advanced_fields(ui, theme, detection);
    });
}
