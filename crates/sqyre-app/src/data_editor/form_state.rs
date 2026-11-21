//! FormState trait: per-tab load / dirty / valid lifecycle.

use super::helpers::parse_i32;
#[cfg(feature = "native-runtime")]
use super::match_check;
use super::{DataEditor, EditorTab};
use sqyre_domain::{Macro, ScalarValue};
use sqyre_persist::{CatalogView, ProgramCatalog, UserSettings};
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
        EditorTab::Atlases => AtlasesForm::load(ed, catalog, settings),
        EditorTab::ScreenCap => ScreenCapForm::load(ed, catalog, settings),
        EditorTab::MatchCheck => MatchCheckForm::load(ed, catalog, settings),
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
        EditorTab::Atlases => AtlasesForm::is_dirty(ed, catalog, settings),
        EditorTab::ScreenCap => ScreenCapForm::is_dirty(ed, catalog, settings),
        EditorTab::MatchCheck => MatchCheckForm::is_dirty(ed, catalog, settings),
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
        EditorTab::Atlases => AtlasesForm::is_valid(ed, active_macro),
        EditorTab::ScreenCap => ScreenCapForm::is_valid(ed, active_macro),
        EditorTab::MatchCheck => MatchCheckForm::is_valid(ed, active_macro),
        EditorTab::Overlay => OverlayForm::is_valid(ed, active_macro),
    }
}

pub(crate) struct ProgramsForm;
pub(crate) struct ItemsForm;
pub(crate) struct PointsForm;
pub(crate) struct SearchAreasForm;
pub(crate) struct MasksForm;
pub(crate) struct CollectionsForm;
pub(crate) struct AtlasesForm;
pub(crate) struct ScreenCapForm;
pub(crate) struct MatchCheckForm;
pub(crate) struct OverlayForm;

impl FormState for ProgramsForm {
    fn load(ed: &mut DataEditor, catalog: &ProgramCatalog, _settings: &UserSettings) {
        ed.form_name = ed.selected_program.clone().unwrap_or_default();
        if let Some(p) = ed.selected_program.as_deref().and_then(|n| catalog.get(n)) {
            ed.form_program.process_path = p.process_path.clone();
            ed.form_program.window_title = p.window_title.clone();
            ed.form_tags = p.tags.clone();
            let name = ed.form_name.trim().to_string();
            ed.form_tags.retain(|tag| tag.trim() != name);
        } else {
            ed.form_program.process_path.clear();
            ed.form_program.window_title.clear();
            ed.form_tags.clear();
        }
        ed.tag_draft.clear();
    }

    fn is_dirty(ed: &DataEditor, catalog: &ProgramCatalog, _settings: &UserSettings) -> bool {
        let Some(sel) = ed.selected_program.as_deref() else {
            return !ed.form_name.trim().is_empty()
                || !ed.form_program.process_path.is_empty()
                || !ed.form_program.window_title.is_empty()
                || !ed.form_tags.is_empty();
        };
        let bound = catalog.get(sel);
        let path = bound.map(|p| p.process_path.as_str()).unwrap_or("");
        let title = bound.map(|p| p.window_title.as_str()).unwrap_or("");
        let tags = bound.map(|p| p.tags.as_slice()).unwrap_or(&[]);
        ed.form_name.trim() != sel
            || ed.form_program.process_path != path
            || ed.form_program.window_title != title
            || ed.form_tags != tags
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
        ed.form_item.stack_max = item.stack_max.to_string();
        ed.form_item.mask = item.mask.clone();
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
            || parse_i32(&ed.form_item.stack_max) != Some(item.stack_max)
            || ed.form_item.mask != item.mask
            || ed.form_tags != item.tags
    }

    fn is_valid(ed: &DataEditor, _active_macro: Option<&Macro>) -> bool {
        ed.selected_program.is_some()
            && validate_item_grid_fields(&ed.form_cols, &ed.form_rows, &ed.form_item.stack_max)
                .is_ok()
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
        ed.form_monitor = pt.monitor.max(1);
        ed.form_point.x = pt.x.as_display();
        ed.form_point.y = pt.y.as_display();
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
            || ed.form_monitor.max(1) != pt.monitor.max(1)
            || ScalarValue::parse_edit(&ed.form_point.x) != pt.x
            || ScalarValue::parse_edit(&ed.form_point.y) != pt.y
    }

