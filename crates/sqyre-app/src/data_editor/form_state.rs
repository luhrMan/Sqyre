//! FormState trait: per-tab load / dirty / valid lifecycle.

use super::helpers::{parse_i32, rgba_color};
use super::{DataEditor, EditorTab};
use sqyre_domain::{Macro, ScalarValue};
use sqyre_persist::{ProgramCatalog, UserSettings};
use sqyre_validate::{
    validate_entity_name, validate_item_grid_fields, validate_numeric_expression,
    validate_search_area_literal_bounds,
};

/// Tab form lifecycle used by [`DataEditor`] dispatch.
pub(crate) trait FormState {
    fn load(ed: &mut DataEditor, catalog: &ProgramCatalog, settings: &UserSettings);
    fn is_dirty(ed: &DataEditor, catalog: &ProgramCatalog, settings: &UserSettings) -> bool;
    fn is_valid(ed: &DataEditor, active_macro: Option<&Macro>) -> bool;
}

pub(crate) fn load_tab(
    tab: EditorTab,
    ed: &mut DataEditor,
    catalog: &ProgramCatalog,
    settings: &UserSettings,
) {
    match tab {
        EditorTab::Programs => ProgramsForm::load(ed, catalog, settings),
        EditorTab::Items => ItemsForm::load(ed, catalog, settings),
        EditorTab::Points => PointsForm::load(ed, catalog, settings),
        EditorTab::SearchAreas => SearchAreasForm::load(ed, catalog, settings),
        EditorTab::Masks => MasksForm::load(ed, catalog, settings),
        EditorTab::Collections => CollectionsForm::load(ed, catalog, settings),
        EditorTab::AutoPic => AutoPicForm::load(ed, catalog, settings),
        EditorTab::Overlay => OverlayForm::load(ed, catalog, settings),
    }
}

pub(crate) fn dirty_tab(
    tab: EditorTab,
    ed: &DataEditor,
    catalog: &ProgramCatalog,
    settings: &UserSettings,
) -> bool {
    match tab {
        EditorTab::Programs => ProgramsForm::is_dirty(ed, catalog, settings),
        EditorTab::Items => ItemsForm::is_dirty(ed, catalog, settings),
        EditorTab::Points => PointsForm::is_dirty(ed, catalog, settings),
        EditorTab::SearchAreas => SearchAreasForm::is_dirty(ed, catalog, settings),
        EditorTab::Masks => MasksForm::is_dirty(ed, catalog, settings),
        EditorTab::Collections => CollectionsForm::is_dirty(ed, catalog, settings),
        EditorTab::AutoPic => AutoPicForm::is_dirty(ed, catalog, settings),
        EditorTab::Overlay => OverlayForm::is_dirty(ed, catalog, settings),
    }
}

pub(crate) fn valid_tab(tab: EditorTab, ed: &DataEditor, active_macro: Option<&Macro>) -> bool {
    if !matches!(tab, EditorTab::Overlay) && validate_entity_name(ed.form_name.trim()).is_err() {
        return false;
    }
    match tab {
        EditorTab::Programs => ProgramsForm::is_valid(ed, active_macro),
        EditorTab::Items => ItemsForm::is_valid(ed, active_macro),
        EditorTab::Points => PointsForm::is_valid(ed, active_macro),
        EditorTab::SearchAreas => SearchAreasForm::is_valid(ed, active_macro),
        EditorTab::Masks => MasksForm::is_valid(ed, active_macro),
        EditorTab::Collections => CollectionsForm::is_valid(ed, active_macro),
        EditorTab::AutoPic => AutoPicForm::is_valid(ed, active_macro),
        EditorTab::Overlay => OverlayForm::is_valid(ed, active_macro),
    }
}

pub(crate) struct ProgramsForm;
pub(crate) struct ItemsForm;
pub(crate) struct PointsForm;
pub(crate) struct SearchAreasForm;
pub(crate) struct MasksForm;
pub(crate) struct CollectionsForm;
pub(crate) struct AutoPicForm;
pub(crate) struct OverlayForm;

impl FormState for ProgramsForm {
    fn load(ed: &mut DataEditor, catalog: &ProgramCatalog, _settings: &UserSettings) {
        ed.form_name = ed.selected_program.clone().unwrap_or_default();
        if let Some(p) = ed.selected_program.as_deref().and_then(|n| catalog.get(n)) {
            ed.form_process_path = p.process_path.clone();
            ed.form_window_title = p.window_title.clone();
        } else {
            ed.form_process_path.clear();
            ed.form_window_title.clear();
        }
    }

