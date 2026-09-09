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
    match_settings::paint_match_method(ui, match_method);
}

/// Borrowed [`ActionKind::ImageSearch`] fields, so the painter takes one
/// argument for the action instead of one per field.
///
/// The caller still matches the variant, which keeps the field set statically
/// checked; this only bundles the resulting `&mut` bindings.
///
/// [`ActionKind::ImageSearch`]: sqyre_domain::ActionKind::ImageSearch
pub(super) struct ImageSearchFields<'a> {
    pub name: &'a mut String,
    pub targets: &'a mut Vec<String>,
    pub search_area: &'a mut CoordinateRef,
    pub tolerance: &'a mut f64,
    pub blur: &'a mut i32,
    pub match_method: &'a mut MatchMethod,
    pub detection: &'a mut DetectionBranch,
}

/// Borrowed [`ActionKind::Ocr`] fields. See [`ImageSearchFields`].
///
/// [`ActionKind::Ocr`]: sqyre_domain::ActionKind::Ocr
pub(super) struct OcrFields<'a> {
    pub name: &'a mut String,
    pub target: &'a mut String,
    pub search_area: &'a mut CoordinateRef,
    pub output_variable: &'a mut String,
    pub blur: &'a mut i32,
    pub min_threshold: &'a mut i32,
    pub resize: &'a mut f64,
    pub grayscale: &'a mut bool,
    pub threshold_otsu: &'a mut bool,
    pub threshold_invert: &'a mut bool,
    pub detection: &'a mut DetectionBranch,
}

/// Borrowed [`ActionKind::FindPixel`] fields. See [`ImageSearchFields`].
///
/// [`ActionKind::FindPixel`]: sqyre_domain::ActionKind::FindPixel
pub(super) struct FindPixelFields<'a> {
    pub name: &'a mut String,
    pub search_area: &'a mut CoordinateRef,
    pub target_color: &'a mut String,
    pub color_tolerance: &'a mut i32,
    pub detection: &'a mut DetectionBranch,
}

pub(super) fn paint_image_search_fields(
    ui: &mut egui::Ui,
    paint: &mut CatalogPaint<'_>,
    picker: &mut ActivePicker,
    theme: VarTheme<'_>,
    fields: ImageSearchFields<'_>,
) {
    let ImageSearchFields {
        name,
        targets,
        search_area,
        tolerance,
        blur,
        match_method,
        detection,
    } = fields;
    detection_primary_header(ui, paint, picker, name, search_area);
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

pub(super) fn paint_ocr_fields(
    ui: &mut egui::Ui,
    paint: &mut CatalogPaint<'_>,
    picker: &mut ActivePicker,
    theme: VarTheme<'_>,
    active_macro: Option<&Macro>,
    fields: OcrFields<'_>,
) {
    let OcrFields {
        name,
        target,
        search_area,
        output_variable,
        blur,
        min_threshold,
        resize,
        grayscale,
        threshold_otsu,
        threshold_invert,
        detection,
    } = fields;
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
            target,
            theme,
            h::OCR_TARGET,
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

pub(super) fn paint_find_pixel_fields(
    ui: &mut egui::Ui,
    paint: &mut CatalogPaint<'_>,
    picker: &mut ActivePicker,
    theme: VarTheme<'_>,
    active_macro: Option<&Macro>,
    screen_click: &ScreenClickBridge,
    fields: FindPixelFields<'_>,
) {
    let FindPixelFields {
        name,
        search_area,
        target_color,
        color_tolerance,
        detection,
    } = fields;
    detection_primary_header(ui, paint, picker, name, search_area);
    tip_wrapped_section(ui, |ui| {
        ui.horizontal(|ui| {
            var_ref_field(
                ui,
                "Target color",
                target_color,
                theme,
                h::PIXEL_COLOR,
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
