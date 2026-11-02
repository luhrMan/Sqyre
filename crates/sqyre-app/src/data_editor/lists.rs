//! Left entity list + program selector (cached against catalog generation).

use super::helpers::{default_editor_program, editor_program_names, is_editor_listed_program};
use super::{DataEditor, EditorTab, ListCache, SearchScope};
use crate::action_tooltip::help;
use crate::data_editor_preview::show_file_hover;
use crate::help_text as ht;
use crate::icon_cache::IconCache;
use crate::overlay_icons;
use crate::pickers::{self, PickerScrollOpts};
use crate::preview_tooltip::{PreviewKind, PreviewTooltipCache};
use crate::widgets::searchable_combo;
use eframe::egui;
use sqyre_persist::{OverlayButtonConfig, ProgramCatalog, ProgramCollection, UserSettings};
use std::cell::Cell;
use std::collections::HashMap;

impl DataEditor {
    pub(crate) fn draw_left_list(
        &mut self,
        ui: &mut egui::Ui,
        catalog: &ProgramCatalog,
        icons: &mut IconCache,
        previews: &mut PreviewTooltipCache,
        settings: &mut UserSettings,
    ) {
        let id_salt = match self.tab {
            EditorTab::Programs => "data_editor_programs",
            EditorTab::Items | EditorTab::Masks | EditorTab::ScreenCap | EditorTab::PixelCheck => {
                "data_editor_items"
            }
            EditorTab::Overlay => "data_editor_overlay_list",
            EditorTab::Points
            | EditorTab::SearchAreas
            | EditorTab::Collections
            | EditorTab::Atlases => "data_editor_coords",
        };
        let tab = self.tab;
        // Entity tabs list only the selected program by default; the All
        // programs scope brings back the collapsible program sections.
        let grouped =
            matches!(self.search_scope, SearchScope::AllPrograms) && tab != EditorTab::Programs;
        // Owned names so the expand/collapse toggle does not borrow `catalog`
        // across the list body.
        let program_names_for_collapse: Vec<String> = if grouped {
            editor_program_names(catalog).cloned().collect()
        } else {
            Vec::new()
        };
        let mut opts = PickerScrollOpts::pane();
        opts.id_salt = Some(id_salt);
        // The query field lives in the window header, not the pane.
        opts.hint_text = None;

        let mut item_selection: Vec<String> = match (
            self.tab,
            self.selected_program.as_deref(),
            self.selected_entity.as_deref(),
        ) {
            (EditorTab::Items | EditorTab::PixelCheck, Some(p), Some(e)) => {
                vec![format!("{p}{}{e}", sqyre_domain::PROGRAM_DELIMITER)]
            }
            _ => Vec::new(),
        };
        let mut clicked_program: Option<String> = None;
        let mut clicked_entity: Option<(String, String)> = None;
        let mut clicked_overlay: Option<(String, String)> = None;
        let mut overlay_enabled_updates: Vec<(String, bool)> = Vec::new();
        // (program, entity) picked from a row's right-click Delete; `None` entity = program.
        let mut delete_target: Option<(String, Option<String>)> = None;

        // The header query filters only while its field is expanded; the text
        // itself is kept while hidden.
        let search_open = self.search_open;
        let mut search = if search_open {
            std::mem::take(&mut self.search)
        } else {
            String::new()
        };
        // Cells: group headers and row painters share the scroll arm without
        // overlapping `&mut` borrows.
        let scroll_to = Cell::new(std::mem::take(&mut self.scroll_left_list_to_selection));
        let did_scroll = Cell::new(false);

        #[cfg(feature = "native-runtime")]
        let wants_add_running = Cell::new(false);
        #[cfg(feature = "native-runtime")]
        let running_pending = self.add_running_programs_pending();
        pickers::picker_searchable_scroll(ui, &mut search, opts, |ui, q| {
            let mut visible = 0usize;
            // Expand/collapse-all sits at the top of the list (no row above
            // the scroll) and only makes sense over program groups.
            if grouped {
                // Shared single toggle (macro tree / pickers use the same helper).
                let ids: Vec<egui::Id> = match tab {
                    EditorTab::Items | EditorTab::PixelCheck => program_names_for_collapse
                        .iter()
                        .map(|n| pickers::items_icon_grid_collapse_id(n))
                        .collect(),
                    _ => program_names_for_collapse
                        .iter()
                        .map(|n| data_editor_list_collapse_id(tab, n))
                        .collect(),
                };
                if !ids.is_empty() {
                    pickers::collapse_all_button(ui, ids.iter().copied(), |ctx, open| match tab {
                        EditorTab::Items | EditorTab::PixelCheck => {
                            pickers::set_items_icon_grid_openness(
                                ctx,
                                program_names_for_collapse.iter().map(|n| n.as_str()),
                                open,
                            );
                        }
                        _ => pickers::set_collapsing_openness(
                            ctx,
                            program_names_for_collapse
                                .iter()
                                .map(|n| data_editor_list_collapse_id(tab, n)),
                            open,
                        ),
                    });
                }
            }
            let entity = match self.tab {
                EditorTab::Programs => "programs",
                EditorTab::Items | EditorTab::PixelCheck => "items",
                EditorTab::Overlay => "overlay buttons",
                EditorTab::Points => "points",
                EditorTab::SearchAreas => "search areas",
                EditorTab::Masks => "masks",
                EditorTab::Collections => "collections",
                EditorTab::Atlases => "atlases",
                EditorTab::ScreenCap => "captures",
            };
            match self.tab {
                EditorTab::Programs => {
                    #[cfg(feature = "native-runtime")]
                    {
                        if ui
                            .add_enabled(
                                !running_pending,
                                crate::widgets::IconButton::new(
                                    egui_phosphor::regular::PERSON_SIMPLE_RUN,
                                    ht::ED_ADD_ALL_PROGRAMS,
                                    Some(crate::theme::MACRO_START),
                                ),
                            )
                            .clicked()
                        {
                            wants_add_running.set(true);
                        }
                    }
                    for name in editor_program_names(catalog) {
                        if !q.is_empty() && !pickers::fuzzy_match_fold(q, name) {
                            continue;
                        }
                        visible += 1;
                        let selected = self.selected_program.as_deref() == Some(name.as_str());
                        let resp = crate::icon_cache::paint_program_label(
                            ui,
                            catalog,
                            icons,
                            name,
                            crate::icon_cache::ProgramLabelStyle::Selectable { selected },
                            settings.compact_program_headers,
                        );
                        if selected {
                            list_scroll_to(ui, &resp, &scroll_to, &did_scroll);
                        }
                        if resp.clicked() {
                            self.select_program(name, catalog, settings);
                        }
                        if crate::widgets::response_danger_menu(ui, &resp, "Delete") {
                            delete_target = Some((name.clone(), None));
                        }
                    }
                }
                EditorTab::Items | EditorTab::PixelCheck => {
                    ui.set_max_width(ui.available_width());
                    let mut item_menu = |ui: &mut egui::Ui, target: &str| {
                        if crate::widgets::menu_item_danger(ui, "Delete", true) {
                            if let Some((prog, item)) =
                                target.split_once(sqyre_domain::PROGRAM_DELIMITER)
                            {
                                delete_target = Some((prog.to_string(), Some(item.to_string())));
                            }
                        }
                    };
                    // Pixel Check reuses the items grid read-only.
                    let menu: Option<&mut pickers::IconCellMenu<'_>> =
                        if matches!(self.tab, EditorTab::Items) {
                            Some(&mut item_menu)
                        } else {
                            None
                        };
                    let sort = self.items_list_sort;
                    if grouped {
                        // Program accordion (grid manages its own scroll flag).
                        let mut grid_scroll = scroll_to.get();
                        pickers::paint_items_icon_grid(
                            ui,
                            catalog,
                            icons,
                            q,
                            &mut item_selection,
                            false,
                            self.selected_program.as_deref(),
                            &mut clicked_program,
                            settings.compact_program_headers,
                            settings.show_empty_program_headers,
                            Some(&mut grid_scroll),
                            sort,
                            &self.items_tag_priority,
                            menu,
                        );
                        scroll_to.set(grid_scroll);
                        visible = catalog
                            .program_names()
                            .filter_map(|prog| {
                                let pdata = catalog.get(prog)?;
                                Some(
                                    pickers::program_item_targets(
                                        pdata,
                                        prog,
                                        q,
                                        true,
                                        sort,
                                        &self.items_tag_priority,
                                    )
                                    .len(),
                                )
                            })
                            .sum();
                    } else if let Some(prog) = self.selected_program.clone() {
                        // Flat grid of just the selected program's items.
                        if let Some(pdata) = catalog.get(&prog) {
                            let targets = pickers::program_item_targets(
                                pdata,
                                &prog,
                                q,
                                false,
                                sort,
                                &self.items_tag_priority,
                            );
                            visible = pickers::paint_program_items_grid(
                                ui,
                                catalog,
                                icons,
                                &targets,
                                &mut item_selection,
                                false,
                                menu,
                            );
                        }
                    }
                }
                EditorTab::Points
                | EditorTab::SearchAreas
                | EditorTab::Masks
                | EditorTab::Collections
                | EditorTab::Atlases => {
                    let kind = match self.tab {
                        EditorTab::Points => Some(PreviewKind::Point),
                        EditorTab::SearchAreas => Some(PreviewKind::SearchArea),
                        EditorTab::Collections => Some(PreviewKind::Collection),
                        _ => None,
                    };
                    self.ensure_list_cache(catalog);
                    let progs = if grouped {
                        self.list_cache.program_names.clone()
                    } else {
                        self.selected_program.clone().into_iter().collect()
                    };
                    for prog in &progs {
                        let entities = self.entity_names(catalog, prog);
                        if entities.is_empty() && !settings.show_empty_program_headers {
                            continue;
                        }
                        // A program-name hit only matters in the grouped scope.
                        let prog_match =
                            grouped && (q.is_empty() || pickers::fuzzy_match_fold(q, prog));
                        let any_entity = entities
                            .iter()
                            .any(|e| q.is_empty() || pickers::fuzzy_match_fold(q, e));
                        if !prog_match && !any_entity {
                            continue;
                        }
                        let prog_selected = self.selected_program.as_deref() == Some(prog.as_str());
                        let child_count = entities.len();
                        if grouped {
                            visible += 1;
                        }
                        let mut rows = |ui: &mut egui::Ui, icons: &mut IconCache| {
                            visible += entity_rows(
                                ui,
                                tab,
                                kind,
                                catalog,
                                icons,
                                previews,
                                prog,
                                &entities,
                                prog_selected,
                                self.selected_entity.as_deref(),
                                q,
                                prog_match,
                                &scroll_to,
                                &did_scroll,
                                &mut clicked_entity,
                                &mut delete_target,
                            );
                        };
                        if grouped {
                            program_group_section(
                                ui,
                                tab,
                                catalog,
                                icons,
                                settings.compact_program_headers,
                                prog,
                                child_count,
                                prog_selected,
                                &scroll_to,
                                &did_scroll,
                                &mut clicked_program,
                                rows,
                            );
                        } else {
                            ui.set_max_width(ui.available_width());
                            rows(ui, icons);
                        }
                    }
                }
                EditorTab::ScreenCap => {
                    self.ensure_list_cache(catalog);
                    let progs = if grouped {
                        self.list_cache.program_names.clone()
                    } else {
                        self.selected_program.clone().into_iter().collect()
                    };
                    for prog in &progs {
                        let search_areas = self.entity_names(catalog, prog);
                        let collections: Vec<ProgramCollection> = catalog
                            .get(prog)
                            .map(|p| p.collections.values().cloned().collect())
                            .unwrap_or_default();
                        if search_areas.is_empty()
                            && collections.is_empty()
                            && !settings.show_empty_program_headers
                        {
                            continue;
                        }
                        let prog_match =
                            grouped && (q.is_empty() || pickers::fuzzy_match_fold(q, prog));
                        let any_sa = search_areas
                            .iter()
                            .any(|e| q.is_empty() || pickers::fuzzy_match_fold(q, e));
                        let any_col = collections
                            .iter()
                            .any(|c| q.is_empty() || pickers::fuzzy_match_fold(q, &c.name));
                        if !prog_match && !any_sa && !any_col {
                            continue;
                        }
                        let prog_selected = self.selected_program.as_deref() == Some(prog.as_str());
                        let child_count = search_areas.len() + collections.len();
                        if grouped {
                            visible += 1;
                        }
                        let mut rows = |ui: &mut egui::Ui, icons: &mut IconCache| {
                            let in_collection =
                                sqyre_domain::CoordinateRef(self.form_search_area.clone())
                                    .is_collection();
                            for ent in &search_areas {
                                if !q.is_empty()
                                    && !pickers::fuzzy_match_fold(q, ent)
                                    && !prog_match
                                {
                                    continue;
                                }
                                visible += 1;
                                let selected = prog_selected
                                    && self.selected_entity.as_deref() == Some(ent.as_str())
                                    && !in_collection;
                                let resp = entity_row(
                                    ui,
                                    tab,
                                    Some(PreviewKind::SearchArea),
                                    catalog,
                                    icons,
                                    previews,
                                    prog,
                                    ent,
                                    selected,
                                );
                                if selected {
                                    list_scroll_to(ui, &resp, &scroll_to, &did_scroll);
                                }
                                if resp.clicked() {
                                    clicked_entity = Some((prog.clone(), ent.clone()));
                                }
                            }
                            for col in &collections {
                                if !q.is_empty()
                                    && !pickers::fuzzy_match_fold(q, &col.name)
                                    && !prog_match
                                {
                                    continue;
                                }
                                visible += 1;
                                let current =
                                    sqyre_domain::CoordinateRef(self.form_search_area.clone());
                                let selected = current.is_collection()
                                    && current.program() == Some(prog.as_str())
                                    && current.name() == col.name;
                                let label = format!("{} (collection)", col.name);
                                let resp = ui.selectable_label(selected, label);
                                previews.show_for_entity(
                                    ui,
                                    &resp,
                                    catalog,
                                    prog,
                                    &col.name,
                                    PreviewKind::Collection,
                                );
                                if selected {
                                    list_scroll_to(ui, &resp, &scroll_to, &did_scroll);
                                }
                                if resp.clicked() {
                                    self.select_screencap_collection(prog, col, selected);
                                }
                            }
                        };
                        if grouped {
                            program_group_section(
                                ui,
                                tab,
                                catalog,
                                icons,
                                settings.compact_program_headers,
                                prog,
                                child_count,
                                prog_selected,
                                &scroll_to,
                                &did_scroll,
                                &mut clicked_program,
                                rows,
                            );
                        } else {
                            ui.set_max_width(ui.available_width());
                            rows(ui, icons);
                        }
                    }
                }
                EditorTab::Overlay => {
                    let live_preview = self.overlay_edit_preview();
                    let progs: Vec<String> = if grouped {
                        editor_program_names(catalog).cloned().collect()
                    } else {
                        self.selected_program.clone().into_iter().collect()
                    };
                    for prog in &progs {
                        let mut buttons: Vec<_> = settings
                            .overlay_buttons
                            .iter()
                            .filter(|b| b.program == *prog)
                            .cloned()
                            .collect();
                        buttons.sort_by(|a, b| {
                            crate::macro_meta::cmp_display_name(a.display_name(), b.display_name())
                                .then_with(|| a.id.cmp(&b.id))
                        });
                        if buttons.is_empty() && !settings.show_empty_program_headers {
                            continue;
                        }
                        let prog_match =
                            grouped && (q.is_empty() || pickers::fuzzy_match_fold(q, prog));
                        let any_btn = buttons.iter().any(|b| {
                            q.is_empty()
                                || pickers::fuzzy_match_fold(q, b.display_name())
                                || pickers::fuzzy_match_fold(q, &b.id)
                        });
                        if !prog_match && !any_btn {
                            continue;
                        }
                        let prog_selected = self.selected_program.as_deref() == Some(prog.as_str());
                        let child_count = buttons.len();
                        if grouped {
                            visible += 1;
                        }
                        let mut rows = |ui: &mut egui::Ui, _icons: &mut IconCache| {
                            visible += overlay_button_rows(
                                ui,
                                prog,
                                &buttons,
                                live_preview.as_ref(),
                                prog_selected,
                                self.selected_entity.as_deref(),
                                q,
                                prog_match,
                                &scroll_to,
                                &did_scroll,
                                &mut clicked_overlay,
                                &mut overlay_enabled_updates,
                                &mut delete_target,
                            );
                        };
                        if grouped {
                            program_group_section(
                                ui,
                                tab,
                                catalog,
                                icons,
                                settings.compact_program_headers,
                                prog,
                                child_count,
                                prog_selected,
                                &scroll_to,
                                &did_scroll,
                                &mut clicked_program,
                                rows,
                            );
                        } else {
                            ui.set_max_width(ui.available_width());
                            rows(ui, icons);
                        }
                    }
                }
            }
            crate::widgets::list_vacancy(ui, q, visible, entity);
        });
        if search_open {
            self.search = search;
        }

        #[cfg(feature = "native-runtime")]
        if wants_add_running.get() {
            self.start_add_running_programs();
        }
        // Keep an unconsumed arm so a filtered-out selection can still scroll
        // into view later; the header search field re-arms it on edits.
        self.scroll_left_list_to_selection = scroll_to.get();

        if !overlay_enabled_updates.is_empty() {
            for (id, enabled) in &overlay_enabled_updates {
                if let Some(btn) = settings.overlay_buttons.iter_mut().find(|b| b.id == *id) {
                    btn.enabled = *enabled;
                }
                if self.selected_entity.as_deref() == Some(id.as_str()) {
                    self.form_overlay_enabled = *enabled;
                }
            }
            self.persist_overlay_settings(settings);
        }

        if let Some((prog, entity)) = delete_target {
            match entity {
                Some(entity) => self.select_entity(&prog, &entity, catalog, settings),
                None => self.select_program(&prog, catalog, settings),
            }
            self.request_delete_selected();
        } else if let Some(prog) = clicked_program {
            self.select_program(&prog, catalog, settings);
        } else if let Some((prog, id)) = clicked_overlay {
            self.select_entity(&prog, &id, catalog, settings);
        } else if let Some((prog, ent)) = clicked_entity {
            self.select_entity(&prog, &ent, catalog, settings);
        } else if matches!(self.tab, EditorTab::Items | EditorTab::PixelCheck) {
            if let Some(target) = item_selection.first() {
                if let Some((prog, item)) = target.split_once(sqyre_domain::PROGRAM_DELIMITER) {
                    let changed = self.selected_program.as_deref() != Some(prog)
                        || self.selected_entity.as_deref() != Some(item);
                    if changed {
                        self.select_entity(prog, item, catalog, settings);
                    }
                }
            }
        }
    }

