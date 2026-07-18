//! Form buffers: load, draw, dirty, valid.

use super::form_state;
use super::helpers::{
    collect_program_item_tags, form_coord_literal, item_tag_completion_options, parse_i32,
};
use super::{DataEditor, EditorTab};
use crate::action_tooltip::help;
use crate::collection_capture::capture_and_save_collection_image;
use crate::data_editor_preview::{
    paint_disk_preview, paint_preview_coord_chip, paint_preview_toolbar,
    paint_zoomable_collection_preview, CardinalEdge,
};
use crate::overlay_icons;
use crate::paint_ctx::CatalogPaint;
use crate::pickers;
use crate::theme;
use crate::var_pills;
use eframe::egui;
use sqyre_domain::{collect_known_variable_names, Macro, PROGRAM_DELIMITER};
use sqyre_hotkeys::ScreenClickBridge;
use sqyre_persist::{
    auto_pic_path, OverlayButtonConfig, ProgramCatalog, ProgramCollection, UserSettings,
    DEFAULT_OVERLAY_BUTTON_SIZE, MAX_OVERLAY_BORDER_WIDTH, MAX_OVERLAY_BUTTON_SIZE,
    MAX_OVERLAY_CORNER_RADIUS, MIN_OVERLAY_BORDER_WIDTH, MIN_OVERLAY_BUTTON_SIZE,
    MIN_OVERLAY_CORNER_RADIUS,
};
use sqyre_validate::validate_numeric_expression;

fn color_alpha_drag(ui: &mut egui::Ui, label: &str, color: &mut egui::Color32) {
    ui.label(label);
    ui.color_edit_button_srgba(color);
    let mut alpha = color.a();
    if ui
        .add(egui::DragValue::new(&mut alpha).range(0..=255).prefix("α "))
        .changed()
    {
        *color = egui::Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), alpha);
    }
}

impl DataEditor {
    /// Name field + optional screen-record arm/cancel controls.
    fn paint_name_record_row(
        &mut self,
        ui: &mut egui::Ui,
        screen_click: &ScreenClickBridge,
        record_tip: &str,
        recording_msg: &str,
        arm: impl FnOnce(&ScreenClickBridge),
    ) {
        ui.horizontal(|ui| {
            help::label(ui, "Name", help::DE_NAME);
            help::tip(
                ui.add(
                    egui::TextEdit::singleline(&mut self.form_name)
                        .desired_width(ui.available_width() - 48.0),
                ),
                help::DE_NAME,
            );
            let armed = screen_click.is_armed();
            if theme::record_icon_button(ui, record_tip, !armed).clicked() {
                self.save_after_record = false;
                arm(screen_click);
                self.set_ok(recording_msg);
            }
            if armed && ui.button("Cancel").clicked() {
                self.save_after_record = false;
                screen_click.disarm();
            }
        });
    }

    pub(crate) fn load_form(&mut self, catalog: &ProgramCatalog, settings: &UserSettings) {
        self.clear_status();
        form_state::load_tab(self.tab, self, catalog, settings);
    }

    pub(crate) fn load_overlay_form(&mut self, settings: &UserSettings) {
        let Some(id) = self.selected_entity.as_deref() else {
            self.reset_overlay_form();
            return;
        };
        let Some(btn) = settings.overlay_buttons.iter().find(|b| b.id == id) else {
            self.reset_overlay_form();
            return;
        };
        self.form_name = btn.label.clone();
        self.form_overlay_x = btn.x;
        self.form_overlay_y = btn.y;
        self.form_overlay_macro = btn.macro_name.clone();
        self.form_overlay_icon = if btn.icon.trim().is_empty() {
            overlay_icons::DEFAULT_ICON_ID.into()
        } else {
            btn.icon.clone()
        };
        self.form_overlay_size = if btn.size > 0.0 {
            btn.size
        } else {
            DEFAULT_OVERLAY_BUTTON_SIZE
        };
        self.load_overlay_style_from_config(btn);
    }

