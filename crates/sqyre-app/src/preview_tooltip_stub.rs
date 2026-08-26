//! Browser editor: coordinate previews without live screen capture.

use crate::image_view::ImageViewTransform;
use eframe::egui;
use sqyre_domain::{Action, ActionKind, CoordinateRef};
use sqyre_persist::ProgramCatalog;

const UNAVAILABLE: &str = "Live screen preview requires the desktop app.";

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
        Self::default()
    }

    pub fn invalidate_entity(&mut self, _name: &str) {}

    pub fn clear(&mut self) {}

    pub fn take_desktop_outline(&mut self) -> Option<(i32, i32, i32, i32)> {
        None
    }

    pub fn show_for_entity(
        &mut self,
        ui: &mut egui::Ui,
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

    pub fn paint_for_coordinate_ref(
        &mut self,
        ui: &mut egui::Ui,
        _catalog: &ProgramCatalog,
        _coord_ref: &CoordinateRef,
        _kind: PreviewKind,
        _force: bool,
    ) {
        ui.colored_label(crate::theme::error_fg(), UNAVAILABLE);
    }

    pub fn show_for_coordinate_ref(
        &mut self,
        ui: &mut egui::Ui,
        response: &egui::Response,
        _catalog: &ProgramCatalog,
        coord_ref: &CoordinateRef,
        _kind: PreviewKind,
    ) {
        if response.hovered() {
            response
                .clone()
                .on_hover_text(format!("{}\n{UNAVAILABLE}", coord_ref.as_str()));
        }
    }

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
        .rect_filled(rect, 4.0, egui::Color32::from_gray(28));
    ui.painter().text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        UNAVAILABLE,
        egui::TextStyle::Small.resolve(ui.style()),
        crate::theme::error_fg(),
    );
    crate::data_editor_preview::paint_preview_frame(ui.painter(), rect);
    rect
}

pub fn coordinate_ref_for_preview(action: &Action) -> Option<(CoordinateRef, PreviewKind)> {
    match &action.kind {
        ActionKind::Move { point, .. } if !point.is_empty() => {
            Some((point.clone(), PreviewKind::Point))
        }
        ActionKind::ImageSearch { search_area, .. }
        | ActionKind::Ocr { search_area, .. }
        | ActionKind::FindPixel { search_area, .. }
            if !search_area.is_empty() =>
        {
            Some((search_area.clone(), PreviewKind::SearchArea))
        }
        _ => None,
    }
}
