//! Left entity list + program selector (cached against catalog generation).

use super::{DataEditor, EditorTab, ListCache};
use crate::data_editor_preview::show_file_hover;
use crate::icon_cache::IconCache;
use crate::preview_tooltip::{PreviewKind, PreviewTooltipCache};
use eframe::egui;
use sqyre_persist::{OverlayButtonConfig, ProgramCatalog, UserSettings};
use std::collections::HashMap;

impl DataEditor {
    pub(crate) fn draw_left_list(
        &mut self,
        ui: &mut egui::Ui,
        catalog: &ProgramCatalog,
        icons: &mut IconCache,
        previews: &mut PreviewTooltipCache,
        settings: &UserSettings,
    ) {
        ui.horizontal(|ui| {
            ui.label("Search");
            ui.add(
                egui::TextEdit::singleline(&mut self.search).desired_width(f32::INFINITY),
            );
        });
        ui.separator();
        let q = self.search.trim().to_string();
        // Remaining height in the fixed left pane — scroll lists must not grow the window.
        let list_h = ui.available_height().max(40.0);
        match self.tab {
            EditorTab::Programs => {
                egui::ScrollArea::vertical()
                    .id_salt("data_editor_programs")
                    .auto_shrink([false, false])
                    .max_height(list_h)
                    .show(ui, |ui| {
                        for name in catalog.program_names() {
                            if !q.is_empty() && !crate::pickers::fuzzy_match_fold(&q, name) {
                                continue;
                            }
                            let selected = self.selected_program.as_deref() == Some(name.as_str());
                            if ui.selectable_label(selected, name).clicked() {
                                self.select_program(name, catalog, settings);
                            }
                        }
                    });
            }
            EditorTab::Items => {
                let mut selected: Vec<String> = match (
                    self.selected_program.as_deref(),
                    self.selected_entity.as_deref(),
                ) {
                    (Some(p), Some(e)) => {
                        vec![format!("{p}{}{e}", sqyre_domain::PROGRAM_DELIMITER)]
                    }
                    _ => Vec::new(),
                };
                egui::ScrollArea::vertical()
                    .id_salt("data_editor_items")
                    .auto_shrink([false, false])
                    .max_height(list_h)
                    .max_width(ui.available_width())
                    .show(ui, |ui| {
                        ui.set_max_width(ui.available_width());
                        crate::pickers::paint_items_icon_grid(
                            ui,
                            catalog,
                            icons,
                            &self.search,
                            &mut selected,
                            false,
                        );
                    });
                if let Some(target) = selected.first() {
                    if let Some((prog, item)) = target.split_once(sqyre_domain::PROGRAM_DELIMITER) {
                        let changed = self.selected_program.as_deref() != Some(prog)
                            || self.selected_entity.as_deref() != Some(item);
                        if changed {
                            self.select_entity(prog, item, catalog, settings);
                        }
                    }
                }
            }
            EditorTab::Points | EditorTab::SearchAreas | EditorTab::Masks | EditorTab::Collections
            | EditorTab::AutoPic => {
                let kind = match self.tab {
                    EditorTab::Points => Some(PreviewKind::Point),
                    EditorTab::SearchAreas | EditorTab::AutoPic => Some(PreviewKind::SearchArea),
                    _ => None,
                };
                egui::ScrollArea::vertical()
                    .id_salt("data_editor_coords")
                    .auto_shrink([false, false])
                    .max_height(list_h)
                    .show(ui, |ui| {
                        self.ensure_list_cache(catalog);
                        let program_names = self.list_cache.program_names.clone();
                        for prog in &program_names {
                            let entities = self.entity_names(catalog, prog);
                            let prog_match = q.is_empty() || crate::pickers::fuzzy_match_fold(&q, prog);
                            let any_entity = entities
                                .iter()
                                .any(|e| q.is_empty() || crate::pickers::fuzzy_match_fold(&q, e));
                            if !prog_match && !any_entity {
                                continue;
                            }
                            ui.label(
                                egui::RichText::new(prog.as_str()).size(16.0).strong(),
                            );
                            for ent in entities {
                                if !q.is_empty()
                                    && !crate::pickers::fuzzy_match_fold(&q, &ent)
                                    && !prog_match
                                {
                                    continue;
                                }
                                let selected = self.selected_program.as_deref()
                                    == Some(prog.as_str())
                                    && self.selected_entity.as_deref() == Some(ent.as_str());
                                let resp = ui.selectable_label(selected, format!("  {ent}"));
                                if let Some(kind) = kind {
                                    previews.show_for_entity(ui, &resp, catalog, prog, &ent, kind);
                                } else if matches!(self.tab, EditorTab::Masks) {
                                    show_file_hover(
                                        ui,
                                        &resp,
                                        icons,
                                        &catalog.mask_image_path(prog, &ent),
                                        &format!("{prog}~{ent}"),
                                    );
                                } else if matches!(self.tab, EditorTab::Collections) {
                                    show_file_hover(
                                        ui,
                                        &resp,
                                        icons,
                                        &catalog.collection_image_path(prog, &ent),
                                        &format!("{prog}~{ent}"),
                                    );
                                }
                                if resp.clicked() {
                                    self.select_entity(prog, &ent, catalog, settings);
                                }
                            }
                        }
                    });
            }
            EditorTab::Overlay => {
                egui::ScrollArea::vertical()
                    .id_salt("data_editor_overlay_list")
                    .auto_shrink([false, false])
                    .max_height(list_h)
                    .show(ui, |ui| {
                        for prog in catalog.program_names() {
                            let buttons: Vec<&OverlayButtonConfig> = settings
                                .overlay_buttons
                                .iter()
                                .filter(|b| b.program == *prog)
                                .collect();
                            let prog_match =
                                q.is_empty() || crate::pickers::fuzzy_match_fold(&q, prog);
                            let any_btn = buttons.iter().any(|b| {
                                q.is_empty()
                                    || crate::pickers::fuzzy_match_fold(&q, b.display_name())
                                    || crate::pickers::fuzzy_match_fold(&q, &b.id)
                            });
                            if !prog_match && !any_btn {
                                continue;
                            }
                            ui.label(egui::RichText::new(prog.as_str()).size(16.0).strong());
                            for btn in buttons {
                                if !q.is_empty()
                                    && !crate::pickers::fuzzy_match_fold(&q, btn.display_name())
                                    && !crate::pickers::fuzzy_match_fold(&q, &btn.id)
                                    && !prog_match
                                {
                                    continue;
                                }
                                let selected = self.selected_program.as_deref()
                                    == Some(prog.as_str())
                                    && self.selected_entity.as_deref() == Some(btn.id.as_str());
                                if ui
                                    .selectable_label(selected, format!("  {}", btn.display_name()))
                                    .clicked()
                                {
                                    self.select_entity(prog, &btn.id, catalog, settings);
                                }
                            }
                        }
                    });
            }
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
        let program_names: Vec<String> = catalog.program_names().cloned().collect();
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

    pub(crate) fn select_program(&mut self, name: &str, catalog: &ProgramCatalog, settings: &UserSettings) {
        self.selected_program = Some(name.to_string());
        self.selected_entity = None;
        self.load_form(catalog, settings);
    }

    /// Select a program for docs screenshots (Programs tab form populated).
    pub(crate) fn select_program_for_docs(&mut self, name: &str, catalog: &ProgramCatalog) {
        self.select_program(name, catalog, &UserSettings::default());
    }

    pub(crate) fn select_entity(
        &mut self,
        program: &str,
        entity: &str,
        catalog: &ProgramCatalog,
        settings: &UserSettings,
    ) {
        self.selected_program = Some(program.to_string());
        self.selected_entity = Some(entity.to_string());
        self.load_form(catalog, settings);
    }


    pub(crate) fn program_selector(&mut self, ui: &mut egui::Ui, catalog: &ProgramCatalog) {
        self.ensure_list_cache(catalog);
        let names = self.list_cache.program_names.clone();
        let mut current = self.selected_program.clone().unwrap_or_default();
        egui::ComboBox::from_label("Program")
            .selected_text(if current.is_empty() {
                "(none)".into()
            } else {
                current.clone()
            })
            .show_ui(ui, |ui| {
                for n in &names {
                    if ui.selectable_value(&mut current, n.clone(), n).clicked() {
                        self.selected_program = Some(n.clone());
                        self.selected_entity = None;
                        self.reset_item_form();
                        self.form_name.clear();
                    }
                }
            });
        if !current.is_empty() {
            self.selected_program = Some(current);
        }
    }
}

fn compute_entity_names(catalog: &ProgramCatalog, tab: EditorTab, program: &str) -> Vec<String> {
    let Some(p) = catalog.get(program) else {
        return Vec::new();
    };
    let res = catalog.resolution_key();
    let mut keys: Vec<(String, String)> = match tab {
        EditorTab::Items => p
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
        EditorTab::SearchAreas | EditorTab::AutoPic => p
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
        EditorTab::Programs | EditorTab::Overlay => Vec::new(),
    };
    keys.sort_by(|a, b| {
        a.1.to_ascii_lowercase()
            .cmp(&b.1.to_ascii_lowercase())
            .then_with(|| a.0.cmp(&b.0))
    });
    keys.into_iter().map(|(k, _)| k).collect()
}