    /// Model count for `program` on the current tab (program dropdown badge).
    fn tab_model_count(
        &mut self,
        catalog: &ProgramCatalog,
        settings: &UserSettings,
        program: &str,
    ) -> usize {
        match self.tab {
            // Screen Cap lists search areas and collections together.
            EditorTab::ScreenCap => {
                self.entity_names(catalog, program).len()
                    + catalog
                        .get(program)
                        .map(|p| p.collections.len())
                        .unwrap_or(0)
            }
            EditorTab::Overlay => settings
                .overlay_buttons
                .iter()
                .filter(|b| b.program == program)
                .count(),
            EditorTab::Programs => 0,
            _ => self.entity_names(catalog, program).len(),
        }
    }

    /// Cached entity keys for `program` on the current tab.
    pub(crate) fn entity_names(&mut self, catalog: &ProgramCatalog, program: &str) -> Vec<String> {
        self.ensure_list_cache(catalog);
        self.list_cache
            .entities_by_program
            .get(program)
            .cloned()
            .unwrap_or_default()
    }

    pub(crate) fn ensure_list_cache(&mut self, catalog: &ProgramCatalog) {
        let gen = catalog.generation();
        let res = catalog.resolution_key();
        if self.list_cache.catalog_generation == gen
            && self.list_cache.resolution_key == res
            && self.list_cache.tab == self.tab
        {
            return;
        }
        self.rebuild_list_cache(catalog);
    }

