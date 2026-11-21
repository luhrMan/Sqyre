//! Coordinate / search-area picker rows.

use crate::action_tooltip::{help, help as h};
use crate::paint_ctx::CatalogPaint;
use crate::pickers::{self, ActivePicker, CoordKind};
use crate::preview_tooltip::{PreviewKind, PreviewTooltipCache};
use crate::theme;
use crate::var_pills::{self};
use eframe::egui;
use sqyre_domain::CoordinateRef;
use sqyre_persist::CatalogView;
use sqyre_validate::EntryValidation;

/// Kind glyph is the pick button; value sits beside it. Returns true on pick.
pub(super) fn picker_display_row(
    ui: &mut egui::Ui,
    icon: &str,
    label: &str,
    help_text: &str,
    display: &str,
) -> bool {
    let mut clicked = false;
    let tip = format!("Pick {label}…");
    crate::widgets::pair_leading(
        ui,
        |ui| {
            clicked = crate::widgets::entity_pick_button(ui, icon, &tip).clicked();
        },
        |ui| {
            help::tip(
                ui.label(if display.is_empty() {
                    sqyre_domain::EMPTY_UNSET
                } else {
                    display
                }),
                help_text,
            );
        },
    );
    clicked
}

pub(super) fn paint_coord_preview(
    ui: &mut egui::Ui,
    catalog: CatalogView<'_>,
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
        if crate::widgets::icon_button(ui, "↻", "Refresh").clicked() {
            force = true;
        }
    });
    previews.paint_for_coordinate_ref(ui, catalog, coord, kind, force, true);
}

pub(super) fn point_picker_row(
    ui: &mut egui::Ui,
    paint: &mut CatalogPaint<'_>,
    point: &mut CoordinateRef,
    picker: &mut ActivePicker,
) {
    coord_picker_row(
        ui,
        crate::model_icons::POINT,
        "Point",
        h::MOVE_POINT,
        paint,
        point,
        CoordKind::Point,
        picker,
    );
}

pub(super) fn search_area_picker_row(
    ui: &mut egui::Ui,
    paint: &mut CatalogPaint<'_>,
    area: &mut CoordinateRef,
    picker: &mut ActivePicker,
) {
    coord_picker_row(
        ui,
        crate::model_icons::SEARCH_AREA,
        "Search area",
        h::SEARCH_AREA,
        paint,
        area,
        CoordKind::SearchArea,
        picker,
    );
}

pub(super) fn cells_picker_row(
    ui: &mut egui::Ui,
    paint: &mut CatalogPaint<'_>,
    cells: &mut CoordinateRef,
    picker: &mut ActivePicker,
) {
    coord_picker_row(
        ui,
        crate::model_icons::COLLECTION,
        "Cells",
        h::FOREACH_CELLS,
        paint,
        cells,
        CoordKind::SearchArea,
        picker,
    );
}

#[expect(
    clippy::too_many_arguments,
    reason = "coord picker row: icon, label, help, paint context, and picker state"
)]
pub(super) fn coord_picker_row(
    ui: &mut egui::Ui,
    icon: &str,
    label: &str,
    help_text: &str,
    paint: &mut CatalogPaint<'_>,
    coord: &mut CoordinateRef,
    kind: CoordKind,
    picker: &mut ActivePicker,
) {
    let catalog = paint.catalog;
    let CatalogPaint {
        icons, previews, ..
    } = paint;
    let display = coord.display_label();
    let validation = coord_unset_validation(kind, coord);
    let color = if !validation.error.is_empty() {
        theme::error_fg()
    } else if !validation.warning.is_empty() {
        theme::warn_fg()
    } else {
        ui.visuals().text_color()
    };
    let pick_tip = format!("Pick {label}…");
    pickers::coord_ref_field(
        ui,
        catalog,
        icons,
        picker,
        kind,
        coord.as_str(),
        pickers::CoordRefFieldOpts {
            icon,
            pick_tip: &pick_tip,
            display,
            program: coord.program(),
            color: Some(color),
            truncate: true,
            clear_tip: None,
        },
        |ui, resp| {
            if !coord.is_empty() {
                attach_coord_hover(ui, resp, catalog, previews, coord, kind, help_text);
            } else if let Some(tip) = var_pills::entry_validation_tip(&validation) {
                resp.clone().on_hover_text(tip);
            } else {
                help::tip(resp.clone(), help_text);
            }
            var_pills::paint_entry_validation_icon(ui, &validation);
        },
    );
}

pub(super) fn coord_unset_validation(kind: CoordKind, coord: &CoordinateRef) -> EntryValidation {
    if !coord.is_empty() {
        return EntryValidation::default();
    }
    match kind {
        CoordKind::Point => EntryValidation {
            error: "set a point before saving".into(),
            warning: String::new(),
        },
        CoordKind::SearchArea => EntryValidation {
            warning: "set a search area".into(),
            error: String::new(),
        },
    }
}

pub(super) fn attach_coord_hover(
    ui: &mut egui::Ui,
    response: &egui::Response,
    catalog: CatalogView<'_>,
    previews: &mut PreviewTooltipCache,
    coord: &CoordinateRef,
    kind: CoordKind,
    help: &str,
) {
    let preview_kind = if coord.is_collection() {
        PreviewKind::Collection
    } else {
        match kind {
            CoordKind::Point => PreviewKind::Point,
            CoordKind::SearchArea => PreviewKind::SearchArea,
        }
    };
    previews.show_for_coordinate_ref(ui, response, catalog, coord, preview_kind, help);
}
