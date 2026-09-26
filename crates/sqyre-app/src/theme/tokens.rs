//! Brand and semantic color tokens for the app.
//!
//! Shared brand colors live in [`sqyre_ui_theme`]; this module re-exports them
//! and adds app-only preview / selection / picker fills.

use eframe::egui::{Color32, Stroke};

pub use sqyre_ui_theme::{
    accent_dim, chip_fill, contrast_fg, error_fg, frame_fill, inner_stroke, ok_fg,
    overlay_panel_fill, paint_galley_centered, paint_text_centered, rgba, warn_fg, MACRO_START,
    MACRO_STOP, PRIMARY, SPACE_12, SPACE_2, SPACE_4, SPACE_8,
};

/// Dark scrim behind preview overlay chips / editors.
pub fn preview_scrim() -> Color32 {
    rgba([16, 16, 16, 170])
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

/// Remove-badge hover fill on icon grid (destructive [`MACRO_STOP`]).
pub fn picker_remove_hover() -> Color32 {
    MACRO_STOP
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
        assert_eq!(picker_remove_hover(), MACRO_STOP);
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