    pub(crate) fn rebuild_list_cache(&mut self, catalog: &ProgramCatalog) {
        let gen = catalog.generation();
        let res = catalog.resolution_key().to_string();
        let tab = self.tab;
        let mut entities_by_program: HashMap<String, Vec<String>> = HashMap::new();
        let program_names: Vec<String> = editor_program_names(catalog).cloned().collect();
        for prog in &program_names {
            let names = compute_entity_names(catalog, tab, prog);
            entities_by_program.insert(prog.clone(), names);
        }
        self.list_cache = ListCache {
            catalog_generation: gen,
            resolution_key: res,
            tab,
            program_names,
            entities_by_program,
        };
    }

    pub(crate) fn select_program(
        &mut self,
        name: &str,
        catalog: &ProgramCatalog,
        settings: &UserSettings,
    ) {
        if !is_editor_listed_program(name) {
            return;
        }
        self.selected_program = Some(name.to_string());
        self.selected_entity = None;
        self.load_form(catalog, settings);
    }

    /// Open Coordinates → Points with a filled entity (docs screenshots).
    pub(crate) fn open_points_for_docs(
        &mut self,
        program: &str,
        point: &str,
        catalog: &ProgramCatalog,
    ) {
        let settings = UserSettings::default();
        self.open = true;
        self.switch_tab(EditorTab::Points, catalog, &settings);
        self.select_entity(program, point, catalog, &settings);
    }

