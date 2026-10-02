//! Shared [`egui::ScrollArea`] builders for panels, floaters, and pickers.
//!
//! Prefer these over ad-hoc `ScrollArea::vertical()` / `::both()` so drag-scroll,
//! viewport caps, and horizontal overflow stay consistent.
//!
//! # When to use which
//!
//! | Helper | Use |
//! |--------|-----|
//! | [`dialog_scroll`] / [`scroll_both`] | Default for lists, forms, floater bodies — both axes; H-bar when content is wider |
//! | [`scroll_vertical`] | Intentional V-only: wrapping grids, tip bodies capped with wrap, surfaces that clip rows |
//!
//! For dense single-line rows that must not wrap, call [`enable_dense_row_extend`]
//! inside a [`dialog_scroll`] body and **do not** `set_max_width(viewport)` (that
//! caps content and hides H-overflow). Grids that should reflow use
//! `horizontal_wrapped` instead. Macro tree stays V-only (egui_ltreeview width ratchet).

use egui::containers::scroll_area::{DragScroll, ScrollSource};

/// Wheel + scrollbar + click-drag. egui's default drag is touch-only (`OnTouch`).
pub const SCROLL_SOURCE: ScrollSource = ScrollSource::ALL;

/// Scroll source for areas that implement their own drag-scroll (e.g. macro tree).
pub const SCROLL_SOURCE_NO_DRAG: ScrollSource = ScrollSource {
    scroll_bar: true,
    drag: DragScroll::Never,
    mouse_wheel: true,
};

/// Vertical [`egui::ScrollArea`] with click-drag scrolling enabled.
pub fn scroll_vertical() -> egui::ScrollArea {
    egui::ScrollArea::vertical().scroll_source(SCROLL_SOURCE)
}

/// Bidirectional [`egui::ScrollArea`] with click-drag scrolling enabled.
pub fn scroll_both() -> egui::ScrollArea {
    egui::ScrollArea::both().scroll_source(SCROLL_SOURCE)
}

/// Bidirectional scroll that fills a capped viewport without expanding the parent.
///
/// Vertical-only `ScrollArea` + `auto_shrink([false, false])` expands to content
/// width and ratchets windows off-screen; enabling both axes keeps width at the
/// viewport (`(true, false) => inner_size` in egui).
pub fn dialog_scroll(max_w: f32, max_h: f32) -> egui::ScrollArea {
    scroll_both()
        .auto_shrink([false, false])
        .max_width(max_w.max(1.0))
        .max_height(max_h.max(1.0))
}

/// Prefer Extend so dense horizontal rows report their intrinsic width to a parent
/// [`dialog_scroll`] (H-bar when the pane is narrower).
pub fn enable_dense_row_extend(ui: &mut egui::Ui) {
    ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Extend);
}
