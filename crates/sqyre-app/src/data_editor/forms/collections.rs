//! Collection and Atlas form bodies.
//!
//! Split out of `draw_form`; see `forms.rs` for the tab dispatch.

use super::super::DataEditor;
use super::*;

impl DataEditor {
    pub(super) fn draw_collections_form(
        &mut self,
        ui: &mut egui::Ui,
        paint: &mut CatalogPaint<'_>,
    ) {
        let CatalogPaint {
            catalog,
            icons,
            previews,
            ..
        } = paint;
        let collection_area = self.selected_program.as_deref().and_then(|prog| {
            let sa = self.form_search_area.trim();
            if sa.is_empty() {
                return None;
            }
            catalog_search_area_pixel_size(catalog, prog, sa)
        });
        let rows = parse_i32(&self.form_rows).unwrap_or(1).max(1);
        let cols = parse_i32(&self.form_cols).unwrap_or(1).max(1);
        {
            // Collection name == search-area name; offer areas that are free or
            // the currently selected collection (so retarget/rename still works).
            let selected = self.selected_entity.clone();
            let areas: Vec<String> = self
                .selected_program
                .as_deref()
                .map(|p| {
                    let res = catalog.resolution_key();
                    let Some(prog) = catalog.get(p) else {
                        return Vec::new();
                    };
                    let keys = prog
                        .search_areas
                        .get(res)
                        .or_else(|| prog.search_areas.values().next())
                        .map(|m| m.keys().cloned().collect::<Vec<_>>())
                        .unwrap_or_default();
                    keys.into_iter()
                        .filter(|sa| {
                            selected.as_deref() == Some(sa.as_str())
                                || !prog.collections.contains_key(sa)
                        })
                        .collect()
                })
                .unwrap_or_default();
            let prog = self.selected_program.clone();
            let mut on_hover = |ui: &mut egui::Ui, resp: &egui::Response, name: &str| {
                if name.is_empty() {
                    return;
                }
                let Some(prog) = prog.as_deref() else {
                    return;
                };
                previews.show_for_entity(ui, resp, catalog, prog, name, PreviewKind::SearchArea);
            };
            let _ = crate::widgets::labeled_searchable_combo(
                ui,
                crate::model_icons::SEARCH_AREA,
                "Search area",
                help::DE_COLLECTION_AREA,
                "collection_sa",
                &mut self.form_search_area,
                SearchableComboOpts {
                    options: &areas,
                    empty_text: sqyre_domain::EMPTY_NONE,
                    none_label: None,
                    width: None,
                    on_option_hover: Some(&mut on_hover),
                    option_icon: None,
                    closed_icon: true,
                    option_label: None,
                },
                None,
            );
        }
        crate::widgets::wrapped_row(ui, |ui| {
            crate::widgets::num_text_field_opts(
                ui,
                egui_phosphor::regular::ROWS,
                "Rows",
                help::DE_COLLECTION_ROWS,
                &mut self.form_rows,
                crate::widgets::NumFieldOpts {
                    width: 80.0,
                    range: 1..=i32::MAX,
                },
            );
            crate::widgets::num_text_field_opts(
                ui,
                egui_phosphor::regular::COLUMNS,
                "Cols",
                help::DE_COLLECTION_COLS,
                &mut self.form_cols,
                crate::widgets::NumFieldOpts {
                    width: 80.0,
                    range: 1..=i32::MAX,
                },
            );
        });
        if let (Some(prog), Some(col_name)) =
            (self.selected_program.clone(), self.selected_entity.clone())
        {
            let path = catalog.collection_image_path(&prog, &col_name);
            let key = (prog.clone(), col_name.clone());
            if self.collection_preview_key.as_ref() != Some(&key) {
                self.collection_preview.reset();
                self.collection_preview_key = Some(key);
            }
            let mut replace = false;
            let capturing = self.collection_capture_pending.is_some();
            paint_zoomable_collection_preview(
                ui,
                icons,
                path.as_path(),
                rows,
                cols,
                collection_area,
                &mut self.collection_preview,
                &mut replace,
                capturing,
            );
            if replace && !capturing {
                let col = ProgramCollection {
                    name: col_name.clone(),
                    rows,
                    cols,
                };
                if let Err(e) = self.start_collection_capture(catalog, &prog, &col, None) {
                    self.set_err(e);
                }
            }
        }
    }