    fn is_dirty(ed: &DataEditor, catalog: &ProgramCatalog, _settings: &UserSettings) -> bool {
        let Some(sel) = ed.selected_program.as_deref() else {
            return !ed.form_name.trim().is_empty()
                || !ed.form_process_path.is_empty()
                || !ed.form_window_title.is_empty();
        };
        let bound = catalog.get(sel);
        let path = bound.map(|p| p.process_path.as_str()).unwrap_or("");
        let title = bound.map(|p| p.window_title.as_str()).unwrap_or("");
        ed.form_name.trim() != sel || ed.form_process_path != path || ed.form_window_title != title
    }

    fn is_valid(_ed: &DataEditor, _active_macro: Option<&Macro>) -> bool {
        true
    }
}

impl FormState for ItemsForm {
    fn load(ed: &mut DataEditor, catalog: &ProgramCatalog, _settings: &UserSettings) {
        let (prog, name) = match (
            ed.selected_program.as_deref(),
            ed.selected_entity.as_deref(),
        ) {
            (Some(p), Some(n)) => (p, n),
            _ => {
                ed.reset_item_form();
                return;
            }
        };
        let Some(item) = catalog.get(prog).and_then(|p| p.items.get(name)) else {
            ed.reset_item_form();
            return;
        };
        ed.form_name = item.name.clone();
        ed.form_cols = item.grid_cols.to_string();
        ed.form_rows = item.grid_rows.to_string();
        ed.form_stack_max = item.stack_max.to_string();
        ed.form_mask = item.mask.clone();
        ed.form_tags = item.tags.clone();
    }

    fn is_dirty(ed: &DataEditor, catalog: &ProgramCatalog, _settings: &UserSettings) -> bool {
        let (Some(prog), Some(ent)) = (
            ed.selected_program.as_deref(),
            ed.selected_entity.as_deref(),
        ) else {
            return !ed.form_name.trim().is_empty();
        };
        let Some(item) = catalog.get(prog).and_then(|p| p.items.get(ent)) else {
            return true;
        };
        ed.form_name.trim() != item.name
            || parse_i32(&ed.form_cols) != Some(item.grid_cols)
            || parse_i32(&ed.form_rows) != Some(item.grid_rows)
            || parse_i32(&ed.form_stack_max) != Some(item.stack_max)
            || ed.form_mask != item.mask
            || ed.form_tags != item.tags
    }

    fn is_valid(ed: &DataEditor, _active_macro: Option<&Macro>) -> bool {
        ed.selected_program.is_some()
            && validate_item_grid_fields(&ed.form_cols, &ed.form_rows, &ed.form_stack_max).is_ok()
    }
}

impl FormState for PointsForm {
    fn load(ed: &mut DataEditor, catalog: &ProgramCatalog, _settings: &UserSettings) {
        let (prog, name) = match (
            ed.selected_program.as_deref(),
            ed.selected_entity.as_deref(),
        ) {
            (Some(p), Some(n)) => (p, n),
            _ => return,
        };
        let res = catalog.resolution_key();
        let Some(pt) = catalog
            .get(prog)
            .and_then(|p| p.points.get(res).or_else(|| p.points.values().next()))
            .and_then(|m| m.get(name))
        else {
            return;
        };
        ed.form_name = pt.name.clone();
        ed.form_x = pt.x.as_display();
        ed.form_y = pt.y.as_display();
    }

    fn is_dirty(ed: &DataEditor, catalog: &ProgramCatalog, _settings: &UserSettings) -> bool {
        let (Some(prog), Some(ent)) = (
            ed.selected_program.as_deref(),
            ed.selected_entity.as_deref(),
        ) else {
            return !ed.form_name.trim().is_empty();
        };
        let res = catalog.resolution_key();
        let Some(pt) = catalog
            .get(prog)
            .and_then(|p| p.points.get(res).or_else(|| p.points.values().next()))
            .and_then(|m| m.get(ent))
        else {
            return true;
        };
        ed.form_name.trim() != pt.name
            || ScalarValue::parse_edit(&ed.form_x) != pt.x
            || ScalarValue::parse_edit(&ed.form_y) != pt.y
    }

    fn is_valid(ed: &DataEditor, active_macro: Option<&Macro>) -> bool {
        ed.selected_program.is_some()
            && !ed.form_x.trim().is_empty()
            && !ed.form_y.trim().is_empty()
            && !validate_numeric_expression(&ed.form_x, active_macro).blocks_submit()
            && !validate_numeric_expression(&ed.form_y, active_macro).blocks_submit()
    }
}

impl FormState for SearchAreasForm {
    fn load(ed: &mut DataEditor, catalog: &ProgramCatalog, _settings: &UserSettings) {
        let (prog, name) = match (
            ed.selected_program.as_deref(),
            ed.selected_entity.as_deref(),
        ) {
            (Some(p), Some(n)) => (p, n),
            _ => return,
        };
        let res = catalog.resolution_key();
        let Some(sa) = catalog
            .get(prog)
            .and_then(|p| {
                p.search_areas
                    .get(res)
                    .or_else(|| p.search_areas.values().next())
            })
            .and_then(|m| m.get(name))
        else {
            return;
        };
        ed.form_name = sa.name.clone();
        ed.form_left = sa.left_x.as_display();
        ed.form_top = sa.top_y.as_display();
        ed.form_right = sa.right_x.as_display();
        ed.form_bottom = sa.bottom_y.as_display();
    }

