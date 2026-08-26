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
use crate::widgets::{configure_match_blur_drag, drag_field, match_settings, text_field, W_VAR};
use eframe::egui;
use sqyre_domain::{
    clamp_color_tolerance, clamp_ocr_resize, clamp_ocr_threshold, parse_hex_color, CoordinateRef,
    DetectionBranch, Macro, MatchMethod, MAX_COLOR_TOLERANCE, MAX_OCR_RESIZE, MAX_OCR_THRESHOLD,
    MIN_COLOR_TOLERANCE, MIN_OCR_RESIZE, MIN_OCR_THRESHOLD,
};
use sqyre_hotkeys::ScreenClickBridge;

/// Optional live match overlay for Image Search search-area previews.
pub(super) struct ImageSearchAreaPreview<'a> {
    pub macro_: Option<&'a Macro>,
    pub targets: &'a [String],
    pub tolerance: f64,
    pub blur: i32,
    pub match_method: sqyre_domain::MatchMethod,
}

fn detection_primary_header(
    ui: &mut egui::Ui,
    paint: &mut CatalogPaint<'_>,
    picker: &mut ActivePicker,
    name: &mut String,
    search_area: &mut CoordinateRef,
    image_search: Option<ImageSearchAreaPreview<'_>>,
) {
    tip_wrapped_section(ui, |ui| {
        text_field(ui, "Name", h::NAME, name);
    });
    search_area_section(ui, paint, search_area, picker, image_search);
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
    match_settings::paint_match_method(ui, match_method);
}

#[allow(clippy::too_many_arguments)]
pub(super) fn paint_image_search_fields(
    ui: &mut egui::Ui,
    paint: &mut CatalogPaint<'_>,
    picker: &mut ActivePicker,
    theme: VarTheme<'_>,
    active_macro: Option<&Macro>,
    name: &mut String,
    targets: &mut Vec<String>,
    search_area: &mut CoordinateRef,
    tolerance: &mut f64,
    blur: &mut i32,
    match_method: &mut MatchMethod,
    detection: &mut DetectionBranch,
) {
    detection_primary_header(
        ui,
        paint,
        picker,
        name,
        search_area,
        Some(ImageSearchAreaPreview {
            macro_: active_macro,
            targets,
            tolerance: *tolerance,
            blur: *blur,
            match_method: *match_method,
        }),
    );
    tip_section(ui, |ui| {
        targets_editor(ui, paint.catalog, paint.icons, targets, picker);
    });
    tip_wrapped_section(ui, |ui| {
        match_settings::paint_match_settings(ui, tolerance, blur, match_method, false);
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
    detection_primary_header(ui, paint, picker, name, search_area, None);
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
        *blur = sqyre_domain::clamp_match_blur(*blur);
        drag_field(ui, "Blur", h::OCR_BLUR, blur, configure_match_blur_drag);
        *min_threshold = clamp_ocr_threshold(*min_threshold);
        drag_field(
            ui,
            "Min threshold",
            h::OCR_MIN_THRESHOLD,
            min_threshold,
            |d| d.speed(1).range(MIN_OCR_THRESHOLD..=MAX_OCR_THRESHOLD),
        );
    });
    tip_advanced(ui, |ui| {
        detection_advanced_fields(ui, theme, detection);
        tip_wrapped_section(ui, |ui| {
            *resize = clamp_ocr_resize(*resize);
            drag_field(ui, "Resize", h::OCR_RESIZE, resize, |d| {
                d.speed(0.01).range(MIN_OCR_RESIZE..=MAX_OCR_RESIZE)
            });
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
    detection_primary_header(ui, paint, picker, name, search_area, None);
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
        *color_tolerance = clamp_color_tolerance(*color_tolerance);
        drag_field(
            ui,
            "Color tolerance",
            h::PIXEL_TOLERANCE,
            color_tolerance,
            |d| d.speed(1).range(MIN_COLOR_TOLERANCE..=MAX_COLOR_TOLERANCE),
        );
    });
    tip_advanced(ui, |ui| {
        detection_advanced_fields(ui, theme, detection);
    });
}