    /// Open the editor on the Programs tab (docs / interaction harnesses).
    pub(crate) fn open_programs_for_docs(&mut self, catalog: &ProgramCatalog) {
        let settings = UserSettings::default();
        self.open = true;
        self.switch_tab(EditorTab::Programs, catalog, &settings);
    }

    pub(crate) fn select_entity(
        &mut self,
        program: &str,
        entity: &str,
        catalog: &ProgramCatalog,
        settings: &UserSettings,
    ) {
        if !is_editor_listed_program(program) {
            return;
        }
        self.selected_program = Some(program.to_string());
        self.selected_entity = Some(entity.to_string());
        self.load_form(catalog, settings);
    }

    /// Screen Cap collection row: select it and open the cell picker.
    fn select_screencap_collection(
        &mut self,
        prog: &str,
        col: &ProgramCollection,
        was_selected: bool,
    ) {
        let current = sqyre_domain::CoordinateRef(self.form_search_area.clone());
        let initial = if was_selected {
            current.cell_range()
        } else {
            None
        };
        self.selected_program = Some(prog.to_string());
        self.selected_entity = Some(col.name.clone());
        self.window_picker = pickers::ActivePicker::Coord {
            kind: pickers::CoordKind::SearchArea,
            search: String::new(),
            value: self.form_search_area.clone(),
            cell_pick: Some(
                pickers::CollectionCellPick::new(prog, &col.name, col.rows, col.cols)
                    .with_initial_sel(initial),
            ),
            scroll_to_selection: false,
        };
    }

