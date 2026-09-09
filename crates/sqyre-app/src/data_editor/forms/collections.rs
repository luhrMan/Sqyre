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
        settings: &mut UserSettings,
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
        ui.horizontal(|ui| {
            ui.heading("Collection");
            if let Some(area) = collection_area {
                ui.weak(collection_size_label(area, rows, cols));
            }
        });
        self.program_selector(ui, catalog, icons, settings);
        ui.add_space(4.0);
        ui.label("Name").on_hover_text(help::DE_NAME);
        help::tip(
            ui.add(egui::TextEdit::singleline(&mut self.form_name).desired_width(f32::INFINITY)),
            help::DE_NAME,
        );
        paint_fs_name_hint(ui, &self.form_name);
        ui.add_space(4.0);
        help::label(ui, "Search area", help::DE_COLLECTION_AREA);
        {
            let areas: Vec<String> = self
                .selected_program
                .as_deref()
                .map(|p| {
                    let res = catalog.resolution_key();
                    catalog
                        .get(p)
                        .and_then(|prog| {
                            prog.search_areas
                                .get(res)
                                .or_else(|| prog.search_areas.values().next())
                        })
                        .map(|m| m.keys().cloned().collect())
                        .unwrap_or_default()
                })
                .unwrap_or_default();
            let mut current = self.form_search_area.clone();
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
            searchable_combo_with(
                ui,
                "collection_sa",
                &mut current,
                &areas,
                "(none)",
                None,
                None,
                Some(&mut on_hover),
                None,
            );
            if current != self.form_search_area {
                self.form_search_area = current;
            }
        }
        help::label(ui, "Rows", help::DE_COLLECTION_ROWS);
        help::tip(
            ui.add(egui::TextEdit::singleline(&mut self.form_rows).desired_width(80.0)),
            help::DE_COLLECTION_ROWS,
        );
        help::label(ui, "Cols", help::DE_COLLECTION_COLS);
        help::tip(
            ui.add(egui::TextEdit::singleline(&mut self.form_cols).desired_width(80.0)),
            help::DE_COLLECTION_COLS,
        );
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
                    search_area: self.form_search_area.trim().to_string(),
                    rows,
                    cols,
                };
                if let Err(e) = self.start_collection_capture(catalog, &prog, &col, None) {
                    self.set_err(e);
                }
            }
        }
    }

    pub(super) fn draw_atlases_form(
        &mut self,
        ui: &mut egui::Ui,
        paint: &mut CatalogPaint<'_>,
        settings: &mut UserSettings,
    ) {
        let CatalogPaint {
            catalog,
            icons,
            previews,
            ..
        } = paint;
        ui.heading("Atlas");
        self.program_selector(ui, catalog, icons, settings);
        ui.add_space(4.0);
        ui.label("Name").on_hover_text(help::DE_NAME);
        help::tip(
            ui.add(egui::TextEdit::singleline(&mut self.form_name).desired_width(f32::INFINITY)),
            help::DE_NAME,
        );
        paint_fs_name_hint(ui, &self.form_name);
        ui.add_space(4.0);
        help::label(ui, "Collections", help::DE_ATLAS_MEMBERS);
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
        ui.horizontal(|ui| {
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
            searchable_combo_with(
                ui,
                "atlas_add_member",
                &mut self.form_atlas_add,
                &available,
                "(add collection)",
                None,
                None,
                Some(&mut on_hover),
                None,
            );
            if ui
                .add_enabled(
                    !self.form_atlas_add.trim().is_empty(),
                    egui::Button::new("Add"),
                )
                .clicked()
            {
                let name = self.form_atlas_add.trim().to_string();
                if !name.is_empty() && !self.form_atlas_members.iter().any(|m| m == &name) {
                    self.form_atlas_members.push(name);
                }
                self.form_atlas_add.clear();
            }
        });
        let mut remove_at: Option<usize> = None;
        for (i, member) in self.form_atlas_members.iter().enumerate() {
            ui.horizontal(|ui| {
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
                if theme::icon_button_colored(ui, "×", Some(theme::MACRO_STOP))
                    .on_hover_text("Remove")
                    .clicked()
                {
                    remove_at = Some(i);
                }
            });
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