    fn is_dirty(ed: &DataEditor, catalog: &ProgramCatalog, _settings: &UserSettings) -> bool {
        let (Some(prog), Some(ent)) = (
            ed.selected_program.as_deref(),
            ed.selected_entity.as_deref(),
        ) else {
            return !ed.form_name.trim().is_empty();
        };
        let res = catalog.resolution_key();
        let Some(sa) = catalog
            .get(prog)
            .and_then(|p| {
                p.search_areas
                    .get(res)
                    .or_else(|| p.search_areas.values().next())
            })
            .and_then(|m| m.get(ent))
        else {
            return true;
        };
        ed.form_name.trim() != sa.name
            || ScalarValue::parse_edit(&ed.form_left) != sa.left_x
            || ScalarValue::parse_edit(&ed.form_top) != sa.top_y
            || ScalarValue::parse_edit(&ed.form_right) != sa.right_x
            || ScalarValue::parse_edit(&ed.form_bottom) != sa.bottom_y
    }

    fn is_valid(ed: &DataEditor, active_macro: Option<&Macro>) -> bool {
        ed.selected_program.is_some()
            && !ed.form_left.trim().is_empty()
            && !ed.form_top.trim().is_empty()
            && !ed.form_right.trim().is_empty()
            && !ed.form_bottom.trim().is_empty()
            && validate_search_area_literal_bounds(
                &ed.form_left,
                &ed.form_top,
                &ed.form_right,
                &ed.form_bottom,
            )
            .is_ok()
            && !validate_numeric_expression(&ed.form_left, active_macro).blocks_submit()
            && !validate_numeric_expression(&ed.form_top, active_macro).blocks_submit()
            && !validate_numeric_expression(&ed.form_right, active_macro).blocks_submit()
            && !validate_numeric_expression(&ed.form_bottom, active_macro).blocks_submit()
    }
}

impl FormState for MasksForm {
    fn load(ed: &mut DataEditor, catalog: &ProgramCatalog, _settings: &UserSettings) {
        let (prog, name) = match (
            ed.selected_program.as_deref(),
            ed.selected_entity.as_deref(),
        ) {
            (Some(p), Some(n)) => (p, n),
            _ => {
                ed.reset_mask_form();
                return;
            }
        };
        let Some(mask) = catalog.get(prog).and_then(|p| p.masks.get(name)) else {
            ed.reset_mask_form();
            return;
        };
        ed.form_name = mask.name.clone();
        ed.form_shape = mask.shape.as_str().to_string();
        ed.form_center_x = mask.center_x.clone();
        ed.form_center_y = mask.center_y.clone();
        ed.form_base = mask.base.clone();
        ed.form_height = mask.height.clone();
        ed.form_radius = mask.radius.clone();
        ed.form_inverse = mask.inverse;
    }

    fn is_dirty(ed: &DataEditor, catalog: &ProgramCatalog, _settings: &UserSettings) -> bool {
        let (Some(prog), Some(ent)) = (
            ed.selected_program.as_deref(),
            ed.selected_entity.as_deref(),
        ) else {
            return !ed.form_name.trim().is_empty();
        };
        let Some(mask) = catalog.get(prog).and_then(|p| p.masks.get(ent)) else {
            return true;
        };
        ed.form_name.trim() != mask.name
            || ed.form_shape != mask.shape.as_str()
            || ed.form_center_x != mask.center_x
            || ed.form_center_y != mask.center_y
            || ed.form_base != mask.base
            || ed.form_height != mask.height
            || ed.form_radius != mask.radius
            || ed.form_inverse != mask.inverse
    }

    fn is_valid(ed: &DataEditor, active_macro: Option<&Macro>) -> bool {
        if ed.selected_program.is_none()
            || (ed.form_shape != "rectangle" && ed.form_shape != "circle")
            || ed.form_center_x.trim().is_empty()
            || ed.form_center_y.trim().is_empty()
        {
            return false;
        }
        if validate_numeric_expression(&ed.form_center_x, active_macro).blocks_submit()
            || validate_numeric_expression(&ed.form_center_y, active_macro).blocks_submit()
        {
            return false;
        }
        if ed.form_shape == "circle" {
            !validate_numeric_expression(&ed.form_radius, active_macro).blocks_submit()
        } else {
            !validate_numeric_expression(&ed.form_base, active_macro).blocks_submit()
                && !validate_numeric_expression(&ed.form_height, active_macro).blocks_submit()
        }
    }
}

