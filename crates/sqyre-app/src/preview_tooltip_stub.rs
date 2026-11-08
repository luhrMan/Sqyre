//! Browser editor: coordinate previews without live screen capture.

use crate::image_view::ImageViewTransform;
use eframe::egui;
use sqyre_domain::{Action, ActionKind, CoordinateRef};
use sqyre_persist::ProgramCatalog;

const UNAVAILABLE: &str = "Live screen preview requires the desktop app.";

/// Display footprint of a coordinate preview; mirrors the native constant used
/// by the view-tip preferred-width measurement.
pub(crate) const DISPLAY_MAX_W: f32 = 260.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PreviewKind {
    Point,
    SearchArea,
    Collection,
}

#[derive(Default)]
pub struct PreviewTooltipCache;

impl PreviewTooltipCache {
    pub fn new() -> Self {
        Self
    }

    pub fn invalidate_entity(&mut self, _name: &str) {}

    pub fn show_for_entity(
        &mut self,
        _ui: &mut egui::Ui,
        response: &egui::Response,
        _catalog: &ProgramCatalog,
        program: &str,
        name: &str,
        _kind: PreviewKind,
    ) {
        if response.hovered() {
            response
                .clone()
                .on_hover_text(format!("{program}~{name}\n{UNAVAILABLE}"));
        }
    }

    /// `interactive` mirrors the native signature: hover-capable surfaces get
    /// the icon + tooltip; the non-interactive view tip keeps visible text.
    pub fn paint_for_coordinate_ref(
        &mut self,
        ui: &mut egui::Ui,
        _catalog: &ProgramCatalog,
        _coord_ref: &CoordinateRef,
        _kind: PreviewKind,
        _force: bool,
        interactive: bool,
    ) {
        if interactive {
            crate::widgets::error_icon(ui, UNAVAILABLE);
        } else {
            ui.colored_label(crate::theme::error_fg(), UNAVAILABLE);
        }
    }

    pub fn show_for_coordinate_ref(
        &mut self,
        _ui: &mut egui::Ui,
        response: &egui::Response,
        _catalog: &ProgramCatalog,
        coord_ref: &CoordinateRef,
        _kind: PreviewKind,
        help: &str,
    ) {
        if response.hovered() {
            let mut text = format!("{}\n{UNAVAILABLE}", coord_ref.as_str());
            if !help.is_empty() {
                text.push_str(&format!("\n{help}"));
            }
            response.clone().on_hover_text(text);
        }
    }

    /// Browser editor has no live desktop to outline on.
    pub fn desktop_outlines_supported(&self) -> bool {
        false
    }

    /// Mirrors the native signature; there is no desktop to outline on.
    #[expect(
        dead_code,
        reason = "the only caller is the native runtime overlay sync"
    )]
    pub fn set_desktop_outlines_enabled(&mut self, _enabled: bool) {}

    pub fn request_outline_abs(&mut self, _rect: (i32, i32, i32, i32)) {}

    pub fn request_outline_abs_hover(&mut self, _rect: (i32, i32, i32, i32)) {}

    pub fn paint_point_panel(
        &mut self,
        ui: &mut egui::Ui,
        _x: Option<i32>,
        _y: Option<i32>,
        _force: bool,
        view: &mut ImageViewTransform,
    ) -> egui::Rect {
        paint_unavailable_panel(ui, view)
    }

    #[expect(
        clippy::too_many_arguments,
        reason = "mirrors the native search-area panel signature"
    )]
    pub fn paint_search_area_panel(
        &mut self,
        ui: &mut egui::Ui,
        _left: Option<i32>,
        _top: Option<i32>,
        _right: Option<i32>,
        _bottom: Option<i32>,
        _force: bool,
        view: &mut ImageViewTransform,
    ) -> (egui::Rect, egui::Vec2) {
        (paint_unavailable_panel(ui, view), egui::Vec2::ZERO)
    }
}

fn paint_unavailable_panel(ui: &mut egui::Ui, _view: &mut ImageViewTransform) -> egui::Rect {
    let desired = crate::data_editor_preview::fit_panel(ui.available_width(), 220.0);
    let (rect, _) = ui.allocate_exact_size(desired, egui::Sense::hover());
    ui.painter()
        .rect_filled(rect, 4.0, crate::theme::preview_status_fill());
    crate::data_editor_preview::attach_preview_error_icon(ui, rect, UNAVAILABLE);
    crate::data_editor_preview::paint_preview_frame(ui.painter(), rect);
    rect
}

pub fn coordinate_ref_for_preview(action: &Action) -> Option<(CoordinateRef, PreviewKind)> {
    match &action.kind {
        ActionKind::Move { point, .. } if !point.is_empty() => {
            Some((point.clone(), PreviewKind::Point))
        }
        ActionKind::ImageSearch(d) if !d.search_area.is_empty() => {
            Some((d.search_area.clone(), PreviewKind::SearchArea))
        }
        ActionKind::Ocr(d) if !d.search_area.is_empty() => {
            Some((d.search_area.clone(), PreviewKind::SearchArea))
        }
        ActionKind::FindPixel(d) if !d.search_area.is_empty() => {
            Some((d.search_area.clone(), PreviewKind::SearchArea))
        }
        _ => None,
    }
}
