//! Brand and semantic color tokens for the app.
//!
//! Shared brand colors live in [`sqyre_ui_theme`]; this module re-exports them
//! and adds app-only preview / selection / picker fills.

use eframe::egui::{Color32, Stroke};

pub use sqyre_ui_theme::{
    accent_dim, chip_fill, contrast_fg, density, element_gap, error_fg, frame_fill, inner_stroke,
    ok_fg, overlay_panel_fill, paint_galley_centered, paint_text_centered, panel_split_stroke,
    rgba, section_gap, set_density, warn_fg, window_border_stroke, DensityGaps, MACRO_START,
    MACRO_STOP, PANEL_SPLITTER_W, PRIMARY, SPACE_12, SPACE_2, SPACE_4, SPACE_8,
    WINDOW_BORDER_RADIUS,
};

/// Dark scrim behind preview overlay chips / editors.
pub fn preview_scrim() -> Color32 {
    rgba([16, 16, 16, 170])
}

/// Veil over the main window while a blocking modal (YAML Macro Builder) is open.
pub fn modal_scrim() -> Color32 {
    rgba([0, 0, 0, 180])
}

/// Numbers, booleans and `null` in the YAML Macro Builder editor.
pub fn syntax_literal() -> Color32 {
    Color32::from_rgb(110, 160, 230)
}

/// Semi-opaque black behind labels on preview imagery.
pub fn preview_label_dim() -> Color32 {
    rgba([0, 0, 0, 150])
}

// --- Preview / PixelCheck visualization tokens (not status chrome) ---
// Analysis overlays keep distinct hues so pass/fail/within remain readable on
// heatmaps. Status text and destructive accents must use `error_fg` / `warn_fg`
// / `ok_fg` / `MACRO_*` instead of copying these RGB values.

/// Red grid / outline stroke on image previews (viz; aliases [`error_fg`]).
pub fn preview_grid_stroke() -> Color32 {
    error_fg()
}

/// Warn stroke / label on preview atlas (unresolved collections).
pub fn preview_warn_stroke() -> Color32 {
    warn_fg()
}

/// Soft blue fill for collection bounds on atlas preview.
pub fn preview_selection_fill() -> Color32 {
    rgba([60, 100, 160, 60])
}

/// Blue stroke for collection bounds on atlas preview.
pub fn preview_selection_stroke() -> Color32 {
    Color32::from_rgb(120, 180, 255)
}

/// Best / passing match marker (PixelCheck viz).
pub fn match_pass_fg() -> Color32 {
    Color32::from_rgb(80, 255, 120)
}

/// Best match below tolerance (PixelCheck viz).
pub fn match_fail_fg() -> Color32 {
    Color32::from_rgb(255, 200, 60)
}

/// Secondary match within tolerance (PixelCheck viz).
pub fn match_within_fg() -> Color32 {
    Color32::from_rgb(120, 230, 180)
}

// --- Preview viewport / monitor-map fills ---
// Dark neutrals backing rendered captures; distinct from panel chrome fills so
// imagery reads as canvas, not UI surface.

/// Backing painted under a shown capture inside an image viewport.
pub fn preview_viewport_fill() -> Color32 {
    Color32::from_gray(20)
}

/// Viewport fill when no image is available (behind the caption text).
pub fn preview_unavailable_fill() -> Color32 {
    Color32::from_gray(40)
}

/// Caption text on [`preview_unavailable_fill`].
pub fn preview_unavailable_fg() -> Color32 {
    Color32::LIGHT_GRAY
}

/// Status/error placeholder frame fill inside preview tips.
pub fn preview_status_fill() -> Color32 {
    Color32::from_gray(28)
}

/// Monitor-map desktop background.
pub fn preview_monitor_bg() -> Color32 {
    Color32::from_gray(22)
}

/// Monitor-map monitor fill.
pub fn preview_monitor_fill() -> Color32 {
    Color32::from_gray(36)
}