    pub(crate) fn reset_overlay_form(&mut self) {
        self.form_name.clear();
        self.form_overlay_x = 48.0;
        self.form_overlay_y = 48.0;
        self.form_overlay_macro.clear();
        self.form_overlay_icon = overlay_icons::DEFAULT_ICON_ID.into();
        self.form_overlay_size = DEFAULT_OVERLAY_BUTTON_SIZE;
        self.reset_overlay_style_form();
    }

    pub(crate) fn reset_item_form(&mut self) {
        self.form_name.clear();
        self.form_cols = "1".into();
        self.form_rows = "1".into();
        self.form_stack_max = "0".into();
        self.form_mask.clear();
        self.form_tags.clear();
    }

    pub(crate) fn reset_mask_form(&mut self) {
        self.form_name.clear();
        self.form_shape = "rectangle".into();
        self.form_center_x = "50".into();
        self.form_center_y = "50".into();
        self.form_base.clear();
        self.form_height.clear();
        self.form_radius.clear();
        self.form_inverse = false;
    }

    pub(crate) fn reset_collection_form(&mut self) {
        self.form_name.clear();
        self.form_search_area.clear();
        self.form_rows = "1".into();
        self.form_cols = "1".into();
    }

    pub(crate) fn draw_form(
        &mut self,
        ui: &mut egui::Ui,
        paint: &mut CatalogPaint<'_>,
        screen_click: &ScreenClickBridge,
        macros: &[Macro],
        active_macro: Option<&Macro>,
        settings: &mut UserSettings,
    ) {
        let CatalogPaint {
            catalog,
            icons,
            previews,
        } = paint;
        let known = active_macro
            .map(collect_known_variable_names)
            .unwrap_or_default();
        let is_dark = ui.visuals().dark_mode;
        match self.tab {
            EditorTab::Programs => {
                ui.heading("Program");
                help::label(ui, "Name", help::DE_NAME);
                help::tip(
                    ui.add(
                        egui::TextEdit::singleline(&mut self.form_name).desired_width(f32::INFINITY),
                    ),
                    help::DE_NAME,
                );
                ui.add_space(8.0);
                help::label(ui, "Running program", help::DE_RUNNING_PROGRAM);
                ui.weak(
                    "Overlay buttons for this program show when this process and window title own focus.",
                );
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
                ui.label(egui::RichText::new(bound).monospace());
                ui.horizontal(|ui| {
                    if ui.button("Select…").clicked() {
                        self.window_picker = pickers::open_window_picker(
                            &self.form_process_path,
                            &self.form_window_title,
                        );
                    }
                    if ui
                        .add_enabled(
                            !self.form_process_path.is_empty()
                                || !self.form_window_title.is_empty(),
                            egui::Button::new("Clear"),
                        )
                        .clicked()
                    {
                        self.form_process_path.clear();
                        self.form_window_title.clear();
                    }
                });
            }
            EditorTab::Items => {
                ui.heading("Item");
                self.program_selector(ui, catalog);
                ui.add_space(4.0);
                help::label(ui, "Name", help::DE_NAME);
                help::tip(
                    ui.add(
                        egui::TextEdit::singleline(&mut self.form_name).desired_width(f32::INFINITY),
                    ),
                    help::DE_NAME,
                );
                ui.add_space(4.0);
                help::label(ui, "Cols", help::DE_COLS);
                help::tip(
                    ui.add(egui::TextEdit::singleline(&mut self.form_cols).desired_width(80.0)),
                    help::DE_COLS,
                );
                help::label(ui, "Rows", help::DE_ROWS);
                help::tip(
                    ui.add(egui::TextEdit::singleline(&mut self.form_rows).desired_width(80.0)),
                    help::DE_ROWS,
                );
                help::label(ui, "Stack max", help::DE_STACK_MAX);
                help::tip(
                    ui.add(
                        egui::TextEdit::singleline(&mut self.form_stack_max).desired_width(80.0),
                    ),
                    help::DE_STACK_MAX,
                );
                ui.add_space(4.0);
                help::label(ui, "Mask", help::DE_MASK);
                {
                    let masks: Vec<String> = self
                        .selected_program
                        .as_deref()
                        .and_then(|p| catalog.get(p))
                        .map(|p| p.masks.keys().cloned().collect())
                        .unwrap_or_default();
                    let mut current = self.form_mask.clone();
                    egui::ComboBox::from_id_salt("item_mask")
                        .selected_text(if current.is_empty() {
                            "(none)".into()
                        } else {
                            current.clone()
                        })
                        .show_ui(ui, |ui| {
                            if ui
                                .selectable_value(&mut current, String::new(), "(none)")
                                .clicked()
                            {
                                self.form_mask.clear();
                            }
                            for m in &masks {
                                if ui.selectable_value(&mut current, m.clone(), m).clicked() {
                                    self.form_mask = m.clone();
                                }
                            }
                        });
                    if current != self.form_mask {
                        self.form_mask = current;
                    }
                    if let (Some(prog), mask) =
                        (self.selected_program.as_deref(), self.form_mask.as_str())
                    {
                        if !mask.is_empty() {
                            if let Some(m) = catalog.get(prog).and_then(|p| p.masks.get(mask)) {
                                let detail = if catalog.mask_image_path(prog, mask).is_file() {
                                    "Image mask on disk".to_string()
                                } else if m.shape == sqyre_domain::MaskShape::Circle {
                                    format!(
                                        "Circle @ ({}, {}) r={}",
                                        m.center_x, m.center_y, m.radius
                                    )
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
                }
                ui.add_space(4.0);
                help::label(ui, "Tags", help::DE_TAGS);
                ui.horizontal_wrapped(|ui| {
                    let mut remove: Option<usize> = None;
                    for (i, tag) in self.form_tags.iter().enumerate() {
                        if ui.button(format!("{tag} ×")).clicked() {
                            remove = Some(i);
                        }
                    }
                    if let Some(i) = remove {
                        self.form_tags.remove(i);
                    }
                });
                ui.horizontal(|ui| {
                    let tag_te = egui::TextEdit::singleline(&mut self.tag_draft)
                        .desired_width(140.0)
                        .hint_text("Add tag…");
                    let tag_resp = ui.add(tag_te);
                    let add_clicked = ui.button("Add tag").clicked();
                    let add_enter =
                        tag_resp.has_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
                    if add_clicked || add_enter {
                        let t = self.tag_draft.trim().to_string();
                        if !t.is_empty() && !self.form_tags.iter().any(|x| x == &t) {
                            self.form_tags.push(t);
                        }
                        self.tag_draft.clear();
                    }
                });
                // Completion from other item tags in this program.
                if !self.tag_draft.trim().is_empty() {
                    if let Some(prog) = self.selected_program.as_deref() {
                        let program_tags = collect_program_item_tags(catalog, prog);
                        let suggestions = item_tag_completion_options(
                            &self.tag_draft,
                            &self.form_tags,
                            &program_tags,
                            8,
                        );
                        if !suggestions.is_empty() {
                            ui.horizontal_wrapped(|ui| {
                                for sug in suggestions {
                                    if ui.small_button(&sug).clicked() {
                                        if !self.form_tags.iter().any(|x| x == &sug) {
                                            self.form_tags.push(sug);
                                        }
                                        self.tag_draft.clear();
                                    }
                                }
                            });
                        }
                    }
                }
                if let (Some(prog), Some(item)) =
                    (self.selected_program.clone(), self.selected_entity.clone())
                {
                    let target = format!("{prog}{PROGRAM_DELIMITER}{item}");
                    self.paint_item_variants_ui(ui, icons, catalog, &target, &item);
                }
            }
            EditorTab::Points => {
                ui.heading("Point");
                self.program_selector(ui, catalog);
                ui.add_space(4.0);
                self.paint_name_record_row(
                    ui,
                    screen_click,
                    "Click on screen to capture X/Y",
                    "Recording… left-click to capture.",
                    ScreenClickBridge::arm_point,
                );
                ui.weak("X/Y overlay the preview; integers or ${var}.");
                let x = form_coord_literal(&self.form_x);
                let y = form_coord_literal(&self.form_y);
                let force = paint_preview_toolbar(ui);
                let rect = previews.paint_point_panel(ui, x, y, force);
                let vx = validate_numeric_expression(&self.form_x, active_macro);
                let vy = validate_numeric_expression(&self.form_y, active_macro);
                paint_preview_coord_chip(
                    ui,
                    rect,
                    CardinalEdge::Left,
                    "X",
                    &mut self.form_x,
                    &known,
                    is_dark,
                    &vx,
                    help::DE_POINT_X,
                );
                paint_preview_coord_chip(
                    ui,
                    rect,
                    CardinalEdge::Bottom,
                    "Y",
                    &mut self.form_y,
                    &known,
                    is_dark,
                    &vy,
                    help::DE_POINT_Y,
                );
            }
            EditorTab::SearchAreas => {
                ui.heading("Search Area");
                self.program_selector(ui, catalog);
                ui.add_space(4.0);
                self.paint_name_record_row(
                    ui,
                    screen_click,
                    "Two clicks: opposite corners of the area",
                    "Recording… click two corners.",
                    ScreenClickBridge::arm_search_area,
                );
                ui.weak("Bounds overlay the preview edges; integers or ${var}.");
                let lx = form_coord_literal(&self.form_left);
                let ty = form_coord_literal(&self.form_top);
                let rx = form_coord_literal(&self.form_right);
                let by = form_coord_literal(&self.form_bottom);
                let force = paint_preview_toolbar(ui);
                let rect = previews.paint_search_area_panel(ui, lx, ty, rx, by, force);
                let v_top = validate_numeric_expression(&self.form_top, active_macro);
                let v_bottom = validate_numeric_expression(&self.form_bottom, active_macro);
                let v_left = validate_numeric_expression(&self.form_left, active_macro);
                let v_right = validate_numeric_expression(&self.form_right, active_macro);
                paint_preview_coord_chip(
                    ui,
                    rect,
                    CardinalEdge::Top,
                    "TopY",
                    &mut self.form_top,
                    &known,
                    is_dark,
                    &v_top,
                    help::DE_AREA_TOP,
                );
                paint_preview_coord_chip(
                    ui,
                    rect,
                    CardinalEdge::Bottom,
                    "BottomY",
                    &mut self.form_bottom,
                    &known,
                    is_dark,
                    &v_bottom,
                    help::DE_AREA_BOTTOM,
                );
                paint_preview_coord_chip(
                    ui,
                    rect,
                    CardinalEdge::Left,
                    "LeftX",
                    &mut self.form_left,
                    &known,
                    is_dark,
                    &v_left,
                    help::DE_AREA_LEFT,
                );
                paint_preview_coord_chip(
                    ui,
                    rect,
                    CardinalEdge::Right,
                    "RightX",
                    &mut self.form_right,
                    &known,
                    is_dark,
                    &v_right,
                    help::DE_AREA_RIGHT,
                );
            }
            EditorTab::Masks => {
                ui.heading("Mask");
                self.program_selector(ui, catalog);
                ui.add_space(4.0);
                ui.label("Name").on_hover_text(help::DE_NAME);
                help::tip(
                    ui.add(
                        egui::TextEdit::singleline(&mut self.form_name).desired_width(f32::INFINITY),
                    ),
                    help::DE_NAME,
                );
                let has_image = self
                    .selected_program
                    .as_deref()
                    .zip(self.selected_entity.as_deref())
                    .map(|(p, m)| catalog.mask_image_path(p, m).is_file())
                    .unwrap_or(false);
                ui.horizontal(|ui| {
                    if ui
                        .add_enabled(
                            self.selected_program.is_some() && self.selected_entity.is_some(),
                            egui::Button::new("Upload Image"),
                        )
                        .on_hover_text("Replace this mask with a PNG from disk.")
                        .clicked()
                    {
                        self.upload_mask_image(catalog, icons);
                    }
                    if ui
                        .add_enabled(has_image, egui::Button::new("Remove Image"))
                        .on_hover_text("Delete the PNG and use shape geometry instead.")
                        .clicked()
                    {
                        self.remove_mask_image(catalog, icons);
                    }
                });
                if has_image {
                    ui.weak("Image mask mode — shape fields hidden while a PNG is on disk.");
                } else {
                    ui.add_space(4.0);
                    help::label(ui, "Shape", help::DE_MASK_SHAPE);
                    ui.horizontal(|ui| {
                        ui.selectable_value(&mut self.form_shape, "rectangle".into(), "Rectangle")
                            .on_hover_text(help::DE_MASK_SHAPE);
                        ui.selectable_value(&mut self.form_shape, "circle".into(), "Circle")
                            .on_hover_text(help::DE_MASK_SHAPE);
                    });
                    ui.checkbox(
                        &mut self.form_inverse,
                        "Inverse (shape included, rest excluded)",
                    )
                    .on_hover_text("When on, only the shape region is kept; the rest is masked out.");
                    ui.add_space(4.0);
                    let cx = validate_numeric_expression(&self.form_center_x, active_macro);
                    var_pills::validated_var_ref_edit(
                        ui,
                        "Center X %",
                        &mut self.form_center_x,
                        &known,
                        is_dark,
                        f32::INFINITY,
                        &cx,
                        "Horizontal center of the shape (0–100%).",
                    );
                    let cy = validate_numeric_expression(&self.form_center_y, active_macro);
                    var_pills::validated_var_ref_edit(
                        ui,
                        "Center Y %",
                        &mut self.form_center_y,
                        &known,
                        is_dark,
                        f32::INFINITY,
                        &cy,
                        "Vertical center of the shape (0–100%).",
                    );
                    if self.form_shape == "circle" {
                        let radius = validate_numeric_expression(&self.form_radius, active_macro);
                        var_pills::validated_var_ref_edit(
                            ui,
                            "Radius",
                            &mut self.form_radius,
                            &known,
                            is_dark,
                            f32::INFINITY,
                            &radius,
                            "Circle radius as a percent of the search area.",
                        );
                    } else {
                        let base = validate_numeric_expression(&self.form_base, active_macro);
                        var_pills::validated_var_ref_edit(
                            ui,
                            "Base",
                            &mut self.form_base,
                            &known,
                            is_dark,
                            f32::INFINITY,
                            &base,
                            "Rectangle width as a percent of the search area.",
                        );
                        let height = validate_numeric_expression(&self.form_height, active_macro);
                        var_pills::validated_var_ref_edit(
                            ui,
                            "Height",
                            &mut self.form_height,
                            &known,
                            is_dark,
                            f32::INFINITY,
                            &height,
                            "Rectangle height as a percent of the search area.",
                        );
                    }
                    ui.weak("Numeric fields accept literals or ${var} expressions.");
                }
                if let (Some(prog), Some(mask)) = (
                    self.selected_program.as_deref(),
                    self.selected_entity.as_deref(),
                ) {
                    let path = catalog.mask_image_path(prog, mask);
                    paint_disk_preview(
                        ui,
                        icons,
                        Some(path.as_path()),
                        None,
                        "Mask image",
                        None,
                        None,
                    );
                }
            }
            EditorTab::Collections => {
                ui.heading("Collection");
                self.program_selector(ui, catalog);
                ui.add_space(4.0);
                ui.label("Name").on_hover_text(help::DE_NAME);
                help::tip(
                    ui.add(
                        egui::TextEdit::singleline(&mut self.form_name).desired_width(f32::INFINITY),
                    ),
                    help::DE_NAME,
                );
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
                    egui::ComboBox::from_id_salt("collection_sa")
                        .selected_text(if current.is_empty() {
                            "(none)".into()
                        } else {
                            current.clone()
                        })
                        .show_ui(ui, |ui| {
                            for a in &areas {
                                if ui.selectable_value(&mut current, a.clone(), a).clicked() {
                                    self.form_search_area = a.clone();
                                }
                            }
                        });
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
                    let rows = parse_i32(&self.form_rows).unwrap_or(1).max(1);
                    let cols = parse_i32(&self.form_cols).unwrap_or(1).max(1);
                    let key = (prog.clone(), col_name.clone());
                    if self.collection_preview_key.as_ref() != Some(&key) {
                        self.collection_preview.reset();
                        self.collection_preview_key = Some(key);
                    }
                    let mut replace = false;
                    paint_zoomable_collection_preview(
                        ui,
                        icons,
                        path.as_path(),
                        rows,
                        cols,
                        &mut self.collection_preview,
                        &mut replace,
                    );
                    if replace {
                        let col = ProgramCollection {
                            name: col_name.clone(),
                            search_area: self.form_search_area.trim().to_string(),
                            rows,
                            cols,
                        };
                        match capture_and_save_collection_image(catalog, &prog, &col) {
                            Ok(()) => {
                                icons.invalidate_path(&path);
                                self.collection_preview.reset();
                                self.set_ok("Replaced collection image.");
                            }
                            Err(e) => self.set_err(e),
                        }
                    }
                }
            }
            EditorTab::AutoPic => {
                ui.heading("AutoPic");
                ui.weak("Select a search area, preview, then save a PNG into images/AutoPic.");
                if self.selected_program.is_some() && self.selected_entity.is_some() {
                    let lx = form_coord_literal(&self.form_left);
                    let ty = form_coord_literal(&self.form_top);
                    let rx = form_coord_literal(&self.form_right);
                    let by = form_coord_literal(&self.form_bottom);
                    let force = paint_preview_toolbar(ui);
                    previews.paint_search_area_panel(ui, lx, ty, rx, by, force);
                    ui.add_space(8.0);
                    let saving = self.autopix_pending.is_some();
                    if ui
                        .add_enabled(
                            !saving,
                            egui::Button::new(if saving { "Saving…" } else { "Save" }),
                        )
                        .clicked()
                    {
                        self.save_autopix();
                        ui.ctx().request_repaint();
                    }
                    ui.weak(format!("Saves to {}", auto_pic_path().display()));
                } else {
                    ui.weak("Select a search area from the list.");
                }
            }
            EditorTab::Overlay => {
                ui.heading("Overlay Button");
                ui.weak(
                    "Buttons appear when this program's bound process and window title own focus. Bind a window on the Programs tab.",
                );
                ui.weak("The selected button is previewed on screen while you edit.");
                ui.add_space(4.0);
                if ui
                    .checkbox(
                        &mut settings.overlay_enabled,
                        "Show overlay buttons on screen",
                    )
                    .changed()
                {
                    self.persist_overlay_settings(settings);
                }
                ui.add_space(6.0);
                self.program_selector(ui, catalog);
                if self.selected_program.is_none() {
                    ui.weak("Select a program, then New to add a button.");
                    return;
                }
                if self.selected_entity.is_none() {
                    ui.weak("Select a button from the list, or click New.");
                    return;
                }
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    let icon = overlay_icons::resolve(&self.form_overlay_icon);
                    let mut preview_cfg = OverlayButtonConfig::new("preview", "");
                    self.apply_overlay_style_to_config(&mut preview_cfg);
                    let style = overlay_icons::OverlayPaintStyle::from_config(&preview_cfg);
                    let preview = overlay_icons::style_preview_button(ui, icon, 48.0, &style)
                        .on_hover_text(help::DE_OVERLAY_ICON);
                    if preview.clicked() {
                        if let Some(id) = self.selected_entity.clone() {
                            self.overlay_icon_search.clear();
                            self.overlay_icon_picker_for = Some(id);
                        }
                    }
                    ui.vertical(|ui| {
                        ui.label(icon.label).on_hover_text(help::DE_OVERLAY_ICON);
                        ui.weak("Click icon to choose from Phosphor library");
                    });
                });
                ui.add_space(6.0);
                help::label(ui, "Label", help::DE_OVERLAY_LABEL);
                help::tip(
                    ui.add(
                        egui::TextEdit::singleline(&mut self.form_name)
                            .desired_width(f32::INFINITY)
                            .hint_text("optional"),
                    ),
                    help::DE_OVERLAY_LABEL,
                );
                ui.add_space(4.0);
                help::label(ui, "Macro", help::DE_OVERLAY_MACRO);
                let mut selected = self.form_overlay_macro.clone();
                let before = selected.clone();
                let macro_names: Vec<String> = macros.iter().map(|m| m.name.clone()).collect();
                egui::ComboBox::from_id_salt("overlay_form_macro")
                    .selected_text(if selected.is_empty() {
                        "(pick macro)".to_string()
                    } else {
                        selected.clone()
                    })
                    .width(220.0)
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut selected, String::new(), "(none)");
                        for name in &macro_names {
                            ui.selectable_value(&mut selected, name.clone(), name);
                        }
                    })
                    .response
                    .on_hover_text(help::DE_OVERLAY_MACRO);
                if selected != before {
                    self.form_overlay_macro = selected;
                }
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    help::label(ui, "X", help::DE_OVERLAY_X);
                    help::tip(
                        ui.add(
                            egui::DragValue::new(&mut self.form_overlay_x)
                                .speed(1.0)
                                .suffix(" px"),
                        ),
                        help::DE_OVERLAY_X,
                    );
                    help::label(ui, "Y", help::DE_OVERLAY_Y);
                    help::tip(
                        ui.add(
                            egui::DragValue::new(&mut self.form_overlay_y)
                                .speed(1.0)
                                .suffix(" px"),
                        ),
                        help::DE_OVERLAY_Y,
                    );
                });
                ui.add_space(8.0);
                ui.collapsing("Appearance", |ui| {
                    ui.horizontal(|ui| {
                        help::label(ui, "Size", help::DE_OVERLAY_SIZE);
                        help::tip(
                            ui.add(
                                egui::DragValue::new(&mut self.form_overlay_size)
                                    .speed(1)
                                    .range(MIN_OVERLAY_BUTTON_SIZE..=MAX_OVERLAY_BUTTON_SIZE),
                            ),
                            help::DE_OVERLAY_SIZE,
                        );
                    });
                    ui.horizontal(|ui| {
                        help::label(ui, "Corner radius", help::DE_OVERLAY_RADIUS);
                        help::tip(
                            ui.add(
                                egui::DragValue::new(&mut self.form_overlay_corner_radius)
                                    .speed(0.5)
                                    .range(MIN_OVERLAY_CORNER_RADIUS..=MAX_OVERLAY_CORNER_RADIUS)
                                    .suffix(" px"),
                            ),
                            help::DE_OVERLAY_RADIUS,
                        );
                        help::label(ui, "Border width", help::DE_OVERLAY_BORDER);
                        help::tip(
                            ui.add(
                                egui::DragValue::new(&mut self.form_overlay_border_width)
                                    .speed(0.1)
                                    .range(MIN_OVERLAY_BORDER_WIDTH..=MAX_OVERLAY_BORDER_WIDTH)
                                    .suffix(" px"),
                            ),
                            help::DE_OVERLAY_BORDER,
                        );
                    });
                    ui.add_space(4.0);
                    ui.horizontal(|ui| {
                        color_alpha_drag(ui, "Border", &mut self.form_overlay_border);
                    });
                    ui.horizontal(|ui| {
                        color_alpha_drag(ui, "Background", &mut self.form_overlay_bg);
                        ui.weak("(α 0 = none)");
                    });
                    ui.horizontal(|ui| {
                        color_alpha_drag(ui, "Icon", &mut self.form_overlay_icon_color);
                    });
                    ui.horizontal(|ui| {
                        ui.label("Icon hover")
                            .on_hover_text("Icon color when the pointer is over the button.");
                        ui.color_edit_button_srgba(&mut self.form_overlay_icon_hover);
                    });
                    ui.add_space(4.0);
                    if ui.button("Reset appearance to defaults").clicked() {
                        self.reset_overlay_style_form();
                    }
                });
            }
        }
    }

    pub(crate) fn is_dirty(&self, catalog: &ProgramCatalog, settings: &UserSettings) -> bool {
        form_state::dirty_tab(self.tab, self, catalog, settings)
    }

    pub(crate) fn form_valid(&self, active_macro: Option<&Macro>) -> bool {
        form_state::valid_tab(self.tab, self, active_macro)
    }
}