    pub(crate) fn program_selector(
        &mut self,
        ui: &mut egui::Ui,
        catalog: &ProgramCatalog,
        icons: &mut IconCache,
        settings: &UserSettings,
    ) {
        self.ensure_list_cache(catalog);
        // The dropdown offers no "(none)": default to General (first listed
        // otherwise) until the user picks a program; the choice then persists.
        // Programs manages its own selection in the left list — auto-picking
        // would fill the name field and make New clone the default program.
        if self.selected_program.is_none() && !matches!(self.tab, EditorTab::Programs) {
            if let Some(name) = default_editor_program(catalog) {
                self.select_program(&name, catalog, settings);
            }
        }
        let names = self.list_cache.program_names.clone();
        // Per-tab model count appended to each option ("Demo Program (3)");
        // Programs has no entity kind to count.
        let counts: HashMap<String, usize> = if matches!(self.tab, EditorTab::Programs) {
            HashMap::new()
        } else {
            names
                .iter()
                .map(|n| (n.clone(), self.tab_model_count(catalog, settings, n)))
                .collect()
        };
        let mut option_label = |name: &str| match counts.get(name) {
            Some(count) => format!("{name} ({count})"),
            None => name.to_string(),
        };
        let mut current = self.selected_program.clone().unwrap_or_default();
        let before = current.clone();
        ui.horizontal(|ui| {
            crate::action_tooltip::help::label(ui, crate::model_icons::PROGRAM, "Program");
            let mut option_icon =
                |ctx: &egui::Context, name: &str| icons.for_program(ctx, catalog, name);
            crate::action_tooltip::help::tip(
                searchable_combo(
                    ui,
                    "data_editor_program",
                    &mut current,
                    crate::widgets::SearchableComboOpts {
                        options: &names,
                        empty_text: sqyre_domain::EMPTY_NONE,
                        none_label: None,
                        width: None,
                        on_option_hover: None,
                        option_icon: Some(&mut option_icon),
                        option_label: Some(&mut option_label),
                    },
                ),
                crate::action_tooltip::help::DE_PROGRAM,
            );
        });
        if !current.is_empty() && current != before {
            self.select_program(&current, catalog, settings);
        }
    }
}

/// Scroll the armed selection row/header into view once (shared scroll flag).
fn list_scroll_to(
    ui: &mut egui::Ui,
    resp: &egui::Response,
    scroll_to: &Cell<bool>,
    did_scroll: &Cell<bool>,
) {
    if scroll_to.get() && !did_scroll.get() {
        ui.scroll_to_rect(resp.rect, Some(egui::Align::Center));
        scroll_to.set(false);
        did_scroll.set(true);
    }
}

/// Entity rows for one program; returns the number painted.
#[expect(
    clippy::too_many_arguments,
    reason = "row painter: program entities plus click/scroll/delete outputs"
)]
fn entity_rows(
    ui: &mut egui::Ui,
    tab: EditorTab,
    kind: Option<PreviewKind>,
    catalog: &ProgramCatalog,
    icons: &mut IconCache,
    previews: &mut PreviewTooltipCache,
    prog: &str,
    entities: &[String],
    prog_selected: bool,
    selected_entity: Option<&str>,
    q: &str,
    prog_match: bool,
    scroll_to: &Cell<bool>,
    did_scroll: &Cell<bool>,
    clicked_entity: &mut Option<(String, String)>,
    delete_target: &mut Option<(String, Option<String>)>,
) -> usize {
    let mut visible = 0;
    for ent in entities {
        if !q.is_empty() && !pickers::fuzzy_match_fold(q, ent) && !prog_match {
            continue;
        }
        visible += 1;
        let selected = prog_selected && selected_entity == Some(ent.as_str());
        let resp = entity_row(ui, tab, kind, catalog, icons, previews, prog, ent, selected);
        if selected {
            list_scroll_to(ui, &resp, scroll_to, did_scroll);
        }
        if resp.clicked() {
            *clicked_entity = Some((prog.to_string(), ent.clone()));
        }
        if crate::widgets::response_danger_menu(ui, &resp, "Delete") {
            *delete_target = Some((prog.to_string(), Some(ent.clone())));
        }
    }
    visible
}

/// One entity row: selectable label + preview/file hover for the tab's kind.
#[expect(
    clippy::too_many_arguments,
    reason = "row painter: preview kind plus catalog/icon/hover context"
)]
fn entity_row(
    ui: &mut egui::Ui,
    tab: EditorTab,
    kind: Option<PreviewKind>,
    catalog: &ProgramCatalog,
    icons: &mut IconCache,
    previews: &mut PreviewTooltipCache,
    prog: &str,
    ent: &str,
    selected: bool,
) -> egui::Response {
    let resp = ui.selectable_label(selected, ent);
    if let Some(kind) = kind {
        previews.show_for_entity(ui, &resp, catalog, prog, ent, kind);
    } else if matches!(tab, EditorTab::Masks) {
        show_file_hover(
            ui,
            &resp,
            icons,
            &catalog.mask_image_path(prog, ent),
            &format!("{prog}~{ent}"),
        );
    }
    resp
}

