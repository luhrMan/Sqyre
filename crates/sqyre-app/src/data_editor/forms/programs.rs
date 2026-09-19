//! Program and Item form bodies.
//!
//! Split out of `draw_form`; see `forms.rs` for the tab dispatch.

use super::super::DataEditor;
use super::*;

impl DataEditor {
    pub(super) fn draw_programs_form(
        &mut self,
        ui: &mut egui::Ui,
        paint: &mut CatalogPaint<'_>,
        ctx: FormCtx<'_>,
    ) -> bool {
        let CatalogPaint { catalog, icons, .. } = paint;
        let FormCtx { macros, .. } = ctx;
        ui.horizontal(|ui| {
            if let Some(name) = self.selected_program.as_deref() {
                crate::icon_cache::paint_program_icon(ui, catalog, icons, name);
            }
            ui.heading("Program");
        });
        help::label(ui, "Name", help::DE_NAME);
        ui.add(egui::TextEdit::singleline(&mut self.form_name).desired_width(f32::INFINITY));
        paint_fs_name_hint(ui, &self.form_name);
        ui.add_space(8.0);
        help::label(ui, "Running program", help::DE_RUNNING_PROGRAM);
        ui.add_space(4.0);
        let bound = if self.form_process_path.trim().is_empty() {
            "(none)".to_string()
        } else if self.form_window_title.trim().is_empty() {
            self.form_process_path.clone()
        } else {
            format!(
                "{}  —  {}",
                self.form_window_title.trim(),
                self.form_process_path.trim()
            )
        };
        ui.horizontal(|ui| {
            if !self.form_process_path.trim().is_empty() {
                let tex = icons
                    .for_process(ui.ctx(), &self.form_process_path, &self.form_window_title)
                    .or_else(|| {
                        let prog = self.selected_program.as_deref()?;
                        let bound = catalog.get(prog)?;
                        if bound.process_path.trim() != self.form_process_path.trim() {
                            return None;
                        }
                        icons.for_path(ui.ctx(), &catalog.process_icon_path(prog))
                    });
                if let Some(tex) = tex {
                    crate::icon_cache::paint_process_icon(
                        ui,
                        &tex,
                        crate::icon_cache::PROCESS_ICON_SIDE * 1.25,
                    );
                }
            }
            ui.label(egui::RichText::new(bound).monospace());
        });
        ui.horizontal(|ui| {
            if ui.button("Select…").clicked() {
                self.window_picker =
                    pickers::open_window_picker(&self.form_process_path, &self.form_window_title);
            }
            if ui
                .add_enabled(
                    !self.form_process_path.is_empty() || !self.form_window_title.is_empty(),
                    egui::Button::new("Clear"),
                )
                .clicked()
            {
                self.form_process_path.clear();
                self.form_window_title.clear();
            }
        });
        ui.add_space(8.0);
        help::label(ui, "Macro tags", help::DE_PROGRAM_MACRO_TAGS);
        let macro_tag_completions = crate::macro_meta::collect_all_macro_tags(macros);
        crate::widgets::tag_chip_editor(
            ui,
            &mut self.form_tags,
            &mut self.tag_draft,
            &macro_tag_completions,
            crate::widgets::TagChipOptions::default(),
        )
        .submitted
    }

    pub(super) fn draw_items_form(
        &mut self,
        ui: &mut egui::Ui,
        paint: &mut CatalogPaint<'_>,
        settings: &mut UserSettings,
    ) -> bool {
        let CatalogPaint { catalog, icons, .. } = paint;
        ui.heading("Item");
        self.program_selector(ui, catalog, icons, settings);
        ui.add_space(4.0);
        help::label(ui, "Name", help::DE_NAME);
        ui.add(egui::TextEdit::singleline(&mut self.form_name).desired_width(f32::INFINITY));
        paint_fs_name_hint(ui, &self.form_name);
        ui.add_space(4.0);
        let tag_submit = self.paint_item_param_fields(ui, catalog, icons);
        if let (Some(prog), Some(item)) =
            (self.selected_program.clone(), self.selected_entity.clone())
        {
            let target = format!("{prog}{PROGRAM_DELIMITER}{item}");
            self.paint_item_variants_ui(ui, icons, catalog, settings, &target, &item);
        }
        tag_submit
    }

    /// Tags, grid footprint, stack max, and mask — shared by Items and ScreenCap New Item.
    pub(super) fn paint_item_param_fields(
        &mut self,
        ui: &mut egui::Ui,
        catalog: &ProgramCatalog,
        icons: &mut crate::icon_cache::IconCache,
    ) -> bool {
        help::label(ui, "Tags", help::DE_TAGS);
        let program_tags = self
            .selected_program
            .as_deref()
            .map(|prog| collect_program_item_tags(catalog, prog))
            .unwrap_or_default();
        let tag_submit = crate::widgets::tag_chip_editor(
            ui,
            &mut self.form_tags,
            &mut self.tag_draft,
            &program_tags,
            crate::widgets::TagChipOptions::default(),
        )
        .submitted;
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            help::label(ui, "Cols", help::DE_COLS);
            ui.add(egui::TextEdit::singleline(&mut self.form_cols).desired_width(80.0));
            help::label(ui, "Rows", help::DE_ROWS);
            ui.add(egui::TextEdit::singleline(&mut self.form_rows).desired_width(80.0));
            help::label(ui, "Stack max", help::DE_STACK_MAX);
            ui.add(egui::TextEdit::singleline(&mut self.form_stack_max).desired_width(80.0));
        });
        ui.add_space(4.0);
        help::label(ui, "Mask", help::DE_MASK);
        let masks: Vec<String> = self
            .selected_program
            .as_deref()
            .and_then(|p| catalog.get(p))
            .map(|p| p.masks.keys().cloned().collect())
            .unwrap_or_default();
        let mut current = self.form_mask.clone();
        let prog = self.selected_program.clone();
        let mut on_hover = |ui: &mut egui::Ui, resp: &egui::Response, name: &str| {
            if name.is_empty() {
                return;
            }
            let Some(prog) = prog.as_deref() else {
                return;
            };
            show_file_hover(
                ui,
                resp,
                icons,
                &catalog.mask_image_path(prog, name),
                &format!("{prog}~{name}"),
            );
        };
        searchable_combo_with(
            ui,
            "item_mask",
            &mut current,
            &masks,
            "(none)",
            Some("(none)"),
            None,
            Some(&mut on_hover),
            None,
        );
        if current != self.form_mask {
            self.form_mask = current;
        }
        if let (Some(prog), mask) = (self.selected_program.as_deref(), self.form_mask.as_str()) {
            if !mask.is_empty() {
                if let Some(m) = catalog.get(prog).and_then(|p| p.masks.get(mask)) {
                    let detail = if catalog.mask_image_path(prog, mask).is_file() {
                        "Image mask on disk".to_string()
                    } else if m.shape == sqyre_domain::MaskShape::Circle {
                        format!("Circle @ ({}, {}) r={}", m.center_x, m.center_y, m.radius)
                    } else {
                        format!(
                            "Rectangle @ ({}, {}) {}×{}",
                            m.center_x, m.center_y, m.base, m.height
                        )
                    };
                    ui.weak(detail);
                }
            }
        }
        tag_submit
    }
}