    pub(super) fn draw_atlases_form(&mut self, ui: &mut egui::Ui, paint: &mut CatalogPaint<'_>) {
        let CatalogPaint {
            catalog,
            icons,
            previews,
            ..
        } = paint;
        paint_name_field(ui, &mut self.form_name);
        let available: Vec<String> = self
            .selected_program
            .as_deref()
            .and_then(|p| catalog.get(p))
            .map(|prog| {
                prog.collections
                    .keys()
                    .filter(|k| !self.form_atlas_members.iter().any(|m| m == *k))
                    .cloned()
                    .collect()
            })
            .unwrap_or_default();
        {
            let prog = self.selected_program.clone();
            let mut on_hover = |ui: &mut egui::Ui, resp: &egui::Response, name: &str| {
                if name.is_empty() {
                    return;
                }
                let Some(prog) = prog.as_deref() else {
                    return;
                };
                previews.show_for_entity(ui, resp, catalog, prog, name, PreviewKind::Collection);
            };
            let can_add = !self.form_atlas_add.trim().is_empty();
            let mut add_clicked = false;
            let mut trailing = |ui: &mut egui::Ui| {
                if crate::widgets::add_button(ui, "Add collection", can_add).clicked() {
                    add_clicked = true;
                }
            };
            let _ = crate::widgets::labeled_searchable_combo(
                ui,
                crate::model_icons::COLLECTION,
                "Collections",
                help::DE_ATLAS_MEMBERS,
                "atlas_add_member",
                &mut self.form_atlas_add,
                SearchableComboOpts {
                    options: &available,
                    empty_text: "(add collection)",
                    none_label: None,
                    width: None,
                    on_option_hover: Some(&mut on_hover),
                    option_icon: None,
                    closed_icon: true,
                    option_label: None,
                },
                Some(&mut trailing),
            );
            if add_clicked {
                let name = self.form_atlas_add.trim().to_string();
                if !name.is_empty() && !self.form_atlas_members.iter().any(|m| m == &name) {
                    self.form_atlas_members.push(name);
                }
                self.form_atlas_add.clear();
            }
        }
        let mut remove_at: Option<usize> = None;
        for (i, member) in self.form_atlas_members.iter().enumerate() {
            let label = ui.label(format!("• {member}"));
            if let Some(prog) = self.selected_program.as_deref() {
                previews.show_for_entity(
                    ui,
                    &label,
                    catalog,
                    prog,
                    member,
                    PreviewKind::Collection,
                );
            }
            let menu_id = egui::Id::new(("atlas_member_menu", member));
            if crate::widgets::row_danger_menu(ui, menu_id, label.rect, Some(&label), "Remove") {
                remove_at = Some(i);
            }
        }
        if let Some(i) = remove_at {
            self.form_atlas_members.remove(i);
        }
        if let (Some(prog), Some(atlas_name)) =
            (self.selected_program.clone(), self.selected_entity.clone())
        {
            let key = (prog.clone(), atlas_name.clone());
            if self.atlas_preview_key.as_ref() != Some(&key) {
                self.atlas_preview.reset();
                self.atlas_preview_key = Some(key);
            }
            paint_zoomable_atlas_preview(
                ui,
                icons,
                catalog,
                &prog,
                &self.form_atlas_members,
                &mut self.atlas_preview,
            );
        } else if !self.form_atlas_members.is_empty() {
            if let Some(prog) = self.selected_program.clone() {
                paint_zoomable_atlas_preview(
                    ui,
                    icons,
                    catalog,
                    &prog,
                    &self.form_atlas_members,
                    &mut self.atlas_preview,
                );
            }
        }
    }
}