/// Collapsible program header (All programs scope) wrapping `body` rows.
///
/// `body` receives the forwarded `icons` — row painters need it too, so it
/// cannot be borrowed by both the header and the rows at once.
#[expect(
    clippy::too_many_arguments,
    reason = "shared group chrome: collapse state, scroll arm, and click output"
)]
fn program_group_section(
    ui: &mut egui::Ui,
    tab: EditorTab,
    catalog: &ProgramCatalog,
    icons: &mut IconCache,
    compact_headers: bool,
    prog: &str,
    child_count: usize,
    prog_selected: bool,
    scroll_to: &Cell<bool>,
    did_scroll: &Cell<bool>,
    clicked_program: &mut Option<String>,
    body: impl FnOnce(&mut egui::Ui, &mut IconCache),
) {
    let id = data_editor_list_collapse_id(tab, prog);
    let mut state =
        egui::collapsing_header::CollapsingState::load_with_default_open(ui.ctx(), id, false);
    if scroll_to.get() && prog_selected && !state.is_open() {
        state.set_open(true);
        state.store(ui.ctx());
    }
    state
        .show_header(ui, |ui| {
            let resp = crate::icon_cache::paint_program_label(
                ui,
                catalog,
                icons,
                prog,
                crate::icon_cache::ProgramLabelStyle::Header {
                    selected: Some(prog_selected),
                    child_count,
                },
                compact_headers,
            );
            if prog_selected {
                list_scroll_to(ui, &resp, scroll_to, did_scroll);
            }
            if resp.clicked() {
                *clicked_program = Some(prog.to_string());
            }
        })
        .body(|ui| {
            ui.set_max_width(ui.available_width());
            body(ui, icons);
        });
}

/// Overlay button rows for one program (thumb, name, context menu).
/// Returns the number of rows painted.
#[expect(
    clippy::too_many_arguments,
    reason = "row painter: per-program buttons plus click/enabled/delete outputs"
)]
fn overlay_button_rows(
    ui: &mut egui::Ui,
    prog: &str,
    buttons: &[OverlayButtonConfig],
    live_preview: Option<&OverlayButtonConfig>,
    prog_selected: bool,
    selected_entity: Option<&str>,
    q: &str,
    prog_match: bool,
    scroll_to: &Cell<bool>,
    did_scroll: &Cell<bool>,
    clicked_overlay: &mut Option<(String, String)>,
    enabled_updates: &mut Vec<(String, bool)>,
    delete_target: &mut Option<(String, Option<String>)>,
) -> usize {
    let mut visible = 0;
    for btn in buttons {
        let display_name = btn.display_name();
        if !q.is_empty()
            && !pickers::fuzzy_match_fold(q, display_name)
            && !pickers::fuzzy_match_fold(q, &btn.id)
            && !prog_match
        {
            continue;
        }
        visible += 1;
        let selected = prog_selected && selected_entity == Some(btn.id.as_str());
        let mut row_anchor = None;
        let row = ui.horizontal(|ui| {
            let paint = live_preview.filter(|p| p.id == btn.id).unwrap_or(btn);
            let thumb = paint_overlay_list_thumb(ui, paint);
            let mut name = egui::RichText::new(paint.display_name());
            if !btn.enabled {
                name = name.weak();
            }
            let mut label = ui.selectable_label(selected, name);
            if !btn.enabled {
                label = label.on_hover_text(ht::ED_ROW_DISABLED);
            }
            if selected {
                list_scroll_to(ui, &label, scroll_to, did_scroll);
            }
            if thumb.clicked() || label.clicked() {
                *clicked_overlay = Some((prog.to_string(), btn.id.clone()));
            }
            row_anchor = Some(label);
        });
        let menu_id = egui::Id::new(("overlay_row_menu", &btn.id));
        crate::widgets::rect_context_menu(
            ui,
            menu_id,
            row.response.rect,
            row_anchor.as_ref(),
            |ui| {
                let mut enabled = btn.enabled;
                if ui
                    .checkbox(&mut enabled, "Enabled")
                    .on_hover_text(help::DE_OVERLAY_ENABLED)
                    .changed()
                {
                    enabled_updates.push((btn.id.clone(), enabled));
                }
                if crate::widgets::menu_item_danger(ui, "Delete", true) {
                    *delete_target = Some((prog.to_string(), Some(btn.id.clone())));
                }
            },
        );
    }
    visible
}

fn data_editor_list_collapse_id(tab: EditorTab, program: &str) -> egui::Id {
    egui::Id::new(("data_editor_list", tab_collapse_key(tab), program))
}

/// Miniature overlay button using the configured fill, border, radius, and icon colors.
fn paint_overlay_list_thumb(ui: &mut egui::Ui, btn: &OverlayButtonConfig) -> egui::Response {
    let icon = overlay_icons::resolve(&btn.icon);
    let thumb = ui
        .spacing()
        .interact_size
        .y
        .max(crate::icon_cache::PROCESS_ICON_SIDE);
    let mut style = overlay_icons::OverlayPaintStyle::from_config(btn);
    let k = thumb / btn.size.max(1.0);
    style.corner_radius *= k;
    style.border_width *= k;
    overlay_icons::style_preview_button(ui, icon, thumb, &style)
}

