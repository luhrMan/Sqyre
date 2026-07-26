use super::sections::tip_section;
use crate::data_editor_preview::show_file_hover;
use crate::icon_cache::IconCache;
use crate::paint_ctx::CatalogPaint;
use crate::pickers::{self, ActivePicker, CoordKind};
use crate::preview_tooltip::{PreviewKind, PreviewTooltipCache};
use eframe::egui;
use sqyre_domain::CoordinateRef;
use sqyre_persist::ProgramCatalog;

fn pick_icon_btn(ui: &mut egui::Ui) -> egui::Response {
    crate::theme::icon_button(ui, "☰").on_hover_text("Pick…")
}

/// Label + read-only value + pick button. Returns true when pick was clicked.
fn picker_display_row(ui: &mut egui::Ui, label: &str, help_text: &str, display: &str) -> bool {
    let mut clicked = false;
    ui.horizontal(|ui| {
        help::label(ui, label, help_text);
        ui.label(if display.is_empty() {
            "(unset)"
        } else {
            display
        });
        if pick_icon_btn(ui).clicked() {
            clicked = true;
        }
    });
    clicked
}

fn paint_coord_preview(
    ui: &mut egui::Ui,
    catalog: &ProgramCatalog,
    previews: &mut PreviewTooltipCache,
    coord: &CoordinateRef,
    kind: PreviewKind,
) {
    if coord.is_empty() {
        return;
    }
    let mut force = false;
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new("Preview").strong());
        if crate::theme::icon_button(ui, "↻")
            .on_hover_text("Refresh")
            .clicked()
        {
            force = true;
        }
    });
    previews.paint_for_coordinate_ref(ui, catalog, coord, kind, force);
}

fn point_picker_row(
    ui: &mut egui::Ui,
    paint: &mut CatalogPaint<'_>,
    point: &mut CoordinateRef,
    picker: &mut ActivePicker,
) {
    coord_picker_row(
        ui,
        "Point",
        h::MOVE_POINT,
        paint,
        point,
        CoordKind::Point,
        picker,
    );
}

fn search_area_picker_row(
    ui: &mut egui::Ui,
    paint: &mut CatalogPaint<'_>,
    area: &mut CoordinateRef,
    picker: &mut ActivePicker,
) {
    coord_picker_row(
        ui,
        "Search area",
        h::SEARCH_AREA,
        paint,
        area,
        CoordKind::SearchArea,
        picker,
    );
}

fn coord_picker_row(
    ui: &mut egui::Ui,
    label: &str,
    help_text: &str,
    paint: &mut CatalogPaint<'_>,
    coord: &mut CoordinateRef,
    kind: CoordKind,
    picker: &mut ActivePicker,
) {
    let CatalogPaint {
        catalog,
        icons,
        previews,
    } = paint;
    let display = if coord.is_empty() {
        "(unset)"
    } else {
        coord.as_str()
    };
    ui.horizontal(|ui| {
        help::label(ui, label, help_text);
        if let Some(prog) = coord.program() {
            crate::icon_cache::paint_program_icon(ui, catalog, icons, prog);
        }
        let resp = ui.monospace(display);
        if !coord.is_empty() {
            attach_coord_hover(ui, &resp, catalog, icons, previews, coord, kind);
        }
        if pick_icon_btn(ui).clicked() {
            *picker = ActivePicker::Coord {
                kind,
                search: String::new(),
                value: coord.0.clone(),
                cell_pick: None,
                scroll_to_selection: true,
            };
        }
    });
}

fn attach_coord_hover(
    ui: &mut egui::Ui,
    response: &egui::Response,
    catalog: &ProgramCatalog,
    icons: &mut IconCache,
    previews: &mut PreviewTooltipCache,
    coord: &CoordinateRef,
    kind: CoordKind,
) {
    if coord.is_collection() {
        let Some(prog) = coord.program() else {
            return;
        };
        show_file_hover(
            ui,
            response,
            icons,
            &catalog.collection_image_path(prog, coord.name()),
            coord.as_str(),
        );
        return;
    }
    let preview_kind = match kind {
        CoordKind::Point => PreviewKind::Point,
        CoordKind::SearchArea => PreviewKind::SearchArea,
    };
    previews.show_for_coordinate_ref(ui, response, catalog, coord, preview_kind);
}