    fn is_valid(ed: &DataEditor, active_macro: Option<&Macro>) -> bool {
        ed.selected_program.is_some()
            && !ed.form_point.x.trim().is_empty()
            && !ed.form_point.y.trim().is_empty()
            && !validate_numeric_expression(&ed.form_point.x, active_macro).blocks_submit()
            && !validate_numeric_expression(&ed.form_point.y, active_macro).blocks_submit()
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
        ed.form_monitor = sa.monitor.max(1);
        ed.form_rect.load_displays(
            sa.left_x.as_display(),
            sa.top_y.as_display(),
            sa.right_x.as_display(),
            sa.bottom_y.as_display(),
        );
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
            || ed.form_monitor.max(1) != sa.monitor.max(1)
            || ScalarValue::parse_edit(&ed.form_rect.left) != sa.left_x
            || ScalarValue::parse_edit(&ed.form_rect.top) != sa.top_y
            || ScalarValue::parse_edit(&ed.form_rect.right) != sa.right_x
            || ScalarValue::parse_edit(&ed.form_rect.bottom) != sa.bottom_y
    }

    fn is_valid(ed: &DataEditor, active_macro: Option<&Macro>) -> bool {
        ed.selected_program.is_some()
            && !ed.form_rect.left.trim().is_empty()
            && !ed.form_rect.top.trim().is_empty()
            && !ed.form_rect.right.trim().is_empty()
            && !ed.form_rect.bottom.trim().is_empty()
            && validate_search_area_literal_bounds(
                &ed.form_rect.left,
                &ed.form_rect.top,
                &ed.form_rect.right,
                &ed.form_rect.bottom,
            )
            .is_ok()
            && !validate_numeric_expression(&ed.form_rect.left, active_macro).blocks_submit()
            && !validate_numeric_expression(&ed.form_rect.top, active_macro).blocks_submit()
            && !validate_numeric_expression(&ed.form_rect.right, active_macro).blocks_submit()
            && !validate_numeric_expression(&ed.form_rect.bottom, active_macro).blocks_submit()
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
        ed.form_mask.shape = mask.shape.as_str().to_string();
        ed.form_mask.center_x = mask.center_x.clone();
        ed.form_mask.center_y = mask.center_y.clone();
        ed.form_mask.base = mask.base.clone();
        ed.form_mask.height = mask.height.clone();
        ed.form_mask.radius = mask.radius.clone();
        ed.form_mask.inverse = mask.inverse;
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
            || ed.form_mask.shape != mask.shape.as_str()
            || ed.form_mask.center_x != mask.center_x
            || ed.form_mask.center_y != mask.center_y
            || ed.form_mask.base != mask.base
            || ed.form_mask.height != mask.height
            || ed.form_mask.radius != mask.radius
            || ed.form_mask.inverse != mask.inverse
    }