fn tab_collapse_key(tab: EditorTab) -> &'static str {
    match tab {
        EditorTab::Programs => "programs",
        EditorTab::Items => "items",
        EditorTab::PixelCheck => "pixel_check",
        EditorTab::Points => "points",
        EditorTab::SearchAreas => "search_areas",
        EditorTab::Masks => "masks",
        EditorTab::Collections => "collections",
        EditorTab::Atlases => "atlases",
        EditorTab::ScreenCap => "screencap",
        EditorTab::Overlay => "overlay",
    }
}

fn compute_entity_names(catalog: &ProgramCatalog, tab: EditorTab, program: &str) -> Vec<String> {
    let Some(p) = catalog.get(program) else {
        return Vec::new();
    };
    let res = catalog.resolution_key();
    let mut keys: Vec<(String, String)> = match tab {
        EditorTab::Items | EditorTab::PixelCheck => p
            .items
            .iter()
            .map(|(k, it)| {
                let display = if it.name.trim().is_empty() {
                    k.clone()
                } else {
                    it.name.clone()
                };
                (k.clone(), display)
            })
            .collect(),
        EditorTab::Points => p
            .points
            .get(res)
            .or_else(|| p.points.values().next())
            .map(|m| {
                m.iter()
                    .map(|(k, pt)| {
                        let display = if pt.name.trim().is_empty() {
                            k.clone()
                        } else {
                            pt.name.clone()
                        };
                        (k.clone(), display)
                    })
                    .collect()
            })
            .unwrap_or_default(),
        EditorTab::SearchAreas | EditorTab::ScreenCap => p
            .search_areas
            .get(res)
            .or_else(|| p.search_areas.values().next())
            .map(|m| {
                m.iter()
                    .map(|(k, sa)| {
                        let display = if sa.name.trim().is_empty() {
                            k.clone()
                        } else {
                            sa.name.clone()
                        };
                        (k.clone(), display)
                    })
                    .collect()
            })
            .unwrap_or_default(),
        EditorTab::Masks => p
            .masks
            .iter()
            .map(|(k, m)| {
                let display = if m.name.trim().is_empty() {
                    k.clone()
                } else {
                    m.name.clone()
                };
                (k.clone(), display)
            })
            .collect(),
        EditorTab::Collections => p
            .collections
            .iter()
            .map(|(k, c)| {
                let display = if c.name.trim().is_empty() {
                    k.clone()
                } else {
                    c.name.clone()
                };
                (k.clone(), display)
            })
            .collect(),
        EditorTab::Atlases => p
            .atlases
            .iter()
            .map(|(k, a)| {
                let display = if a.name.trim().is_empty() {
                    k.clone()
                } else {
                    a.name.clone()
                };
                (k.clone(), display)
            })
            .collect(),
        EditorTab::Programs | EditorTab::Overlay => Vec::new(),
    };
    keys.sort_by(|a, b| {
        a.1.to_ascii_lowercase()
            .cmp(&b.1.to_ascii_lowercase())
            .then_with(|| a.0.cmp(&b.0))
    });
    keys.into_iter().map(|(k, _)| k).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqyre_domain::{ScalarValue, TEMPORARY_PROGRAM};
    use sqyre_persist::{
        ProgramCatalog, ProgramCollection, ProgramItem, ProgramMask, ProgramPoint,
    };

    fn catalog() -> ProgramCatalog {
        let mut cat = ProgramCatalog::default();
        cat.set_resolution_key("1920x1080");
        cat.create_program("Game").unwrap();
        cat
    }

    #[test]
    fn tab_collapse_key_unique_per_tab() {
        let keys: std::collections::HashSet<&'static str> = [
            EditorTab::Programs,
            EditorTab::Items,
            EditorTab::PixelCheck,
            EditorTab::Points,
            EditorTab::SearchAreas,
            EditorTab::Masks,
            EditorTab::Collections,
            EditorTab::Atlases,
            EditorTab::ScreenCap,
            EditorTab::Overlay,
        ]
        .into_iter()
        .map(tab_collapse_key)
        .collect();
        assert_eq!(keys.len(), 10, "collapse keys must be unique per tab");
    }

    #[test]
    fn compute_entity_names_sorts_by_display_name() {
        let mut cat = catalog();
        // Keys chosen so key order differs from display-name order.
        for (key, display) in [("zz", "Alpha"), ("aa", "Beta")] {
            let data = cat.programs_mut().get_mut("Game").unwrap();
            data.items.insert(
                key.to_string(),
                ProgramItem {
                    name: display.into(),
                    ..Default::default()
                },
            );
        }
        let names = compute_entity_names(&cat, EditorTab::Items, "Game");
        assert_eq!(
            names,
            vec!["zz", "aa"],
            "sorted by display name, keys returned"
        );
        // Unknown program → empty.
        assert!(compute_entity_names(&cat, EditorTab::Items, "Nope").is_empty());
        // Programs / Overlay tabs list no entities.
        assert!(compute_entity_names(&cat, EditorTab::Programs, "Game").is_empty());
        assert!(compute_entity_names(&cat, EditorTab::Overlay, "Game").is_empty());
    }

    #[test]
    fn compute_entity_names_falls_back_to_key_when_display_blank() {
        let mut cat = catalog();
        // Blank display names are impossible via the validate-upsert path, but
        // can exist in catalogs loaded from older/edited YAML.
        cat.programs_mut().get_mut("Game").unwrap().masks.insert(
            "maskkey".into(),
            ProgramMask {
                name: " ".into(),
                ..Default::default()
            },
        );
        cat.upsert_mask(
            "Game",
            ProgramMask {
                name: "named".into(),
                ..Default::default()
            },
        )
        .unwrap();
        let names = compute_entity_names(&cat, EditorTab::Masks, "Game");
        assert_eq!(names.len(), 2);
        assert!(names.contains(&"maskkey".to_string()));
        assert!(names.contains(&"named".to_string()));
    }

    #[test]
    fn compute_entity_names_per_tab() {
        let mut cat = catalog();
        cat.upsert_point(
            "Game",
            ProgramPoint {
                name: "Spawn".into(),
                monitor: 1,
                x: ScalarValue::Int(1),
                y: ScalarValue::Int(2),
            },
        )
        .unwrap();
        cat.upsert_collection(
            "Game",
            ProgramCollection {
                name: "bag".into(),
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(
            compute_entity_names(&cat, EditorTab::Points, "Game"),
            vec!["Spawn"]
        );
        assert_eq!(
            compute_entity_names(&cat, EditorTab::Collections, "Game"),
            vec!["bag"]
        );
        // PixelCheck shares the items list.
        assert_eq!(
            compute_entity_names(&cat, EditorTab::PixelCheck, "Game"),
            compute_entity_names(&cat, EditorTab::Items, "Game")
        );
    }

    #[test]
    fn list_cache_refreshes_on_generation_and_tab() {
        let mut cat = catalog();
        cat.upsert_item(
            "Game",
            ProgramItem {
                name: "Potion".into(),
                ..Default::default()
            },
        )
        .unwrap();
        let mut ed = DataEditor {
            tab: EditorTab::Items,
            ..Default::default()
        };

        assert_eq!(ed.entity_names(&cat, "Game"), vec!["Potion"]);
        let gen0 = ed.list_cache.catalog_generation;

        // Mutating the catalog bumps generation → cache rebuild sees the new item.
        cat.upsert_item(
            "Game",
            ProgramItem {
                name: "Elixir".into(),
                ..Default::default()
            },
        )
        .unwrap();
        let names = ed.entity_names(&cat, "Game");
        assert_eq!(names, vec!["Elixir", "Potion"]);
        assert!(ed.list_cache.catalog_generation > gen0);

        // Switching tabs rebuilds the entity view.
        ed.tab = EditorTab::Masks;
        assert!(ed.entity_names(&cat, "Game").is_empty());
        assert_eq!(ed.list_cache.tab, EditorTab::Masks);
    }

    #[test]
    fn tab_model_count_matches_current_tab_models() {
        let mut cat = catalog();
        cat.upsert_item(
            "Game",
            ProgramItem {
                name: "Potion".into(),
                ..Default::default()
            },
        )
        .unwrap();
        cat.upsert_item(
            "Game",
            ProgramItem {
                name: "Elixir".into(),
                ..Default::default()
            },
        )
        .unwrap();
        cat.upsert_point(
            "Game",
            ProgramPoint {
                name: "Spawn".into(),
                monitor: 1,
                x: ScalarValue::Int(1),
                y: ScalarValue::Int(2),
            },
        )
        .unwrap();
        cat.upsert_collection(
            "Game",
            ProgramCollection {
                name: "bag".into(),
                ..Default::default()
            },
        )
        .unwrap();
        cat.upsert_search_area(
            "Game",
            sqyre_persist::ProgramSearchArea {
                name: "hud".into(),
                monitor: 1,
                left_x: ScalarValue::Int(0),
                top_y: ScalarValue::Int(0),
                right_x: ScalarValue::Int(100),
                bottom_y: ScalarValue::Int(100),
            },
        )
        .unwrap();
        let settings = UserSettings {
            overlay_buttons: vec![
                OverlayButtonConfig::new("b1", "Game"),
                OverlayButtonConfig::new("b2", "Game"),
                OverlayButtonConfig::new("b3", "Other"),
            ],
            ..Default::default()
        };
        let mut ed = DataEditor {
            tab: EditorTab::Items,
            ..Default::default()
        };

        assert_eq!(ed.tab_model_count(&cat, &settings, "Game"), 2);
        ed.tab = EditorTab::Points;
        assert_eq!(ed.tab_model_count(&cat, &settings, "Game"), 1);
        // Screen Cap counts search areas and collections together.
        ed.tab = EditorTab::ScreenCap;
        assert_eq!(ed.tab_model_count(&cat, &settings, "Game"), 2);
        ed.tab = EditorTab::Overlay;
        assert_eq!(ed.tab_model_count(&cat, &settings, "Game"), 2);
        ed.tab = EditorTab::Programs;
        assert_eq!(ed.tab_model_count(&cat, &settings, "Game"), 0);
    }

    #[test]
    fn select_program_and_entity_reject_temporary_program() {
        let mut cat = catalog();
        cat.create_program(TEMPORARY_PROGRAM).unwrap();
        let settings = UserSettings::default();
        let mut ed = DataEditor::default();

        ed.select_program(TEMPORARY_PROGRAM, &cat, &settings);
        assert!(ed.selected_program.is_none());

        ed.select_program("Game", &cat, &settings);
        assert_eq!(ed.selected_program.as_deref(), Some("Game"));
        // Selecting a program clears the entity selection.
        ed.selected_entity = Some("old".into());
        ed.select_program("Game", &cat, &settings);
        assert!(ed.selected_entity.is_none());

        ed.select_entity(TEMPORARY_PROGRAM, "x", &cat, &settings);
        assert_eq!(ed.selected_program.as_deref(), Some("Game"));

        ed.select_entity("Game", "Spawn", &cat, &settings);
        assert_eq!(ed.selected_entity.as_deref(), Some("Spawn"));
    }
}
