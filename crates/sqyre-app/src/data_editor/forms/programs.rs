//! Program and Item form bodies.
//!
//! Split out of `draw_form`; see `forms.rs` for the tab dispatch.

use super::super::DataEditor;
use super::*;
use crate::help_text as ht;

impl DataEditor {
    pub(super) fn draw_programs_form(
        &mut self,
        ui: &mut egui::Ui,
        paint: &mut CatalogPaint<'_>,
        ctx: FormCtx<'_>,
    ) -> bool {
        let CatalogPaint { catalog, icons, .. } = paint;
        let FormCtx { macros, .. } = ctx;
        paint_name_field(ui, &mut self.form_name);
        ui.add_space(crate::theme::element_gap(ui));
        let title = self.form_window_title.trim().to_string();
        let path = self.form_process_path.trim().to_string();
        // Program name is the window title when bound; the path falls back to
        // being the name, with the folder glyph tooltip carrying the full path.
        let name = if path.is_empty() {
            sqyre_domain::EMPTY_NONE.to_string()
        } else if title.is_empty() {
            path.clone()
        } else {
            title
        };
        let has_bound = !path.is_empty() || !name.is_empty();
        ui.horizontal(|ui| {
            help::label(ui, crate::model_icons::PROGRAM, "Running program");
            // The icon IS the selector — click the program's own icon (or the
            // window glyph when unbound) to pick a window.
            let icon_side = crate::icon_cache::PROCESS_ICON_SIDE * 1.25;
            let pick = if !path.is_empty() {
                let tex = icons
                    .for_process(ui.ctx(), &self.form_process_path, &self.form_window_title)
                    .or_else(|| {
                        let prog = self.selected_program.as_deref()?;
                        let bound = catalog.get(prog)?;
                        if bound.process_path.trim() != path {
                            return None;
                        }
                        icons.for_path(ui.ctx(), &catalog.process_icon_path(prog))
                    });
                match tex {
                    Some(tex) => ui
                        .add(egui::Button::image(egui::Image::new((
                            tex.id(),
                            egui::Vec2::splat(icon_side),
                        ))))
                        .on_hover_text(ht::ED_PICK_WINDOW),
                    None => {
                        crate::widgets::icon_button(ui, crate::model_icons::WINDOW, "Pick window…")
                    }
                }
            } else {
                crate::widgets::icon_button(ui, crate::model_icons::WINDOW, "Pick window…")
            };
            if pick.clicked() {
                self.window_picker =
                    pickers::open_window_picker(&self.form_process_path, &self.form_window_title);
            }
            help::tip(
                ui.label(egui::RichText::new(name).monospace()),
                help::DE_RUNNING_PROGRAM,
            );
            if !path.is_empty() {
                help::tip(
                    ui.label(egui::RichText::new(egui_phosphor::regular::FOLDER).weak()),
                    &path,
                );
            }
            if ui
                .add_enabled(
                    has_bound,
                    crate::widgets::IconButton::new(
                        egui_phosphor::regular::X,
                        "Clear",
                        has_bound.then_some(crate::theme::MACRO_STOP),
                    ),
                )
                .clicked()
            {
                self.form_process_path.clear();
                self.form_window_title.clear();
            }
        });
        ui.add_space(crate::theme::element_gap(ui));
        let macro_tag_completions = crate::macro_meta::collect_all_macro_tags(macros);
        crate::widgets::tag_chip_editor(
            ui,
            &mut self.form_tags,
            &mut self.tag_draft,
            &macro_tag_completions,
            crate::widgets::TagChipOptions {
                label_name: Some("Macro tags"),
                draft_hover: Some(help::DE_PROGRAM_MACRO_TAGS),
                ..Default::default()
            },
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
        self.program_selector(ui, catalog, icons, settings);
        ui.add_space(crate::theme::element_gap(ui));
        paint_name_field(ui, &mut self.form_name);
        ui.add_space(crate::theme::element_gap(ui));
        let tag_submit = self.paint_item_param_fields(ui, catalog, icons);
        if let (Some(prog), Some(item)) =
            (self.selected_program.clone(), self.selected_entity.clone())
        {
            let target = format!("{prog}{PROGRAM_DELIMITER}{item}");
            self.paint_item_variants_ui(ui, icons, catalog, &target, &item);
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
            crate::widgets::TagChipOptions {
                label_name: Some("Tags"),
                draft_hover: Some(help::DE_TAGS),
                ..Default::default()
            },
        )
        .submitted;
        ui.add_space(crate::theme::element_gap(ui));
        crate::widgets::wrapped_row(ui, |ui| {
            for (icon, label, help_text, value, range) in [
                (
                    egui_phosphor::regular::COLUMNS,
                    "Cols",
                    help::DE_COLS,
                    &mut self.form_cols,
                    1..=i32::MAX,
                ),
                (
                    egui_phosphor::regular::ROWS,
                    "Rows",
                    help::DE_ROWS,
                    &mut self.form_rows,
                    1..=i32::MAX,
                ),
                (
                    egui_phosphor::regular::STACK,
                    "Stack max",
                    help::DE_STACK_MAX,
                    &mut self.form_stack_max,
                    0..=i32::MAX,
                ),
            ] {
                crate::widgets::num_text_field_opts(
                    ui,
                    icon,
                    label,
                    help_text,
                    value,
                    crate::widgets::NumFieldOpts { width: 80.0, range },
                );
            }
        });
        ui.add_space(crate::theme::element_gap(ui));
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
        crate::widgets::wrap_unit(ui, |ui| {
            help::label(ui, crate::model_icons::MASK, "Mask");
            help::tip(
                searchable_combo(
                    ui,
                    "item_mask",
                    &mut current,
                    crate::widgets::SearchableComboOpts {
                        options: &masks,
                        empty_text: sqyre_domain::EMPTY_NONE,
                        none_label: Some(sqyre_domain::EMPTY_NONE),
                        width: None,
                        on_option_hover: Some(&mut on_hover),
                        option_icon: None,
                    },
                ),
                help::DE_MASK,
            );
        });
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