    fn is_valid(ed: &DataEditor, active_macro: Option<&Macro>) -> bool {
        if ed.selected_program.is_none()
            || (ed.form_mask.shape != "rectangle" && ed.form_mask.shape != "circle")
            || ed.form_mask.center_x.trim().is_empty()
            || ed.form_mask.center_y.trim().is_empty()
        {
            return false;
        }
        if validate_numeric_expression(&ed.form_mask.center_x, active_macro).blocks_submit()
            || validate_numeric_expression(&ed.form_mask.center_y, active_macro).blocks_submit()
        {
            return false;
        }
        if ed.form_mask.shape == "circle" {
            !validate_numeric_expression(&ed.form_mask.radius, active_macro).blocks_submit()
        } else {
            !validate_numeric_expression(&ed.form_mask.base, active_macro).blocks_submit()
                && !validate_numeric_expression(&ed.form_mask.height, active_macro).blocks_submit()
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
        ed.form_search_area = col.name.clone();
        ed.form_rows = col.rows.to_string();
        ed.form_cols = col.cols.to_string();
        ed.previews.collection.key = Some((prog.to_string(), name.to_string()));
    }

    fn is_dirty(ed: &DataEditor, catalog: &ProgramCatalog, _settings: &UserSettings) -> bool {
        let (Some(prog), Some(ent)) = (
            ed.selected_program.as_deref(),
            ed.selected_entity.as_deref(),
        ) else {
            return !ed.form_search_area.trim().is_empty()
                || ed.form_rows != "1"
                || ed.form_cols != "1";
        };
        let Some(col) = catalog.get(prog).and_then(|p| p.collections.get(ent)) else {
            return true;
        };
        ed.form_search_area.trim() != col.name
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

impl FormState for AtlasesForm {
    fn load(ed: &mut DataEditor, catalog: &ProgramCatalog, _settings: &UserSettings) {
        let (prog, name) = match (
            ed.selected_program.as_deref(),
            ed.selected_entity.as_deref(),
        ) {
            (Some(p), Some(n)) => (p, n),
            _ => {
                ed.reset_atlas_form();
                return;
            }
        };
        let Some(atlas) = catalog.get(prog).and_then(|p| p.atlases.get(name)) else {
            ed.reset_atlas_form();
            return;
        };
        ed.form_name = atlas.name.clone();
        ed.form_atlas.members = atlas.collections.clone();
        ed.form_atlas.add.clear();
        ed.previews.atlas.key = Some((prog.to_string(), name.to_string()));
    }

    fn is_dirty(ed: &DataEditor, catalog: &ProgramCatalog, _settings: &UserSettings) -> bool {
        let (Some(prog), Some(ent)) = (
            ed.selected_program.as_deref(),
            ed.selected_entity.as_deref(),
        ) else {
            return !ed.form_name.trim().is_empty() || !ed.form_atlas.members.is_empty();
        };
        let Some(atlas) = catalog.get(prog).and_then(|p| p.atlases.get(ent)) else {
            return true;
        };
        ed.form_name.trim() != atlas.name || ed.form_atlas.members != atlas.collections
    }

    fn is_valid(ed: &DataEditor, _active_macro: Option<&Macro>) -> bool {
        ed.selected_program.is_some() && !ed.form_atlas.members.is_empty()
    }
}

impl FormState for ScreenCapForm {
    fn load(ed: &mut DataEditor, catalog: &ProgramCatalog, _settings: &UserSettings) {
        let (prog, name) = match (ed.selected_program.clone(), ed.selected_entity.clone()) {
            (Some(p), Some(n)) => (p, n),
            _ => return,
        };
        // Collection rows open a cell picker; skip until a cell range is chosen.
        let is_collection_only = catalog.get(&prog).is_some_and(|p| {
            let has_col = p.collections.contains_key(&name);
            let has_sa = p
                .search_areas
                .get(catalog.resolution_key())
                .or_else(|| p.search_areas.values().next())
                .is_some_and(|m| m.contains_key(&name));
            has_col && !has_sa
        });
        if is_collection_only {
            return;
        }
        let coord =
            sqyre_domain::CoordinateRef(format!("{prog}{}{name}", sqyre_domain::PROGRAM_DELIMITER));
        if ed.apply_screen_cap_reference(CatalogView::from(catalog), coord) {
            return;
        }
        let res = catalog.resolution_key();
        let Some(sa) = catalog
            .get(&prog)
            .and_then(|p| {
                p.search_areas
                    .get(res)
                    .or_else(|| p.search_areas.values().next())
            })
            .and_then(|m| m.get(&name))
        else {
            return;
        };
        ed.form_name = sa.name.clone();
        ed.form_search_area = format!("{prog}{}{name}", sqyre_domain::PROGRAM_DELIMITER);
        ed.form_monitor = sa.monitor.max(1);
        ed.form_rect.load_displays(
            sa.left_x.as_display(),
            sa.top_y.as_display(),
            sa.right_x.as_display(),
            sa.bottom_y.as_display(),
        );
    }

    fn is_dirty(_ed: &DataEditor, _catalog: &ProgramCatalog, _settings: &UserSettings) -> bool {
        false
    }

    fn is_valid(_ed: &DataEditor, _active_macro: Option<&Macro>) -> bool {
        false
    }
}

impl FormState for MatchCheckForm {
    fn load(ed: &mut DataEditor, catalog: &ProgramCatalog, _settings: &UserSettings) {
        let (prog, name) = match (ed.selected_program.clone(), ed.selected_entity.clone()) {
            (Some(p), Some(n)) => (p, n),
            _ => {
                ed.form_name.clear();
                return;
            }
        };
        let Some(item) = catalog.get(&prog).and_then(|p| p.items.get(&name)) else {
            ed.form_name.clear();
            return;
        };
        ed.form_name = item.name.clone();
        #[cfg(feature = "native-runtime")]
        {
            ed.invalidate_match_check();
            ed.match_check.paused = false;
            ed.match_check.last_live_at = None;
            ed.match_check.capture_once = false;
            ed.match_check.awaiting_panel = false;
            ed.match_check.rematch_once = false;
            let variant = match_check::default_variant(CatalogView::from(catalog), &prog, &name);
            ed.match_check.variant = variant;
        }
    }

    fn is_dirty(_ed: &DataEditor, _catalog: &ProgramCatalog, _settings: &UserSettings) -> bool {
        false
    }

    fn is_valid(ed: &DataEditor, active_macro: Option<&Macro>) -> bool {
        #[cfg(feature = "native-runtime")]
        {
            if ed.selected_program.is_none() || ed.selected_entity.is_none() {
                return false;
            }
            validate_search_area_literal_bounds(
                &ed.form_rect.left,
                &ed.form_rect.top,
                &ed.form_rect.right,
                &ed.form_rect.bottom,
            )
            .is_ok()
                && !validate_numeric_expression(&ed.form_rect.left, active_macro).blocks_submit()
                && !validate_numeric_expression(&ed.form_rect.top, active_macro).blocks_submit()
                && !validate_numeric_expression(&ed.form_rect.right, active_macro).blocks_submit()
                && !validate_numeric_expression(&ed.form_rect.bottom, active_macro).blocks_submit()
        }
        #[cfg(not(feature = "native-runtime"))]
        {
            let _ = (ed, active_macro);
            false
        }
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
        let Some(btn) = settings.overlay.overlay_buttons.iter().find(|b| b.id == id) else {
            return true;
        };
        ed.form_overlay.placement_is_dirty_vs(btn)
            || ed.form_overlay.style_is_dirty_vs(btn)
            || ed.selected_program.as_deref() != Some(btn.program.as_str())
            || ed.form_overlay.gate.is_dirty_vs(&btn.visibility_gate)
    }

    fn is_valid(ed: &DataEditor, _active_macro: Option<&Macro>) -> bool {
        ed.selected_program.is_some()
            && ed.selected_entity.is_some()
            && !ed.form_overlay.macro_name.trim().is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqyre_persist::{ProgramItem, ProgramMask, ProgramPoint, ProgramSearchArea};

    fn catalog() -> ProgramCatalog {
        let mut cat = ProgramCatalog::default();
        cat.set_resolution_key("1920x1080");
        cat.create_program("Game").unwrap();
        cat
    }

    fn editor_on(tab: EditorTab) -> DataEditor {
        DataEditor {
            tab,
            ..Default::default()
        }
    }

    #[test]
    fn valid_tab_rejects_bad_entity_names_everywhere() {
        let mut ed = editor_on(EditorTab::Items);
        ed.selected_program = Some("Game".into());
        ed.form_name = "a/b".into();
        ed.form_cols = "1".into();
        ed.form_rows = "1".into();
        ed.form_item.stack_max = "0".into();
        assert!(!valid_tab(EditorTab::Items, &ed, None));
    }

    #[test]
    fn programs_form_load_and_dirty() {
        let mut cat = catalog();
        let settings = UserSettings::default();
        cat.set_process_binding("Game", "/bin/game", "Game Window")
            .unwrap();
        cat.set_program_tags("Game", vec!["farm".into()]).unwrap();

        let mut ed = editor_on(EditorTab::Programs);
        ed.selected_program = Some("Game".into());
        ProgramsForm::load(&mut ed, &cat, &settings);
        assert_eq!(ed.form_name, "Game");
        assert_eq!(ed.form_program.process_path, "/bin/game");
        assert_eq!(ed.form_program.window_title, "Game Window");
        assert_eq!(ed.form_tags, vec!["farm".to_string()]);
        assert!(!ProgramsForm::is_dirty(&ed, &cat, &settings));

        // Renaming or editing identity fields marks dirty.
        ed.form_program.process_path = "/bin/other".into();
        assert!(ProgramsForm::is_dirty(&ed, &cat, &settings));
        ed.form_program.process_path = "/bin/game".into();
        ed.form_name = "Renamed".into();
        assert!(ProgramsForm::is_dirty(&ed, &cat, &settings));

        // Unselected + any entered content → dirty (new entity in progress).
        ed.selected_program = None;
        assert!(ProgramsForm::is_dirty(&ed, &cat, &settings));
        ed.form_name.clear();
        ed.form_program.process_path.clear();
        ed.form_program.window_title.clear();
        ed.form_tags.clear();
        assert!(!ProgramsForm::is_dirty(&ed, &cat, &settings));
        ed.form_name = "New".into();
        assert!(ProgramsForm::is_dirty(&ed, &cat, &settings));
    }

    #[test]
    fn items_form_load_dirty_valid() {
        let mut cat = catalog();
        let settings = UserSettings::default();
        cat.upsert_item(
            "Game",
            ProgramItem {
                name: "Potion".into(),
                mask: "circle".into(),
                stack_max: 5,
                grid_cols: 2,
                grid_rows: 3,
                tags: vec!["heal".into()],
            },
        )
        .unwrap();

        let mut ed = editor_on(EditorTab::Items);
        ed.selected_program = Some("Game".into());
        ed.selected_entity = Some("Potion".into());
        ItemsForm::load(&mut ed, &cat, &settings);
        assert_eq!(ed.form_name, "Potion");
        assert_eq!(ed.form_cols, "2");
        assert_eq!(ed.form_rows, "3");
        assert_eq!(ed.form_item.stack_max, "5");
        assert_eq!(ed.form_item.mask, "circle");
        assert_eq!(ed.form_tags, vec!["heal".to_string()]);
        assert!(!ItemsForm::is_dirty(&ed, &cat, &settings));
        assert!(ItemsForm::is_valid(&ed, None));

        ed.form_cols = "0".into();
        assert!(ItemsForm::is_dirty(&ed, &cat, &settings));
        assert!(!ItemsForm::is_valid(&ed, None));

        // Missing entity → dirty forces the form to look edited.
        ed.selected_entity = Some("ghost".into());
        assert!(ItemsForm::is_dirty(&ed, &cat, &settings));
        // Unselected + blank name → clean.
        ed.selected_program = None;
        ed.selected_entity = None;
        ed.form_name.clear();
        assert!(!ItemsForm::is_dirty(&ed, &cat, &settings));
    }

    #[test]
    fn overlay_form_dirty_uses_scale_aware_float_compare() {
        let cat = catalog();
        let mut settings = UserSettings::default();
        let mut btn = sqyre_persist::OverlayButtonConfig::new("b1", "Game");
        btn.macro_name = "Macro".into();
        // Load maps an empty icon to the default id, so a blank config icon
        // would read dirty; set the resolved id to isolate the float checks.
        btn.icon = crate::overlay_icons::DEFAULT_ICON_ID.into();
        btn.border_width = 0.25;
        settings.overlay.overlay_buttons.push(btn);

        let mut ed = editor_on(EditorTab::Overlay);
        ed.selected_program = Some("Game".into());
        ed.selected_entity = Some("b1".into());
        OverlayForm::load(&mut ed, &cat, &settings);
        assert!(!OverlayForm::is_dirty(&ed, &cat, &settings));

        // A real diff below the absolute f32::EPSILON (≈1.19e-7): at 0.25 the
        // representable spacing is finer, so an absolute threshold reported
        // clean while the scaled compare still catches the change.
        ed.form_overlay.border_width += f32::EPSILON / 2.0;
        assert!(OverlayForm::is_dirty(&ed, &cat, &settings));
        ed.form_overlay.border_width -= f32::EPSILON / 2.0;
        assert!(!OverlayForm::is_dirty(&ed, &cat, &settings));

        // Same for the f64 gate tolerance (a 0–1 ratio field): a two-ULP diff
        // at ~0.9 is smaller than an absolute f64::EPSILON would flag at 1.0.
        ed.form_overlay.gate.tolerance += f64::EPSILON;
        assert!(OverlayForm::is_dirty(&ed, &cat, &settings));
    }

    #[test]
    fn points_form_load_and_valid() {
        let mut cat = catalog();
        let settings = UserSettings::default();
        cat.upsert_point(
            "Game",
            ProgramPoint {
                name: "Spawn".into(),
                monitor: 1,
                x: ScalarValue::Int(10),
                y: ScalarValue::Int(20),
            },
        )
        .unwrap();

        let mut ed = editor_on(EditorTab::Points);
        ed.selected_program = Some("Game".into());
        ed.selected_entity = Some("Spawn".into());
        PointsForm::load(&mut ed, &cat, &settings);
        assert_eq!(ed.form_name, "Spawn");
        assert_eq!(ed.form_point.x, "10");
        assert_eq!(ed.form_point.y, "20");
        assert!(!PointsForm::is_dirty(&ed, &cat, &settings));
        assert!(PointsForm::is_valid(&ed, None));

        // Blank coords are invalid.
        ed.form_point.x.clear();
        assert!(!PointsForm::is_valid(&ed, None));
    }

    #[test]
    fn dispatch_helpers_route_to_the_tab_form() {
        let cat = catalog();
        let settings = UserSettings::default();
        let mut ed = editor_on(EditorTab::Programs);
        ed.selected_program = Some("Game".into());
        load_tab(EditorTab::Programs, &mut ed, &cat, &settings);
        assert_eq!(ed.form_name, "Game");
        assert!(!dirty_tab(EditorTab::Programs, &ed, &cat, &settings));
        assert!(valid_tab(EditorTab::Programs, &ed, None));
    }

    #[test]
    fn masks_form_load_and_dirty() {
        let mut cat = catalog();
        let settings = UserSettings::default();
        cat.upsert_mask(
            "Game",
            ProgramMask {
                name: "Hole".into(),
                shape: sqyre_domain::MaskShape::Circle,
                center_x: "10".into(),
                center_y: "20".into(),
                radius: "5".into(),
                ..Default::default()
            },
        )
        .unwrap();

        let mut ed = editor_on(EditorTab::Masks);
        ed.selected_program = Some("Game".into());
        ed.selected_entity = Some("Hole".into());
        MasksForm::load(&mut ed, &cat, &settings);
        assert_eq!(ed.form_name, "Hole");
        assert_eq!(ed.form_mask.shape, "circle");
        assert_eq!(ed.form_mask.center_x, "10");
        assert_eq!(ed.form_mask.radius, "5");
        assert!(!MasksForm::is_dirty(&ed, &cat, &settings));
        assert!(MasksForm::is_valid(&ed, None));

        ed.form_mask.radius = "9".into();
        assert!(MasksForm::is_dirty(&ed, &cat, &settings));
    }

    #[test]
    fn search_areas_form_load_and_dirty() {
        let mut cat = catalog();
        let settings = UserSettings::default();
        cat.upsert_search_area(
            "Game",
            ProgramSearchArea {
                name: "Zone".into(),
                monitor: 1,
                left_x: ScalarValue::Int(0),
                top_y: ScalarValue::Int(0),
                right_x: ScalarValue::Int(50),
                bottom_y: ScalarValue::Int(50),
            },
        )
        .unwrap();

        let mut ed = editor_on(EditorTab::SearchAreas);
        ed.selected_program = Some("Game".into());
        ed.selected_entity = Some("Zone".into());
        SearchAreasForm::load(&mut ed, &cat, &settings);
        assert_eq!(ed.form_name, "Zone");
        assert_eq!(ed.form_rect.left, "0");
        assert_eq!(ed.form_rect.right, "50");
        assert!(!SearchAreasForm::is_dirty(&ed, &cat, &settings));
        assert!(SearchAreasForm::is_valid(&ed, None));

        ed.form_rect.right = "80".into();
        assert!(SearchAreasForm::is_dirty(&ed, &cat, &settings));
    }
}