/// Monitor-map monitor outline.
pub fn preview_monitor_stroke() -> Stroke {
    Stroke::new(1.5, Color32::from_gray(90))
}

/// Monitor-map monitor label.
pub fn preview_monitor_label() -> Color32 {
    Color32::from_gray(200)
}

/// Label text painted over [`preview_label_dim`] chips.
pub fn preview_label_fg() -> Color32 {
    Color32::WHITE
}

/// Plain text painted inside the dark var-overlay edit field.
pub fn preview_plain_fg() -> Color32 {
    Color32::from_gray(230)
}

/// Underlay behind screen captures and capture outlines (must read over any image).
pub fn capture_underlay() -> Color32 {
    Color32::BLACK
}

/// Main-window clear color behind the egui surface.
pub fn window_clear_color() -> Color32 {
    Color32::from_rgb(0x1a, 0x1a, 0x1a)
}

/// Selected card / list stroke (Sqyre primary).
pub fn selection_stroke() -> Stroke {
    Stroke::new(2.0, PRIMARY)
}

/// Soft primary tint for related-row owner highlight.
pub fn highlight_owner_fill() -> Color32 {
    rgba([0xdc, 0x9d, 0x2e, 0x28])
}

/// Soft error tint behind invalid tree rows (from [`error_fg`]).
pub fn highlight_invalid_fill() -> Color32 {
    let c = error_fg();
    Color32::from_rgba_unmultiplied(c.r(), c.g(), c.b(), 45)
}

/// Soft green tint behind an Image Search include (`+`) tag section.
pub fn tag_include_fill() -> Color32 {
    Color32::from_rgba_unmultiplied(MACRO_START.r(), MACRO_START.g(), MACRO_START.b(), 40)
}

/// Soft red tint behind an Image Search exclude (`−`) tag section.
pub fn tag_exclude_fill() -> Color32 {
    Color32::from_rgba_unmultiplied(MACRO_STOP.r(), MACRO_STOP.g(), MACRO_STOP.b(), 40)
}

/// Soft blue fill for execution cursor row.
pub fn highlight_cursor_fill() -> Color32 {
    rgba([90, 160, 240, 70])
}

/// Soft green fill for execution progress overlay.
pub fn highlight_progress_fill() -> Color32 {
    rgba([90, 200, 130, 90])
}

/// Icon-grid selected cell fill.
pub fn picker_selected_fill() -> Color32 {
    rgba([80, 160, 100, 60])
}

/// Icon-grid selected cell stroke.
pub fn picker_selected_stroke() -> Color32 {
    Color32::from_rgb(60, 140, 80)
}

/// DnD drop-target hover stroke on icon grid.
pub fn picker_drop_stroke() -> Color32 {
    Color32::from_rgb(80, 140, 200)
}

/// Collection cell selection fill.
pub fn cell_selection_fill() -> Color32 {
    rgba([60, 160, 255, 70])
}

/// Collection cell selection stroke.
pub fn cell_selection_stroke() -> Color32 {
    Color32::from_rgb(40, 140, 255)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn primary_is_sqyre_yellow() {
        assert_eq!(PRIMARY, Color32::from_rgb(220, 157, 46));
        assert_eq!(PRIMARY.to_array(), [0xdc, 0x9d, 0x2e, 0xff]);
    }

    #[test]
    fn status_and_destructive_accents_share_semantic_helpers() {
        assert_eq!(preview_grid_stroke(), error_fg());
        assert_eq!(preview_warn_stroke(), warn_fg());
        let err = error_fg();
        assert_eq!(
            highlight_invalid_fill(),
            Color32::from_rgba_unmultiplied(err.r(), err.g(), err.b(), 45)
        );
        // PixelCheck viz tokens stay distinct from status chrome.
        assert_ne!(match_pass_fg(), ok_fg());
        assert_ne!(match_fail_fg(), warn_fg());
        assert_ne!(match_fail_fg(), error_fg());
    }
}