impl FormState for CollectionsForm {
    fn load(ed: &mut DataEditor, catalog: &ProgramCatalog, _settings: &UserSettings) {
        let (prog, name) = match (
            ed.selected_program.as_deref(),
            ed.selected_entity.as_deref(),
        ) {
            (Some(p), Some(n)) => (p, n),
            _ => {
                ed.reset_collection_form();
                return;
            }
        };
        let Some(col) = catalog.get(prog).and_then(|p| p.collections.get(name)) else {
            ed.reset_collection_form();
            return;
        };
        ed.form_name = col.name.clone();
        ed.form_search_area = col.search_area.clone();
        ed.form_rows = col.rows.to_string();
        ed.form_cols = col.cols.to_string();
        ed.collection_preview_key = Some((prog.to_string(), name.to_string()));
    }

    fn is_dirty(ed: &DataEditor, catalog: &ProgramCatalog, _settings: &UserSettings) -> bool {
        let (Some(prog), Some(ent)) = (
            ed.selected_program.as_deref(),
            ed.selected_entity.as_deref(),
        ) else {
            return !ed.form_name.trim().is_empty();
        };
        let Some(col) = catalog.get(prog).and_then(|p| p.collections.get(ent)) else {
            return true;
        };
        ed.form_name.trim() != col.name
            || ed.form_search_area != col.search_area
            || parse_i32(&ed.form_rows) != Some(col.rows)
            || parse_i32(&ed.form_cols) != Some(col.cols)
    }

    fn is_valid(ed: &DataEditor, _active_macro: Option<&Macro>) -> bool {
        ed.selected_program.is_some()
            && !ed.form_search_area.trim().is_empty()
            && parse_i32(&ed.form_rows).map(|n| n >= 1).unwrap_or(false)
            && parse_i32(&ed.form_cols).map(|n| n >= 1).unwrap_or(false)
    }
}

impl FormState for AutoPicForm {
    fn load(ed: &mut DataEditor, catalog: &ProgramCatalog, _settings: &UserSettings) {
        let (prog, name) = match (
            ed.selected_program.as_deref(),
            ed.selected_entity.as_deref(),
        ) {
            (Some(p), Some(n)) => (p, n),
            _ => return,
        };
        let res = catalog.resolution_key();
        let Some(sa) = catalog
            .get(prog)
            .and_then(|p| {
                p.search_areas
                    .get(res)
                    .or_else(|| p.search_areas.values().next())
            })
            .and_then(|m| m.get(name))
        else {
            return;
        };
        ed.form_name = sa.name.clone();
        ed.form_left = sa.left_x.as_display();
        ed.form_top = sa.top_y.as_display();
        ed.form_right = sa.right_x.as_display();
        ed.form_bottom = sa.bottom_y.as_display();
    }

    fn is_dirty(_ed: &DataEditor, _catalog: &ProgramCatalog, _settings: &UserSettings) -> bool {
        false
    }

    fn is_valid(_ed: &DataEditor, _active_macro: Option<&Macro>) -> bool {
        false
    }
}

impl FormState for OverlayForm {
    fn load(ed: &mut DataEditor, _catalog: &ProgramCatalog, settings: &UserSettings) {
        ed.load_overlay_form(settings);
    }

    fn is_dirty(ed: &DataEditor, _catalog: &ProgramCatalog, settings: &UserSettings) -> bool {
        let Some(id) = ed.selected_entity.as_deref() else {
            return false;
        };
        let Some(btn) = settings.overlay_buttons.iter().find(|b| b.id == id) else {
            return true;
        };
        ed.form_name.trim() != btn.label.trim()
            || ed.form_overlay_macro.trim() != btn.macro_name.trim()
            || ed.form_overlay_icon != btn.icon
            || (ed.form_overlay_x - btn.x).abs() > f32::EPSILON
            || (ed.form_overlay_y - btn.y).abs() > f32::EPSILON
            || (ed.form_overlay_size - btn.size).abs() > f32::EPSILON
            || (ed.form_overlay_corner_radius - btn.corner_radius).abs() > f32::EPSILON
            || (ed.form_overlay_border_width - btn.border_width).abs() > f32::EPSILON
            || ed.form_overlay_border != rgba_color(btn.border_rgba())
            || ed.form_overlay_bg != rgba_color(btn.bg_rgba())
            || ed.form_overlay_icon_color != rgba_color(btn.icon_rgba())
            || ed.form_overlay_icon_hover != rgba_color(btn.icon_hover_rgba())
            || ed.selected_program.as_deref() != Some(btn.program.as_str())
    }

    fn is_valid(ed: &DataEditor, _active_macro: Option<&Macro>) -> bool {
        ed.selected_program.is_some()
            && ed.selected_entity.is_some()
            && !ed.form_overlay_macro.trim().is_empty()
    }
}
