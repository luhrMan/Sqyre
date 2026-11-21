//! FormState trait: per-tab load / dirty / valid lifecycle.

use super::helpers::parse_i32;
#[cfg(feature = "native-runtime")]
use super::match_check;
use super::{DataEditor, EditorTab};
use sqyre_domain::{Macro, ScalarValue};
use sqyre_persist::{CatalogView, UserSettings};
use sqyre_validate::{
    validate_entity_name, validate_item_grid_fields, validate_numeric_expression,
    validate_search_area_literal_bounds,
};

/// Tab form lifecycle used by [`DataEditor`] dispatch.
pub(crate) trait FormState {
    fn load(ed: &mut DataEditor, catalog: CatalogView<'_>, settings: &UserSettings);
    fn is_dirty(ed: &DataEditor, catalog: CatalogView<'_>, settings: &UserSettings) -> bool;
    fn is_valid(ed: &DataEditor, active_macro: Option<&Macro>) -> bool;
}

pub(crate) fn load_tab(
    tab: EditorTab,
    ed: &mut DataEditor,
    catalog: CatalogView<'_>,
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
    catalog: CatalogView<'_>,
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
    if !matches!(tab, EditorTab::Overlay) && validate_entity_name(ed.drafts.name.trim()).is_err() {
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
    fn load(ed: &mut DataEditor, catalog: CatalogView<'_>, _settings: &UserSettings) {
        ed.drafts.name = ed.nav.selected_program.clone().unwrap_or_default();
        if let Some(p) = ed
            .nav
            .selected_program
            .as_deref()
            .and_then(|n| catalog.get(n))
        {
            ed.drafts.program.process_path = p.process_path.clone();
            ed.drafts.program.window_title = p.window_title.clone();
            ed.drafts.tags = p.tags.clone();
            let name = ed.drafts.name.trim().to_string();
            ed.drafts.tags.retain(|tag| tag.trim() != name);
        } else {
            ed.drafts.program.process_path.clear();
            ed.drafts.program.window_title.clear();
            ed.drafts.tags.clear();
        }
        ed.drafts.tag_draft.clear();
    }

    fn is_dirty(ed: &DataEditor, catalog: CatalogView<'_>, _settings: &UserSettings) -> bool {
        let Some(sel) = ed.nav.selected_program.as_deref() else {
            return !ed.drafts.name.trim().is_empty()
                || !ed.drafts.program.process_path.is_empty()
                || !ed.drafts.program.window_title.is_empty()
                || !ed.drafts.tags.is_empty();
        };
        let bound = catalog.get(sel);
        let path = bound.map(|p| p.process_path.as_str()).unwrap_or("");
        let title = bound.map(|p| p.window_title.as_str()).unwrap_or("");
        let tags = bound.map(|p| p.tags.as_slice()).unwrap_or(&[]);
        ed.drafts.name.trim() != sel
            || ed.drafts.program.process_path != path
            || ed.drafts.program.window_title != title
            || ed.drafts.tags != tags
    }

    fn is_valid(_ed: &DataEditor, _active_macro: Option<&Macro>) -> bool {
        true
    }
}

impl FormState for ItemsForm {
    fn load(ed: &mut DataEditor, catalog: CatalogView<'_>, _settings: &UserSettings) {
        let (prog, name) = match (
            ed.nav.selected_program.as_deref(),
            ed.nav.selected_entity.as_deref(),
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
        ed.drafts.name = item.name.clone();
        ed.drafts.cols = item.grid_cols.to_string();
        ed.drafts.rows = item.grid_rows.to_string();
        ed.drafts.item.stack_max = item.stack_max.to_string();
        ed.drafts.item.mask = item.mask.clone();
        ed.drafts.tags = item.tags.clone();
    }

    fn is_dirty(ed: &DataEditor, catalog: CatalogView<'_>, _settings: &UserSettings) -> bool {
        let (Some(prog), Some(ent)) = (
            ed.nav.selected_program.as_deref(),
            ed.nav.selected_entity.as_deref(),
        ) else {
            return !ed.drafts.name.trim().is_empty();
        };
        let Some(item) = catalog.get(prog).and_then(|p| p.items.get(ent)) else {
            return true;
        };
        ed.drafts.name.trim() != item.name
            || parse_i32(&ed.drafts.cols) != Some(item.grid_cols)
            || parse_i32(&ed.drafts.rows) != Some(item.grid_rows)
            || parse_i32(&ed.drafts.item.stack_max) != Some(item.stack_max)
            || ed.drafts.item.mask != item.mask
            || ed.drafts.tags != item.tags
    }

    fn is_valid(ed: &DataEditor, _active_macro: Option<&Macro>) -> bool {
        ed.nav.selected_program.is_some()
            && validate_item_grid_fields(
                &ed.drafts.cols,
                &ed.drafts.rows,
                &ed.drafts.item.stack_max,
            )
            .is_ok()
    }
}

impl FormState for PointsForm {
    fn load(ed: &mut DataEditor, catalog: CatalogView<'_>, _settings: &UserSettings) {
        let (prog, name) = match (
            ed.nav.selected_program.as_deref(),
            ed.nav.selected_entity.as_deref(),
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
        ed.drafts.name = pt.name.clone();
        ed.drafts.monitor = pt.monitor.max(1);
        ed.drafts.point.x = pt.x.as_display();
        ed.drafts.point.y = pt.y.as_display();
    }

    fn is_dirty(ed: &DataEditor, catalog: CatalogView<'_>, _settings: &UserSettings) -> bool {
        let (Some(prog), Some(ent)) = (
            ed.nav.selected_program.as_deref(),
            ed.nav.selected_entity.as_deref(),
        ) else {
            return !ed.drafts.name.trim().is_empty();
        };
        let res = catalog.resolution_key();
        let Some(pt) = catalog
            .get(prog)
            .and_then(|p| p.points.get(res).or_else(|| p.points.values().next()))
            .and_then(|m| m.get(ent))
        else {
            return true;
        };
        ed.drafts.name.trim() != pt.name
            || ed.drafts.monitor.max(1) != pt.monitor.max(1)
            || ScalarValue::parse_edit(&ed.drafts.point.x) != pt.x
            || ScalarValue::parse_edit(&ed.drafts.point.y) != pt.y
    }

    fn is_valid(ed: &DataEditor, active_macro: Option<&Macro>) -> bool {
        ed.nav.selected_program.is_some()
            && !ed.drafts.point.x.trim().is_empty()
            && !ed.drafts.point.y.trim().is_empty()
            && !validate_numeric_expression(&ed.drafts.point.x, active_macro).blocks_submit()
            && !validate_numeric_expression(&ed.drafts.point.y, active_macro).blocks_submit()
    }
}

impl FormState for SearchAreasForm {
    fn load(ed: &mut DataEditor, catalog: CatalogView<'_>, _settings: &UserSettings) {
        let (prog, name) = match (
            ed.nav.selected_program.as_deref(),
            ed.nav.selected_entity.as_deref(),
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
        ed.drafts.name = sa.name.clone();
        ed.drafts.monitor = sa.monitor.max(1);
        ed.drafts.rect.load_displays(
            sa.left_x.as_display(),
            sa.top_y.as_display(),
            sa.right_x.as_display(),
            sa.bottom_y.as_display(),
        );
    }

    fn is_dirty(ed: &DataEditor, catalog: CatalogView<'_>, _settings: &UserSettings) -> bool {
        let (Some(prog), Some(ent)) = (
            ed.nav.selected_program.as_deref(),
            ed.nav.selected_entity.as_deref(),
        ) else {
            return !ed.drafts.name.trim().is_empty();
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
        ed.drafts.name.trim() != sa.name
            || ed.drafts.monitor.max(1) != sa.monitor.max(1)
            || ScalarValue::parse_edit(&ed.drafts.rect.left) != sa.left_x
            || ScalarValue::parse_edit(&ed.drafts.rect.top) != sa.top_y
            || ScalarValue::parse_edit(&ed.drafts.rect.right) != sa.right_x
            || ScalarValue::parse_edit(&ed.drafts.rect.bottom) != sa.bottom_y
    }

    fn is_valid(ed: &DataEditor, active_macro: Option<&Macro>) -> bool {
        ed.nav.selected_program.is_some()
            && !ed.drafts.rect.left.trim().is_empty()
            && !ed.drafts.rect.top.trim().is_empty()
            && !ed.drafts.rect.right.trim().is_empty()
            && !ed.drafts.rect.bottom.trim().is_empty()
            && validate_search_area_literal_bounds(
                &ed.drafts.rect.left,
                &ed.drafts.rect.top,
                &ed.drafts.rect.right,
                &ed.drafts.rect.bottom,
            )
            .is_ok()
            && !validate_numeric_expression(&ed.drafts.rect.left, active_macro).blocks_submit()
            && !validate_numeric_expression(&ed.drafts.rect.top, active_macro).blocks_submit()
            && !validate_numeric_expression(&ed.drafts.rect.right, active_macro).blocks_submit()
            && !validate_numeric_expression(&ed.drafts.rect.bottom, active_macro).blocks_submit()
    }
}

impl FormState for MasksForm {
    fn load(ed: &mut DataEditor, catalog: CatalogView<'_>, _settings: &UserSettings) {
        let (prog, name) = match (
            ed.nav.selected_program.as_deref(),
            ed.nav.selected_entity.as_deref(),
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
        ed.drafts.name = mask.name.clone();
        ed.drafts.mask.shape = mask.shape.as_str().to_string();
        ed.drafts.mask.center_x = mask.center_x.clone();
        ed.drafts.mask.center_y = mask.center_y.clone();
        ed.drafts.mask.base = mask.base.clone();
        ed.drafts.mask.height = mask.height.clone();
        ed.drafts.mask.radius = mask.radius.clone();
        ed.drafts.mask.inverse = mask.inverse;
    }

    fn is_dirty(ed: &DataEditor, catalog: CatalogView<'_>, _settings: &UserSettings) -> bool {
        let (Some(prog), Some(ent)) = (
            ed.nav.selected_program.as_deref(),
            ed.nav.selected_entity.as_deref(),
        ) else {
            return !ed.drafts.name.trim().is_empty();
        };
        let Some(mask) = catalog.get(prog).and_then(|p| p.masks.get(ent)) else {
            return true;
        };
        ed.drafts.name.trim() != mask.name
            || ed.drafts.mask.shape != mask.shape.as_str()
            || ed.drafts.mask.center_x != mask.center_x
            || ed.drafts.mask.center_y != mask.center_y
            || ed.drafts.mask.base != mask.base
            || ed.drafts.mask.height != mask.height
            || ed.drafts.mask.radius != mask.radius
            || ed.drafts.mask.inverse != mask.inverse
    }

    fn is_valid(ed: &DataEditor, active_macro: Option<&Macro>) -> bool {
        if ed.nav.selected_program.is_none()
            || (ed.drafts.mask.shape != "rectangle" && ed.drafts.mask.shape != "circle")
            || ed.drafts.mask.center_x.trim().is_empty()
            || ed.drafts.mask.center_y.trim().is_empty()
        {
            return false;
        }
        if validate_numeric_expression(&ed.drafts.mask.center_x, active_macro).blocks_submit()
            || validate_numeric_expression(&ed.drafts.mask.center_y, active_macro).blocks_submit()
        {
            return false;
        }
        if ed.drafts.mask.shape == "circle" {
            !validate_numeric_expression(&ed.drafts.mask.radius, active_macro).blocks_submit()
        } else {
            !validate_numeric_expression(&ed.drafts.mask.base, active_macro).blocks_submit()
                && !validate_numeric_expression(&ed.drafts.mask.height, active_macro)
                    .blocks_submit()
        }
    }
}

impl FormState for CollectionsForm {
    fn load(ed: &mut DataEditor, catalog: CatalogView<'_>, _settings: &UserSettings) {
        let (prog, name) = match (
            ed.nav.selected_program.as_deref(),
            ed.nav.selected_entity.as_deref(),
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
        ed.drafts.search_area = col.name.clone();
        ed.drafts.rows = col.rows.to_string();
        ed.drafts.cols = col.cols.to_string();
        ed.previews.collection.key = Some((prog.to_string(), name.to_string()));
    }

    fn is_dirty(ed: &DataEditor, catalog: CatalogView<'_>, _settings: &UserSettings) -> bool {
        let (Some(prog), Some(ent)) = (
            ed.nav.selected_program.as_deref(),
            ed.nav.selected_entity.as_deref(),
        ) else {
            return !ed.drafts.search_area.trim().is_empty()
                || ed.drafts.rows != "1"
                || ed.drafts.cols != "1";
        };
        let Some(col) = catalog.get(prog).and_then(|p| p.collections.get(ent)) else {
            return true;
        };
        ed.drafts.search_area.trim() != col.name
            || parse_i32(&ed.drafts.rows) != Some(col.rows)
            || parse_i32(&ed.drafts.cols) != Some(col.cols)
    }

    fn is_valid(ed: &DataEditor, _active_macro: Option<&Macro>) -> bool {
        ed.nav.selected_program.is_some()
            && !ed.drafts.search_area.trim().is_empty()
            && parse_i32(&ed.drafts.rows).map(|n| n >= 1).unwrap_or(false)
            && parse_i32(&ed.drafts.cols).map(|n| n >= 1).unwrap_or(false)
    }
}

impl FormState for AtlasesForm {
    fn load(ed: &mut DataEditor, catalog: CatalogView<'_>, _settings: &UserSettings) {
        let (prog, name) = match (
            ed.nav.selected_program.as_deref(),
            ed.nav.selected_entity.as_deref(),
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
        ed.drafts.name = atlas.name.clone();
        ed.drafts.atlas.members = atlas.collections.clone();
        ed.drafts.atlas.add.clear();
        ed.previews.atlas.key = Some((prog.to_string(), name.to_string()));
    }

    fn is_dirty(ed: &DataEditor, catalog: CatalogView<'_>, _settings: &UserSettings) -> bool {
        let (Some(prog), Some(ent)) = (
            ed.nav.selected_program.as_deref(),
            ed.nav.selected_entity.as_deref(),
        ) else {
            return !ed.drafts.name.trim().is_empty() || !ed.drafts.atlas.members.is_empty();
        };
        let Some(atlas) = catalog.get(prog).and_then(|p| p.atlases.get(ent)) else {
            return true;
        };
        ed.drafts.name.trim() != atlas.name || ed.drafts.atlas.members != atlas.collections
    }

    fn is_valid(ed: &DataEditor, _active_macro: Option<&Macro>) -> bool {
        ed.nav.selected_program.is_some() && !ed.drafts.atlas.members.is_empty()
    }
}

impl FormState for ScreenCapForm {
    fn load(ed: &mut DataEditor, catalog: CatalogView<'_>, _settings: &UserSettings) {
        let (prog, name) = match (
            ed.nav.selected_program.clone(),
            ed.nav.selected_entity.clone(),
        ) {
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
        if ed.apply_screen_cap_reference(catalog, coord) {
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
        ed.drafts.name = sa.name.clone();
        ed.drafts.search_area = format!("{prog}{}{name}", sqyre_domain::PROGRAM_DELIMITER);
        ed.drafts.monitor = sa.monitor.max(1);
        ed.drafts.rect.load_displays(
            sa.left_x.as_display(),
            sa.top_y.as_display(),
            sa.right_x.as_display(),
            sa.bottom_y.as_display(),
        );
    }

    fn is_dirty(_ed: &DataEditor, _catalog: CatalogView<'_>, _settings: &UserSettings) -> bool {
        false
    }

    fn is_valid(_ed: &DataEditor, _active_macro: Option<&Macro>) -> bool {
        false
    }
}

impl FormState for MatchCheckForm {
    fn load(ed: &mut DataEditor, catalog: CatalogView<'_>, _settings: &UserSettings) {
        let (prog, name) = match (
            ed.nav.selected_program.clone(),
            ed.nav.selected_entity.clone(),
        ) {
            (Some(p), Some(n)) => (p, n),
            _ => {
                ed.drafts.name.clear();
                return;
            }
        };
        let Some(item) = catalog.get(&prog).and_then(|p| p.items.get(&name)) else {
            ed.drafts.name.clear();
            return;
        };
        ed.drafts.name = item.name.clone();
        #[cfg(feature = "native-runtime")]
        {
            ed.match_check.invalidate();
            ed.match_check.settings.paused = false;
            ed.match_check.settings.last_live_at = None;
            ed.match_check.settings.capture_once = false;
            ed.match_check.settings.awaiting_panel = false;
            ed.match_check.settings.rematch_once = false;
            let variant = match_check::default_variant(catalog, &prog, &name);
            ed.match_check.settings.variant = variant;
        }
    }

    fn is_dirty(_ed: &DataEditor, _catalog: CatalogView<'_>, _settings: &UserSettings) -> bool {
        false
    }

    fn is_valid(ed: &DataEditor, active_macro: Option<&Macro>) -> bool {
        #[cfg(feature = "native-runtime")]
        {
            if ed.nav.selected_program.is_none() || ed.nav.selected_entity.is_none() {
                return false;
            }
            validate_search_area_literal_bounds(
                &ed.drafts.rect.left,
                &ed.drafts.rect.top,
                &ed.drafts.rect.right,
                &ed.drafts.rect.bottom,
            )
            .is_ok()
                && !validate_numeric_expression(&ed.drafts.rect.left, active_macro).blocks_submit()
                && !validate_numeric_expression(&ed.drafts.rect.top, active_macro).blocks_submit()
                && !validate_numeric_expression(&ed.drafts.rect.right, active_macro).blocks_submit()
                && !validate_numeric_expression(&ed.drafts.rect.bottom, active_macro)
                    .blocks_submit()
        }
        #[cfg(not(feature = "native-runtime"))]
        {
            let _ = (ed, active_macro);
            false
        }
    }
}

impl FormState for OverlayForm {
    fn load(ed: &mut DataEditor, _catalog: CatalogView<'_>, settings: &UserSettings) {
        ed.load_overlay_form(settings);
    }

    fn is_dirty(ed: &DataEditor, _catalog: CatalogView<'_>, settings: &UserSettings) -> bool {
        let Some(id) = ed.nav.selected_entity.as_deref() else {
            return false;
        };
        let Some(btn) = settings.overlay.overlay_buttons.iter().find(|b| b.id == id) else {
            return true;
        };
        ed.drafts.overlay.placement_is_dirty_vs(btn)
            || ed.drafts.overlay.style_is_dirty_vs(btn)
            || ed.nav.selected_program.as_deref() != Some(btn.program.as_str())
            || ed.drafts.overlay.gate.is_dirty_vs(&btn.visibility_gate)
    }

    fn is_valid(ed: &DataEditor, _active_macro: Option<&Macro>) -> bool {
        ed.nav.selected_program.is_some()
            && ed.nav.selected_entity.is_some()
            && !ed.drafts.overlay.macro_name.trim().is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data_editor::nav::EditorNav;
    use sqyre_persist::{
        ProgramCatalog, ProgramItem, ProgramMask, ProgramPoint, ProgramSearchArea,
    };

    fn catalog() -> ProgramCatalog {
        let mut cat = ProgramCatalog::default();
        cat.set_resolution_key("1920x1080");
        cat.create_program("Game").unwrap();
        cat
    }

    fn editor_on(tab: EditorTab) -> DataEditor {
        DataEditor {
            nav: EditorNav {
                tab,
                ..Default::default()
            },
            ..Default::default()
        }
    }

    #[test]
    fn valid_tab_rejects_bad_entity_names_everywhere() {
        let mut ed = editor_on(EditorTab::Items);
        ed.nav.selected_program = Some("Game".into());
        ed.drafts.name = "a/b".into();
        ed.drafts.cols = "1".into();
        ed.drafts.rows = "1".into();
        ed.drafts.item.stack_max = "0".into();
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
        ed.nav.selected_program = Some("Game".into());
        ProgramsForm::load(&mut ed, CatalogView::from(&cat), &settings);
        assert_eq!(ed.drafts.name, "Game");
        assert_eq!(ed.drafts.program.process_path, "/bin/game");
        assert_eq!(ed.drafts.program.window_title, "Game Window");
        assert_eq!(ed.drafts.tags, vec!["farm".to_string()]);
        assert!(!ProgramsForm::is_dirty(
            &ed,
            CatalogView::from(&cat),
            &settings
        ));

        // Renaming or editing identity fields marks dirty.
        ed.drafts.program.process_path = "/bin/other".into();
        assert!(ProgramsForm::is_dirty(
            &ed,
            CatalogView::from(&cat),
            &settings
        ));
        ed.drafts.program.process_path = "/bin/game".into();
        ed.drafts.name = "Renamed".into();
        assert!(ProgramsForm::is_dirty(
            &ed,
            CatalogView::from(&cat),
            &settings
        ));

        // Unselected + any entered content → dirty (new entity in progress).
        ed.nav.selected_program = None;
        assert!(ProgramsForm::is_dirty(
            &ed,
            CatalogView::from(&cat),
            &settings
        ));
        ed.drafts.name.clear();
        ed.drafts.program.process_path.clear();
        ed.drafts.program.window_title.clear();
        ed.drafts.tags.clear();
        assert!(!ProgramsForm::is_dirty(
            &ed,
            CatalogView::from(&cat),
            &settings
        ));
        ed.drafts.name = "New".into();
        assert!(ProgramsForm::is_dirty(
            &ed,
            CatalogView::from(&cat),
            &settings
        ));
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
        ed.nav.selected_program = Some("Game".into());
        ed.nav.selected_entity = Some("Potion".into());
        ItemsForm::load(&mut ed, CatalogView::from(&cat), &settings);
        assert_eq!(ed.drafts.name, "Potion");
        assert_eq!(ed.drafts.cols, "2");
        assert_eq!(ed.drafts.rows, "3");
        assert_eq!(ed.drafts.item.stack_max, "5");
        assert_eq!(ed.drafts.item.mask, "circle");
        assert_eq!(ed.drafts.tags, vec!["heal".to_string()]);
        assert!(!ItemsForm::is_dirty(
            &ed,
            CatalogView::from(&cat),
            &settings
        ));
        assert!(ItemsForm::is_valid(&ed, None));

        ed.drafts.cols = "0".into();
        assert!(ItemsForm::is_dirty(&ed, CatalogView::from(&cat), &settings));
        assert!(!ItemsForm::is_valid(&ed, None));

        // Missing entity → dirty forces the form to look edited.
        ed.nav.selected_entity = Some("ghost".into());
        assert!(ItemsForm::is_dirty(&ed, CatalogView::from(&cat), &settings));
        // Unselected + blank name → clean.
        ed.nav.selected_program = None;
        ed.nav.selected_entity = None;
        ed.drafts.name.clear();
        assert!(!ItemsForm::is_dirty(
            &ed,
            CatalogView::from(&cat),
            &settings
        ));
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
        ed.nav.selected_program = Some("Game".into());
        ed.nav.selected_entity = Some("b1".into());
        OverlayForm::load(&mut ed, CatalogView::from(&cat), &settings);
        assert!(!OverlayForm::is_dirty(
            &ed,
            CatalogView::from(&cat),
            &settings
        ));

        // A real diff below the absolute f32::EPSILON (≈1.19e-7): at 0.25 the
        // representable spacing is finer, so an absolute threshold reported
        // clean while the scaled compare still catches the change.
        ed.drafts.overlay.border_width += f32::EPSILON / 2.0;
        assert!(OverlayForm::is_dirty(
            &ed,
            CatalogView::from(&cat),
            &settings
        ));
        ed.drafts.overlay.border_width -= f32::EPSILON / 2.0;
        assert!(!OverlayForm::is_dirty(
            &ed,
            CatalogView::from(&cat),
            &settings
        ));

        // Same for the f64 gate tolerance (a 0–1 ratio field): a two-ULP diff
        // at ~0.9 is smaller than an absolute f64::EPSILON would flag at 1.0.
        ed.drafts.overlay.gate.tolerance += f64::EPSILON;
        assert!(OverlayForm::is_dirty(
            &ed,
            CatalogView::from(&cat),
            &settings
        ));
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
        ed.nav.selected_program = Some("Game".into());
        ed.nav.selected_entity = Some("Spawn".into());
        PointsForm::load(&mut ed, CatalogView::from(&cat), &settings);
        assert_eq!(ed.drafts.name, "Spawn");
        assert_eq!(ed.drafts.point.x, "10");
        assert_eq!(ed.drafts.point.y, "20");
        assert!(!PointsForm::is_dirty(
            &ed,
            CatalogView::from(&cat),
            &settings
        ));
        assert!(PointsForm::is_valid(&ed, None));

        // Blank coords are invalid.
        ed.drafts.point.x.clear();
        assert!(!PointsForm::is_valid(&ed, None));
    }

    #[test]
    fn dispatch_helpers_route_to_the_tab_form() {
        let cat = catalog();
        let settings = UserSettings::default();
        let mut ed = editor_on(EditorTab::Programs);
        ed.nav.selected_program = Some("Game".into());
        load_tab(
            EditorTab::Programs,
            &mut ed,
            CatalogView::from(&cat),
            &settings,
        );
        assert_eq!(ed.drafts.name, "Game");
        assert!(!dirty_tab(
            EditorTab::Programs,
            &ed,
            CatalogView::from(&cat),
            &settings
        ));
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
        ed.nav.selected_program = Some("Game".into());
        ed.nav.selected_entity = Some("Hole".into());
        MasksForm::load(&mut ed, CatalogView::from(&cat), &settings);
        assert_eq!(ed.drafts.name, "Hole");
        assert_eq!(ed.drafts.mask.shape, "circle");
        assert_eq!(ed.drafts.mask.center_x, "10");
        assert_eq!(ed.drafts.mask.radius, "5");
        assert!(!MasksForm::is_dirty(
            &ed,
            CatalogView::from(&cat),
            &settings
        ));
        assert!(MasksForm::is_valid(&ed, None));

        ed.drafts.mask.radius = "9".into();
        assert!(MasksForm::is_dirty(&ed, CatalogView::from(&cat), &settings));
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
        ed.nav.selected_program = Some("Game".into());
        ed.nav.selected_entity = Some("Zone".into());
        SearchAreasForm::load(&mut ed, CatalogView::from(&cat), &settings);
        assert_eq!(ed.drafts.name, "Zone");
        assert_eq!(ed.drafts.rect.left, "0");
        assert_eq!(ed.drafts.rect.right, "50");
        assert!(!SearchAreasForm::is_dirty(
            &ed,
            CatalogView::from(&cat),
            &settings
        ));
        assert!(SearchAreasForm::is_valid(&ed, None));

        ed.drafts.rect.right = "80".into();
        assert!(SearchAreasForm::is_dirty(
            &ed,
            CatalogView::from(&cat),
            &settings
        ));
    }
}
