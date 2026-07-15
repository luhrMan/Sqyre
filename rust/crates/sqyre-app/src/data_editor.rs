//! Floating Data Editor: Programs / Items / Points / Search Areas / Masks / Collections.

use crate::collection_capture::capture_and_save_collection_image;
use crate::icon_cache::IconCache;
use crate::preview_tooltip::{PreviewKind, PreviewTooltipCache};
use eframe::egui;
use sqyre_domain::{Macro, ProgramEntityKind, ScalarValue, PROGRAM_DELIMITER};
use sqyre_persist::{
    Database, ProgramCatalog, ProgramCollection, ProgramItem, ProgramMask, ProgramPoint,
    ProgramSearchArea,
};
use sqyre_validate::validate_entity_name;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EditorTab {
    Programs,
    Items,
    Points,
    SearchAreas,
    Masks,
    Collections,
}

#[derive(Debug, Clone)]
enum PendingConfirm {
    Delete {
        label: String,
    },
    Overwrite {
        kind: &'static str,
        name: String,
    },
}

pub struct DataEditor {
    pub open: bool,
    tab: EditorTab,
    search: String,
    /// Width of the left list pane (drag-adjustable).
    left_width: f32,
    /// Selected program name (all tabs).
    selected_program: Option<String>,
    /// Selected entity within program (items / points / search areas).
    selected_entity: Option<String>,
    // Form buffers
    form_name: String,
    form_x: String,
    form_y: String,
    form_left: String,
    form_top: String,
    form_right: String,
    form_bottom: String,
    form_cols: String,
    form_rows: String,
    form_stack_max: String,
    form_mask: String,
    form_tags: Vec<String>,
    tag_draft: String,
    form_icon_path: String,
    form_shape: String,
    form_center_x: String,
    form_center_y: String,
    form_base: String,
    form_height: String,
    form_radius: String,
    form_inverse: bool,
    form_search_area: String,
    status: Option<String>,
    status_error: bool,
    confirm: Option<PendingConfirm>,
}

impl Default for DataEditor {
    fn default() -> Self {
        Self {
            open: false,
            tab: EditorTab::Programs,
            search: String::new(),
            left_width: 280.0,
            selected_program: None,
            selected_entity: None,
            form_name: String::new(),
            form_x: String::new(),
            form_y: String::new(),
            form_left: String::new(),
            form_top: String::new(),
            form_right: String::new(),
            form_bottom: String::new(),
            form_cols: "1".into(),
            form_rows: "1".into(),
            form_stack_max: "0".into(),
            form_mask: String::new(),
            form_tags: Vec::new(),
            tag_draft: String::new(),
            form_icon_path: String::new(),
            form_shape: "rectangle".into(),
            form_center_x: "50".into(),
            form_center_y: "50".into(),
            form_base: String::new(),
            form_height: String::new(),
            form_radius: String::new(),
            form_inverse: false,
            form_search_area: String::new(),
            status: None,
            status_error: false,
            confirm: None,
        }
    }
}

impl DataEditor {
    pub fn show(
        &mut self,
        ctx: &egui::Context,
        db: &mut Database,
        macros: &mut Vec<Macro>,
        catalog: &mut ProgramCatalog,
        icons: &mut IconCache,
        previews: &mut PreviewTooltipCache,
    ) {
        if !self.open {
            return;
        }
        let mut open = self.open;
        egui::Window::new("Data Editor")
            .open(&mut open)
            .default_size([880.0, 560.0])
            .min_size([520.0, 280.0])
            // No huge max_size — egui auto-expands toward max when content min_size ratchets.
            .resizable(true)
            .constrain(true)
            .show(ctx, |ui| {
                self.ui(ui, db, macros, catalog, icons, previews);
            });
        self.open = open;
    }

    fn ui(
        &mut self,
        ui: &mut egui::Ui,
        db: &mut Database,
        macros: &mut Vec<Macro>,
        catalog: &mut ProgramCatalog,
        icons: &mut IconCache,
        previews: &mut PreviewTooltipCache,
    ) {
        ui.horizontal(|ui| {
            let prev = self.tab;
            ui.selectable_value(&mut self.tab, EditorTab::Programs, "Programs");
            ui.selectable_value(&mut self.tab, EditorTab::Items, "Items");
            ui.selectable_value(&mut self.tab, EditorTab::Points, "Points");
            ui.selectable_value(&mut self.tab, EditorTab::SearchAreas, "Search Areas");
            ui.selectable_value(&mut self.tab, EditorTab::Masks, "Masks");
            ui.selectable_value(&mut self.tab, EditorTab::Collections, "Collections");
            if self.tab != prev {
                self.selected_entity = None;
                self.load_form(catalog);
            }
        });
        ui.separator();

        if let Some(msg) = &self.status {
            let color = if self.status_error {
                egui::Color32::from_rgb(220, 80, 80)
            } else {
                egui::Color32::from_rgb(80, 160, 80)
            };
            ui.colored_label(color, msg);
        }

        if let Some(confirm) = self.confirm.clone() {
            self.draw_confirm(ui, confirm, db, macros, catalog, previews);
            return;
        }

        // Claim exactly the remaining window area once (body + footer).
        // Allocating body then drawing footer separately made min_size > window size,
        // so egui's Resize auto-expand ratcheted toward max every frame.
        let rem = ui.available_size();
        let (outer, _) = ui.allocate_exact_size(rem, egui::Sense::hover());
        let footer_h =
            (ui.spacing().interact_size.y + ui.spacing().item_spacing.y * 3.0 + 8.0).min(rem.y * 0.4);
        let body_h = (rem.y - footer_h).max(40.0);
        let body_rect = egui::Rect::from_min_size(outer.min, egui::vec2(rem.x, body_h));
        let footer_rect = egui::Rect::from_min_max(
            egui::pos2(outer.min.x, outer.min.y + body_h),
            outer.max,
        );

        let item_gap = ui.spacing().item_spacing.x;
        const SPLITTER_W: f32 = 6.0;
        const MIN_LEFT: f32 = 140.0;
        const MIN_RIGHT: f32 = 200.0;
        let avail_w = body_rect.width();
        let max_left = (avail_w - SPLITTER_W - MIN_RIGHT - item_gap * 2.0).max(MIN_LEFT);
        self.left_width = self.left_width.clamp(MIN_LEFT, max_left);
        let body_left = body_rect.left();

        ui.scope_builder(egui::UiBuilder::new().max_rect(body_rect), |ui| {
            ui.set_clip_rect(body_rect);
            ui.with_layout(egui::Layout::left_to_right(egui::Align::Min), |ui| {
                ui.allocate_ui_with_layout(
                    egui::vec2(self.left_width, body_h),
                    egui::Layout::top_down(egui::Align::Min),
                    |ui| {
                        ui.set_max_size(egui::vec2(self.left_width, body_h));
                        self.draw_left_list(ui, catalog, icons, previews);
                    },
                );

                let (split_rect, split_resp) = ui.allocate_exact_size(
                    egui::vec2(SPLITTER_W, body_h),
                    egui::Sense::click_and_drag(),
                );
                let stroke = if split_resp.hovered() || split_resp.dragged() {
                    ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeHorizontal);
                    ui.visuals().widgets.active.fg_stroke
                } else {
                    ui.visuals().widgets.noninteractive.bg_stroke
                };
                ui.painter().vline(
                    split_rect.center().x,
                    split_rect.y_range(),
                    egui::Stroke::new(1.0, stroke.color),
                );
                if split_resp.dragged() {
                    if let Some(pos) = split_resp.interact_pointer_pos() {
                        self.left_width =
                            (pos.x - body_left - item_gap).clamp(MIN_LEFT, max_left);
                    }
                }

                let right_w = ui.available_width().max(MIN_RIGHT);
                ui.allocate_ui_with_layout(
                    egui::vec2(right_w, body_h),
                    egui::Layout::top_down(egui::Align::Min),
                    |ui| {
                        ui.set_max_size(egui::vec2(right_w, body_h));
                        egui::ScrollArea::vertical()
                            .id_salt("data_editor_form")
                            .auto_shrink([false, false])
                            .max_height(body_h)
                            .show(ui, |ui| {
                                ui.set_max_width(ui.available_width());
                                self.draw_form(ui, catalog, icons, previews);
                            });
                    },
                );
            });
        });

        ui.scope_builder(egui::UiBuilder::new().max_rect(footer_rect), |ui| {
            ui.set_clip_rect(footer_rect);
            ui.vertical(|ui| {
                ui.separator();
                ui.horizontal(|ui| {
                    if ui.button("New").clicked() {
                        self.on_new(db, macros, catalog, icons);
                    }
                    let dirty = self.is_dirty(catalog);
                    let valid = self.form_valid();
                    if ui
                        .add_enabled(dirty && valid, egui::Button::new("Update"))
                        .clicked()
                    {
                        self.on_update(db, macros, catalog, previews);
                    }
                    let can_delete = match self.tab {
                        EditorTab::Programs => self.selected_program.is_some(),
                        _ => self.selected_program.is_some() && self.selected_entity.is_some(),
                    };
                    if ui
                        .add_enabled(can_delete, egui::Button::new("Delete"))
                        .clicked()
                    {
                        let label = match self.tab {
                            EditorTab::Programs => format!(
                                "program “{}”",
                                self.selected_program.as_deref().unwrap_or("")
                            ),
                            EditorTab::Items => format!(
                                "item “{}”",
                                self.selected_entity.as_deref().unwrap_or("")
                            ),
                            EditorTab::Points => format!(
                                "point “{}”",
                                self.selected_entity.as_deref().unwrap_or("")
                            ),
                            EditorTab::SearchAreas => format!(
                                "search area “{}”",
                                self.selected_entity.as_deref().unwrap_or("")
                            ),
                            EditorTab::Masks => format!(
                                "mask “{}”",
                                self.selected_entity.as_deref().unwrap_or("")
                            ),
                            EditorTab::Collections => format!(
                                "collection “{}”",
                                self.selected_entity.as_deref().unwrap_or("")
                            ),
                        };
                        self.confirm = Some(PendingConfirm::Delete { label });
                    }
                });
            });
        });
    }

    fn draw_confirm(
        &mut self,
        ui: &mut egui::Ui,
        confirm: PendingConfirm,
        db: &mut Database,
        macros: &mut Vec<Macro>,
        catalog: &mut ProgramCatalog,
        previews: &mut PreviewTooltipCache,
    ) {
        match &confirm {
            PendingConfirm::Delete { label } => {
                ui.label(format!("Delete {label}? This cannot be undone."));
            }
            PendingConfirm::Overwrite { kind, name } => {
                ui.label(format!(
                    "{kind} “{name}” already exists. Overwrite / rename onto it?"
                ));
            }
        }
        ui.horizontal(|ui| {
            if ui.button("Cancel").clicked() {
                self.confirm = None;
            }
            if ui.button("Confirm").clicked() {
                match confirm {
                    PendingConfirm::Delete { .. } => {
                        self.confirm = None;
                        self.on_delete(db, macros, catalog, previews);
                    }
                    PendingConfirm::Overwrite { .. } => {
                        self.confirm = None;
                        self.apply_update(db, macros, catalog, true, previews);
                    }
                }
            }
        });
    }

    fn draw_left_list(
        &mut self,
        ui: &mut egui::Ui,
        catalog: &ProgramCatalog,
        icons: &mut IconCache,
        previews: &mut PreviewTooltipCache,
    ) {
        ui.horizontal(|ui| {
            ui.label("Search");
            ui.add(
                egui::TextEdit::singleline(&mut self.search).desired_width(f32::INFINITY),
            );
        });
        ui.separator();
        let q = self.search.to_ascii_lowercase();
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
                            if !q.is_empty() && !name.to_ascii_lowercase().contains(&q) {
                                continue;
                            }
                            let selected = self.selected_program.as_deref() == Some(name.as_str());
                            if ui.selectable_label(selected, name).clicked() {
                                self.select_program(name, catalog);
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
                            self.select_entity(prog, item, catalog);
                        }
                    }
                }
            }
            EditorTab::Points | EditorTab::SearchAreas | EditorTab::Masks | EditorTab::Collections => {
                let kind = match self.tab {
                    EditorTab::Points => Some(PreviewKind::Point),
                    EditorTab::SearchAreas => Some(PreviewKind::SearchArea),
                    _ => None,
                };
                egui::ScrollArea::vertical()
                    .id_salt("data_editor_coords")
                    .auto_shrink([false, false])
                    .max_height(list_h)
                    .show(ui, |ui| {
                        for prog in catalog.program_names() {
                            let entities = self.entity_names(catalog, prog);
                            let prog_match = q.is_empty() || prog.to_ascii_lowercase().contains(&q);
                            let any_entity = entities
                                .iter()
                                .any(|e| q.is_empty() || e.to_ascii_lowercase().contains(&q));
                            if !prog_match && !any_entity {
                                continue;
                            }
                            ui.label(
                                egui::RichText::new(prog.as_str()).size(16.0).strong(),
                            );
                            for ent in entities {
                                if !q.is_empty()
                                    && !ent.to_ascii_lowercase().contains(&q)
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
                                    self.select_entity(prog, &ent, catalog);
                                }
                            }
                        }
                    });
            }
        }
    }

    fn entity_names(&self, catalog: &ProgramCatalog, program: &str) -> Vec<String> {
        let Some(p) = catalog.get(program) else {
            return Vec::new();
        };
        let res = catalog.resolution_key();
        match self.tab {
            EditorTab::Items => p.items.keys().cloned().collect(),
            EditorTab::Points => p
                .points
                .get(res)
                .or_else(|| p.points.values().next())
                .map(|m| m.keys().cloned().collect())
                .unwrap_or_default(),
            EditorTab::SearchAreas => p
                .search_areas
                .get(res)
                .or_else(|| p.search_areas.values().next())
                .map(|m| m.keys().cloned().collect())
                .unwrap_or_default(),
            EditorTab::Masks => p.masks.keys().cloned().collect(),
            EditorTab::Collections => p.collections.keys().cloned().collect(),
            EditorTab::Programs => Vec::new(),
        }
    }

    fn select_program(&mut self, name: &str, catalog: &ProgramCatalog) {
        self.selected_program = Some(name.to_string());
        self.selected_entity = None;
        self.load_form(catalog);
    }

    fn select_entity(&mut self, program: &str, entity: &str, catalog: &ProgramCatalog) {
        self.selected_program = Some(program.to_string());
        self.selected_entity = Some(entity.to_string());
        self.load_form(catalog);
    }

    fn load_form(&mut self, catalog: &ProgramCatalog) {
        self.clear_status();
        match self.tab {
            EditorTab::Programs => {
                self.form_name = self.selected_program.clone().unwrap_or_default();
            }
            EditorTab::Items => {
                let (prog, name) = match (
                    self.selected_program.as_deref(),
                    self.selected_entity.as_deref(),
                ) {
                    (Some(p), Some(n)) => (p, n),
                    _ => {
                        self.reset_item_form();
                        return;
                    }
                };
                let Some(item) = catalog.get(prog).and_then(|p| p.items.get(name)) else {
                    self.reset_item_form();
                    return;
                };
                self.form_name = item.name.clone();
                self.form_cols = item.grid_cols.to_string();
                self.form_rows = item.grid_rows.to_string();
                self.form_stack_max = item.stack_max.to_string();
                self.form_mask = item.mask.clone();
                self.form_tags = item.tags.clone();
                self.form_icon_path.clear();
            }
            EditorTab::Points => {
                let (prog, name) = match (
                    self.selected_program.as_deref(),
                    self.selected_entity.as_deref(),
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
                self.form_name = pt.name.clone();
                self.form_x = scalar_to_edit(&pt.x);
                self.form_y = scalar_to_edit(&pt.y);
            }
            EditorTab::SearchAreas => {
                let (prog, name) = match (
                    self.selected_program.as_deref(),
                    self.selected_entity.as_deref(),
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
                self.form_name = sa.name.clone();
                self.form_left = scalar_to_edit(&sa.left_x);
                self.form_top = scalar_to_edit(&sa.top_y);
                self.form_right = scalar_to_edit(&sa.right_x);
                self.form_bottom = scalar_to_edit(&sa.bottom_y);
            }
            EditorTab::Masks => {
                let (prog, name) = match (
                    self.selected_program.as_deref(),
                    self.selected_entity.as_deref(),
                ) {
                    (Some(p), Some(n)) => (p, n),
                    _ => {
                        self.reset_mask_form();
                        return;
                    }
                };
                let Some(mask) = catalog.get(prog).and_then(|p| p.masks.get(name)) else {
                    self.reset_mask_form();
                    return;
                };
                self.form_name = mask.name.clone();
                self.form_shape = mask.shape.clone();
                self.form_center_x = mask.center_x.clone();
                self.form_center_y = mask.center_y.clone();
                self.form_base = mask.base.clone();
                self.form_height = mask.height.clone();
                self.form_radius = mask.radius.clone();
                self.form_inverse = mask.inverse;
            }
            EditorTab::Collections => {
                let (prog, name) = match (
                    self.selected_program.as_deref(),
                    self.selected_entity.as_deref(),
                ) {
                    (Some(p), Some(n)) => (p, n),
                    _ => {
                        self.reset_collection_form();
                        return;
                    }
                };
                let Some(col) = catalog.get(prog).and_then(|p| p.collections.get(name)) else {
                    self.reset_collection_form();
                    return;
                };
                self.form_name = col.name.clone();
                self.form_search_area = col.search_area.clone();
                self.form_rows = col.rows.to_string();
                self.form_cols = col.cols.to_string();
            }
        }
    }

    fn reset_item_form(&mut self) {
        self.form_name.clear();
        self.form_cols = "1".into();
        self.form_rows = "1".into();
        self.form_stack_max = "0".into();
        self.form_mask.clear();
        self.form_tags.clear();
        self.form_icon_path.clear();
    }

    fn reset_mask_form(&mut self) {
        self.form_name.clear();
        self.form_shape = "rectangle".into();
        self.form_center_x = "50".into();
        self.form_center_y = "50".into();
        self.form_base.clear();
        self.form_height.clear();
        self.form_radius.clear();
        self.form_inverse = false;
    }

    fn reset_collection_form(&mut self) {
        self.form_name.clear();
        self.form_search_area.clear();
        self.form_rows = "1".into();
        self.form_cols = "1".into();
    }

    fn draw_form(
        &mut self,
        ui: &mut egui::Ui,
        catalog: &ProgramCatalog,
        icons: &mut IconCache,
        previews: &mut PreviewTooltipCache,
    ) {
        match self.tab {
            EditorTab::Programs => {
                ui.heading("Program");
                ui.label("Name");
                ui.add(
                    egui::TextEdit::singleline(&mut self.form_name).desired_width(f32::INFINITY),
                );
            }
            EditorTab::Items => {
                ui.heading("Item");
                self.program_selector(ui, catalog);
                ui.add_space(4.0);
                ui.label("Name");
                ui.add(
                    egui::TextEdit::singleline(&mut self.form_name).desired_width(f32::INFINITY),
                );
                ui.add_space(4.0);
                ui.label("Cols");
                ui.add(egui::TextEdit::singleline(&mut self.form_cols).desired_width(80.0));
                ui.label("Rows");
                ui.add(egui::TextEdit::singleline(&mut self.form_rows).desired_width(80.0));
                ui.label("Stack max");
                ui.add(egui::TextEdit::singleline(&mut self.form_stack_max).desired_width(80.0));
                ui.add_space(4.0);
                ui.label("Mask");
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
                }
                ui.add_space(4.0);
                ui.label("Tags");
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
                    ui.text_edit_singleline(&mut self.tag_draft);
                    if ui.button("Add tag").clicked() {
                        let t = self.tag_draft.trim().to_string();
                        if !t.is_empty() && !self.form_tags.iter().any(|x| x == &t) {
                            self.form_tags.push(t);
                        }
                        self.tag_draft.clear();
                    }
                });
                ui.add_space(4.0);
                ui.label("Copy PNG to icons");
                ui.add(
                    egui::TextEdit::singleline(&mut self.form_icon_path)
                        .desired_width(f32::INFINITY),
                );
                ui.weak("On Update, if path is set, copies into images/icons/<program>/.");
                if let (Some(prog), Some(item)) = (
                    self.selected_program.as_deref(),
                    self.selected_entity.as_deref(),
                ) {
                    let target = format!("{prog}{PROGRAM_DELIMITER}{item}");
                    let path = catalog.variant_paths(&target).into_iter().next();
                    let fallback = icons.for_target_or_fallback(ui.ctx(), catalog, &target);
                    paint_disk_preview(
                        ui,
                        icons,
                        path.as_deref(),
                        Some(fallback),
                        "Icon preview",
                        None,
                        None,
                    );
                }
            }
            EditorTab::Points => {
                ui.heading("Point");
                self.program_selector(ui, catalog);
                ui.add_space(4.0);
                ui.label("Name");
                ui.add(
                    egui::TextEdit::singleline(&mut self.form_name).desired_width(f32::INFINITY),
                );
                ui.label("X");
                ui.add(
                    egui::TextEdit::singleline(&mut self.form_x).desired_width(f32::INFINITY),
                );
                ui.label("Y");
                ui.add(
                    egui::TextEdit::singleline(&mut self.form_y).desired_width(f32::INFINITY),
                );
                ui.weak("Coords accept integers or ${var} references.");
                let x = form_coord_i32(&self.form_x);
                let y = form_coord_i32(&self.form_y);
                let force = paint_preview_toolbar(ui);
                previews.paint_point_panel(ui, x, y, force);
            }
            EditorTab::SearchAreas => {
                ui.heading("Search Area");
                self.program_selector(ui, catalog);
                ui.add_space(4.0);
                ui.label("Name");
                ui.add(
                    egui::TextEdit::singleline(&mut self.form_name).desired_width(f32::INFINITY),
                );
                ui.label("Left");
                ui.add(
                    egui::TextEdit::singleline(&mut self.form_left).desired_width(f32::INFINITY),
                );
                ui.label("Top");
                ui.add(
                    egui::TextEdit::singleline(&mut self.form_top).desired_width(f32::INFINITY),
                );
                ui.label("Right");
                ui.add(
                    egui::TextEdit::singleline(&mut self.form_right).desired_width(f32::INFINITY),
                );
                ui.label("Bottom");
                ui.add(
                    egui::TextEdit::singleline(&mut self.form_bottom).desired_width(f32::INFINITY),
                );
                let lx = form_coord_i32(&self.form_left);
                let ty = form_coord_i32(&self.form_top);
                let rx = form_coord_i32(&self.form_right);
                let by = form_coord_i32(&self.form_bottom);
                let force = paint_preview_toolbar(ui);
                previews.paint_search_area_panel(ui, lx, ty, rx, by, force);
            }
            EditorTab::Masks => {
                ui.heading("Mask");
                self.program_selector(ui, catalog);
                ui.add_space(4.0);
                ui.label("Name");
                ui.add(
                    egui::TextEdit::singleline(&mut self.form_name).desired_width(f32::INFINITY),
                );
                ui.add_space(4.0);
                ui.label("Shape");
                ui.horizontal(|ui| {
                    ui.selectable_value(&mut self.form_shape, "rectangle".into(), "Rectangle");
                    ui.selectable_value(&mut self.form_shape, "circle".into(), "Circle");
                });
                ui.checkbox(&mut self.form_inverse, "Inverse");
                ui.add_space(4.0);
                ui.label("Center X %");
                ui.add(
                    egui::TextEdit::singleline(&mut self.form_center_x)
                        .desired_width(f32::INFINITY),
                );
                ui.label("Center Y %");
                ui.add(
                    egui::TextEdit::singleline(&mut self.form_center_y)
                        .desired_width(f32::INFINITY),
                );
                if self.form_shape == "circle" {
                    ui.label("Radius");
                    ui.add(
                        egui::TextEdit::singleline(&mut self.form_radius)
                            .desired_width(f32::INFINITY),
                    );
                } else {
                    ui.label("Base");
                    ui.add(
                        egui::TextEdit::singleline(&mut self.form_base)
                            .desired_width(f32::INFINITY),
                    );
                    ui.label("Height");
                    ui.add(
                        egui::TextEdit::singleline(&mut self.form_height)
                            .desired_width(f32::INFINITY),
                    );
                }
                ui.weak("Numeric fields accept literals or ${var} expressions.");
                if let (Some(prog), Some(mask)) = (
                    self.selected_program.as_deref(),
                    self.selected_entity.as_deref(),
                ) {
                    let path = catalog.mask_image_path(prog, mask);
                    paint_disk_preview(ui, icons, Some(path.as_path()), None, "Mask image", None, None);
                }
            }
            EditorTab::Collections => {
                ui.heading("Collection");
                self.program_selector(ui, catalog);
                ui.add_space(4.0);
                ui.label("Name");
                ui.add(
                    egui::TextEdit::singleline(&mut self.form_name).desired_width(f32::INFINITY),
                );
                ui.add_space(4.0);
                ui.label("Search area");
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
                ui.label("Rows");
                ui.add(egui::TextEdit::singleline(&mut self.form_rows).desired_width(80.0));
                ui.label("Cols");
                ui.add(egui::TextEdit::singleline(&mut self.form_cols).desired_width(80.0));
                if let (Some(prog), Some(col_name)) = (
                    self.selected_program.clone(),
                    self.selected_entity.clone(),
                ) {
                    let path = catalog.collection_image_path(&prog, &col_name);
                    let rows = parse_i32(&self.form_rows).unwrap_or(1).max(1);
                    let cols = parse_i32(&self.form_cols).unwrap_or(1).max(1);
                    let mut replace = false;
                    paint_disk_preview(
                        ui,
                        icons,
                        Some(path.as_path()),
                        None,
                        "Collection image",
                        Some((rows, cols)),
                        Some(&mut replace),
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
                                self.set_ok("Replaced collection image.");
                            }
                            Err(e) => self.set_err(e),
                        }
                    }
                }
            }
        }
    }

    fn program_selector(&mut self, ui: &mut egui::Ui, catalog: &ProgramCatalog) {
        let names: Vec<String> = catalog.program_names().cloned().collect();
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

    fn is_dirty(&self, catalog: &ProgramCatalog) -> bool {
        match self.tab {
            EditorTab::Programs => {
                let Some(sel) = self.selected_program.as_deref() else {
                    return !self.form_name.trim().is_empty();
                };
                self.form_name.trim() != sel
            }
            EditorTab::Items => {
                let (Some(prog), Some(ent)) = (
                    self.selected_program.as_deref(),
                    self.selected_entity.as_deref(),
                ) else {
                    return !self.form_name.trim().is_empty();
                };
                let Some(item) = catalog.get(prog).and_then(|p| p.items.get(ent)) else {
                    return true;
                };
                self.form_name.trim() != item.name
                    || parse_i32(&self.form_cols) != Some(item.grid_cols)
                    || parse_i32(&self.form_rows) != Some(item.grid_rows)
                    || parse_i32(&self.form_stack_max) != Some(item.stack_max)
                    || self.form_mask != item.mask
                    || self.form_tags != item.tags
                    || !self.form_icon_path.trim().is_empty()
            }
            EditorTab::Points => {
                let (Some(prog), Some(ent)) = (
                    self.selected_program.as_deref(),
                    self.selected_entity.as_deref(),
                ) else {
                    return !self.form_name.trim().is_empty();
                };
                let res = catalog.resolution_key();
                let Some(pt) = catalog
                    .get(prog)
                    .and_then(|p| p.points.get(res).or_else(|| p.points.values().next()))
                    .and_then(|m| m.get(ent))
                else {
                    return true;
                };
                self.form_name.trim() != pt.name
                    || parse_scalar(&self.form_x) != pt.x
                    || parse_scalar(&self.form_y) != pt.y
            }
            EditorTab::SearchAreas => {
                let (Some(prog), Some(ent)) = (
                    self.selected_program.as_deref(),
                    self.selected_entity.as_deref(),
                ) else {
                    return !self.form_name.trim().is_empty();
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
                self.form_name.trim() != sa.name
                    || parse_scalar(&self.form_left) != sa.left_x
                    || parse_scalar(&self.form_top) != sa.top_y
                    || parse_scalar(&self.form_right) != sa.right_x
                    || parse_scalar(&self.form_bottom) != sa.bottom_y
            }
            EditorTab::Masks => {
                let (Some(prog), Some(ent)) = (
                    self.selected_program.as_deref(),
                    self.selected_entity.as_deref(),
                ) else {
                    return !self.form_name.trim().is_empty();
                };
                let Some(mask) = catalog.get(prog).and_then(|p| p.masks.get(ent)) else {
                    return true;
                };
                self.form_name.trim() != mask.name
                    || self.form_shape != mask.shape
                    || self.form_center_x != mask.center_x
                    || self.form_center_y != mask.center_y
                    || self.form_base != mask.base
                    || self.form_height != mask.height
                    || self.form_radius != mask.radius
                    || self.form_inverse != mask.inverse
            }
            EditorTab::Collections => {
                let (Some(prog), Some(ent)) = (
                    self.selected_program.as_deref(),
                    self.selected_entity.as_deref(),
                ) else {
                    return !self.form_name.trim().is_empty();
                };
                let Some(col) = catalog.get(prog).and_then(|p| p.collections.get(ent)) else {
                    return true;
                };
                self.form_name.trim() != col.name
                    || self.form_search_area != col.search_area
                    || parse_i32(&self.form_rows) != Some(col.rows)
                    || parse_i32(&self.form_cols) != Some(col.cols)
            }
        }
    }

    fn form_valid(&self) -> bool {
        if validate_entity_name(self.form_name.trim()).is_err() {
            return false;
        }
        match self.tab {
            EditorTab::Programs => true,
            EditorTab::Items => {
                self.selected_program.is_some()
                    && parse_i32(&self.form_cols).is_some()
                    && parse_i32(&self.form_rows).is_some()
                    && parse_i32(&self.form_stack_max).is_some()
            }
            EditorTab::Points => {
                self.selected_program.is_some()
                    && !self.form_x.trim().is_empty()
                    && !self.form_y.trim().is_empty()
            }
            EditorTab::SearchAreas => {
                self.selected_program.is_some()
                    && !self.form_left.trim().is_empty()
                    && !self.form_top.trim().is_empty()
                    && !self.form_right.trim().is_empty()
                    && !self.form_bottom.trim().is_empty()
            }
            EditorTab::Masks => {
                self.selected_program.is_some()
                    && (self.form_shape == "rectangle" || self.form_shape == "circle")
                    && !self.form_center_x.trim().is_empty()
                    && !self.form_center_y.trim().is_empty()
            }
            EditorTab::Collections => {
                self.selected_program.is_some()
                    && !self.form_search_area.trim().is_empty()
                    && parse_i32(&self.form_rows).map(|n| n >= 1).unwrap_or(false)
                    && parse_i32(&self.form_cols).map(|n| n >= 1).unwrap_or(false)
            }
        }
    }

    fn on_new(
        &mut self,
        db: &mut Database,
        macros: &mut Vec<Macro>,
        catalog: &mut ProgramCatalog,
        icons: &mut IconCache,
    ) {
        self.clear_status();
        let created = match self.tab {
            EditorTab::Programs => {
                let base = if self.form_name.trim().is_empty() {
                    "New Program".to_string()
                } else {
                    self.form_name.trim().to_string()
                };
                let name = unique_name(&base, |n| catalog.get(n).is_some());
                match catalog.create_program(&name) {
                    Ok(()) => {
                        self.selected_program = Some(name.clone());
                        self.form_name = name;
                        Ok("Created program.")
                    }
                    Err(e) => Err(e.to_string()),
                }
            }
            EditorTab::Items => {
                let Some(prog) = self.selected_program.clone() else {
                    self.set_err("Select a program first.");
                    return;
                };
                let base = if self.form_name.trim().is_empty() {
                    "New Item".to_string()
                } else {
                    self.form_name.trim().to_string()
                };
                let name = unique_name(&base, |n| {
                    catalog.get(&prog).and_then(|p| p.items.get(n)).is_some()
                });
                let item = ProgramItem {
                    name: name.clone(),
                    mask: String::new(),
                    stack_max: 0,
                    grid_cols: 1,
                    grid_rows: 1,
                    tags: Vec::new(),
                };
                match catalog.upsert_item(&prog, item) {
                    Ok(()) => {
                        self.selected_entity = Some(name.clone());
                        self.load_form(catalog);
                        Ok("Created item.")
                    }
                    Err(e) => Err(e.to_string()),
                }
            }
            EditorTab::Points => {
                let Some(prog) = self.selected_program.clone() else {
                    self.set_err("Select a program first.");
                    return;
                };
                let base = if self.form_name.trim().is_empty() {
                    "New Point".to_string()
                } else {
                    self.form_name.trim().to_string()
                };
                let name = unique_name(&base, |n| {
                    let res = catalog.resolution_key();
                    catalog
                        .get(&prog)
                        .and_then(|p| p.points.get(res))
                        .and_then(|m| m.get(n))
                        .is_some()
                });
                let pt = ProgramPoint {
                    name: name.clone(),
                    x: ScalarValue::Int(0),
                    y: ScalarValue::Int(0),
                };
                match catalog.upsert_point(&prog, pt) {
                    Ok(()) => {
                        self.selected_entity = Some(name);
                        self.load_form(catalog);
                        Ok("Created point.")
                    }
                    Err(e) => Err(e.to_string()),
                }
            }
            EditorTab::SearchAreas => {
                let Some(prog) = self.selected_program.clone() else {
                    self.set_err("Select a program first.");
                    return;
                };
                let base = if self.form_name.trim().is_empty() {
                    "New Search Area".to_string()
                } else {
                    self.form_name.trim().to_string()
                };
                let name = unique_name(&base, |n| {
                    let res = catalog.resolution_key();
                    catalog
                        .get(&prog)
                        .and_then(|p| p.search_areas.get(res))
                        .and_then(|m| m.get(n))
                        .is_some()
                });
                let sa = ProgramSearchArea {
                    name: name.clone(),
                    left_x: ScalarValue::Int(0),
                    top_y: ScalarValue::Int(0),
                    right_x: ScalarValue::Int(100),
                    bottom_y: ScalarValue::Int(100),
                };
                match catalog.upsert_search_area(&prog, sa) {
                    Ok(()) => {
                        self.selected_entity = Some(name);
                        self.load_form(catalog);
                        Ok("Created search area.")
                    }
                    Err(e) => Err(e.to_string()),
                }
            }
            EditorTab::Masks => {
                let Some(prog) = self.selected_program.clone() else {
                    self.set_err("Select a program first.");
                    return;
                };
                let base = if self.form_name.trim().is_empty() {
                    "New Mask".to_string()
                } else {
                    self.form_name.trim().to_string()
                };
                let name = unique_name(&base, |n| {
                    catalog.get(&prog).and_then(|p| p.masks.get(n)).is_some()
                });
                let mask = ProgramMask {
                    name: name.clone(),
                    ..Default::default()
                };
                match catalog.upsert_mask(&prog, mask) {
                    Ok(()) => {
                        self.selected_entity = Some(name);
                        self.load_form(catalog);
                        Ok("Created mask.")
                    }
                    Err(e) => Err(e.to_string()),
                }
            }
            EditorTab::Collections => {
                let Some(prog) = self.selected_program.clone() else {
                    self.set_err("Select a program first.");
                    return;
                };
                let base = if self.form_name.trim().is_empty() {
                    "New Collection".to_string()
                } else {
                    self.form_name.trim().to_string()
                };
                let name = unique_name(&base, |n| {
                    catalog
                        .get(&prog)
                        .and_then(|p| p.collections.get(n))
                        .is_some()
                });
                let default_sa = catalog
                    .get(&prog)
                    .and_then(|p| {
                        let res = catalog.resolution_key();
                        p.search_areas
                            .get(res)
                            .or_else(|| p.search_areas.values().next())
                            .and_then(|m| m.keys().next().cloned())
                    })
                    .unwrap_or_default();
                let search_area = if !self.form_search_area.trim().is_empty() {
                    self.form_search_area.trim().to_string()
                } else {
                    default_sa
                };
                if search_area.is_empty() {
                    self.set_err("Create a search area before capturing a collection image.");
                    return;
                }
                let rows = parse_i32(&self.form_rows).unwrap_or(1).max(1);
                let cols = parse_i32(&self.form_cols).unwrap_or(1).max(1);
                let col = ProgramCollection {
                    name: name.clone(),
                    search_area,
                    rows,
                    cols,
                };
                match catalog.upsert_collection(&prog, col.clone()) {
                    Ok(()) => match capture_and_save_collection_image(catalog, &prog, &col) {
                        Ok(()) => {
                            let path = catalog.collection_image_path(&prog, &name);
                            icons.invalidate_path(&path);
                            self.selected_entity = Some(name);
                            self.load_form(catalog);
                            Ok("Created collection.")
                        }
                        Err(e) => {
                            let _ = catalog.delete_collection(&prog, &name);
                            Err(e)
                        }
                    },
                    Err(e) => Err(e.to_string()),
                }
            }
        };
        match created {
            Ok(msg) => {
                if let Err(e) = self.persist(db, macros, catalog) {
                    self.set_err(e);
                } else {
                    self.set_ok(msg);
                }
            }
            Err(e) => self.set_err(e),
        }
    }

    fn on_update(
        &mut self,
        db: &mut Database,
        macros: &mut Vec<Macro>,
        catalog: &mut ProgramCatalog,
        previews: &mut PreviewTooltipCache,
    ) {
        // Check overwrite for renames onto existing keys
        if let Some((kind, name)) = self.would_overwrite(catalog) {
            self.confirm = Some(PendingConfirm::Overwrite { kind, name });
            return;
        }
        self.apply_update(db, macros, catalog, false, previews);
    }

    fn would_overwrite(&self, catalog: &ProgramCatalog) -> Option<(&'static str, String)> {
        let new = self.form_name.trim();
        match self.tab {
            EditorTab::Programs => {
                let old = self.selected_program.as_deref()?;
                if old != new && catalog.get(new).is_some() {
                    return Some(("Program", new.to_string()));
                }
            }
            EditorTab::Items => {
                let prog = self.selected_program.as_deref()?;
                let old = self.selected_entity.as_deref()?;
                if old != new && catalog.get(prog).and_then(|p| p.items.get(new)).is_some() {
                    return Some(("Item", new.to_string()));
                }
            }
            EditorTab::Points => {
                let prog = self.selected_program.as_deref()?;
                let old = self.selected_entity.as_deref()?;
                let res = catalog.resolution_key();
                if old != new
                    && catalog
                        .get(prog)
                        .and_then(|p| p.points.get(res))
                        .and_then(|m| m.get(new))
                        .is_some()
                {
                    return Some(("Point", new.to_string()));
                }
            }
            EditorTab::SearchAreas => {
                let prog = self.selected_program.as_deref()?;
                let old = self.selected_entity.as_deref()?;
                let res = catalog.resolution_key();
                if old != new
                    && catalog
                        .get(prog)
                        .and_then(|p| p.search_areas.get(res))
                        .and_then(|m| m.get(new))
                        .is_some()
                {
                    return Some(("Search area", new.to_string()));
                }
            }
            EditorTab::Masks => {
                let prog = self.selected_program.as_deref()?;
                let old = self.selected_entity.as_deref()?;
                if old != new && catalog.get(prog).and_then(|p| p.masks.get(new)).is_some() {
                    return Some(("Mask", new.to_string()));
                }
            }
            EditorTab::Collections => {
                let prog = self.selected_program.as_deref()?;
                let old = self.selected_entity.as_deref()?;
                if old != new
                    && catalog
                        .get(prog)
                        .and_then(|p| p.collections.get(new))
                        .is_some()
                {
                    return Some(("Collection", new.to_string()));
                }
            }
        }
        None
    }

    fn apply_update(
        &mut self,
        db: &mut Database,
        macros: &mut Vec<Macro>,
        catalog: &mut ProgramCatalog,
        overwrite: bool,
        previews: &mut PreviewTooltipCache,
    ) {
        self.clear_status();
        let new_name = self.form_name.trim().to_string();
        if validate_entity_name(&new_name).is_err() {
            self.set_err("Invalid name.");
            return;
        }

        let old_entity = self.selected_entity.clone();
        let result = match self.tab {
            EditorTab::Programs => {
                if let Some(old) = self.selected_program.clone() {
                    if old == new_name {
                        Ok(())
                    } else {
                        if overwrite {
                            let _ = catalog.delete_program(&new_name);
                        }
                        catalog.rename_program(&old, &new_name).map(|_| {
                            for m in macros.iter_mut() {
                                m.rename_program(&old, &new_name);
                            }
                            self.selected_program = Some(new_name.clone());
                        })
                    }
                } else {
                    catalog.create_program(&new_name).map(|_| {
                        self.selected_program = Some(new_name.clone());
                    })
                }
            }
            EditorTab::Items => self.update_item(catalog, macros, &new_name, overwrite),
            EditorTab::Points => self.update_point(catalog, macros, &new_name, overwrite),
            EditorTab::SearchAreas => self.update_search_area(catalog, macros, &new_name, overwrite),
            EditorTab::Masks => self.update_mask(catalog, &new_name, overwrite),
            EditorTab::Collections => {
                self.update_collection(catalog, macros, &new_name, overwrite)
            }
        };

        match result {
            Ok(()) => {
                if matches!(self.tab, EditorTab::Points | EditorTab::SearchAreas) {
                    if let Some(old) = old_entity.as_deref() {
                        previews.invalidate_entity(old);
                    }
                    previews.invalidate_entity(&new_name);
                }
                if let Err(e) = self.persist(db, macros, catalog) {
                    self.set_err(e);
                } else {
                    self.load_form(catalog);
                    self.set_ok("Saved.");
                }
            }
            Err(e) => self.set_err(e.to_string()),
        }
    }

    fn update_item(
        &mut self,
        catalog: &mut ProgramCatalog,
        macros: &mut Vec<Macro>,
        new_name: &str,
        overwrite: bool,
    ) -> Result<(), sqyre_persist::PersistError> {
        let prog = self
            .selected_program
            .clone()
            .ok_or_else(|| sqyre_persist::PersistError::Message("no program".into()))?;
        let cols = parse_i32(&self.form_cols).unwrap_or(1);
        let rows = parse_i32(&self.form_rows).unwrap_or(1);
        let stack = parse_i32(&self.form_stack_max).unwrap_or(0);
        let item = ProgramItem {
            name: new_name.to_string(),
            mask: self.form_mask.clone(),
            stack_max: stack,
            grid_cols: cols,
            grid_rows: rows,
            tags: self.form_tags.clone(),
        };
        if let Some(old) = self.selected_entity.clone() {
            if old != new_name {
                if overwrite {
                    let _ = catalog.delete_item(&prog, new_name);
                }
                catalog.rename_item(&prog, &old, new_name)?;
                for m in macros.iter_mut() {
                    m.rename_program_entity(ProgramEntityKind::Item, &prog, &old, new_name);
                }
                self.selected_entity = Some(new_name.to_string());
            }
            catalog.upsert_item(&prog, item)?;
        } else {
            catalog.upsert_item(&prog, item)?;
            self.selected_entity = Some(new_name.to_string());
        }
        // Optional icon copy
        let src = self.form_icon_path.trim();
        if !src.is_empty() {
            let dest_dir = catalog.icons_dir(&prog);
            let _ = std::fs::create_dir_all(&dest_dir);
            let dest = dest_dir.join(format!("{new_name}.png"));
            if let Err(e) = std::fs::copy(src, &dest) {
                return Err(sqyre_persist::PersistError::Message(format!(
                    "copy icon: {e}"
                )));
            }
            self.form_icon_path.clear();
        }
        Ok(())
    }

    fn update_point(
        &mut self,
        catalog: &mut ProgramCatalog,
        macros: &mut Vec<Macro>,
        new_name: &str,
        overwrite: bool,
    ) -> Result<(), sqyre_persist::PersistError> {
        let prog = self
            .selected_program
            .clone()
            .ok_or_else(|| sqyre_persist::PersistError::Message("no program".into()))?;
        let pt = ProgramPoint {
            name: new_name.to_string(),
            x: parse_scalar(&self.form_x),
            y: parse_scalar(&self.form_y),
        };
        if let Some(old) = self.selected_entity.clone() {
            if old != new_name {
                if overwrite {
                    let _ = catalog.delete_point(&prog, new_name);
                }
                catalog.rename_point(&prog, &old, new_name)?;
                for m in macros.iter_mut() {
                    m.rename_program_entity(ProgramEntityKind::Point, &prog, &old, new_name);
                }
                self.selected_entity = Some(new_name.to_string());
            }
            catalog.upsert_point(&prog, pt)?;
        } else {
            catalog.upsert_point(&prog, pt)?;
            self.selected_entity = Some(new_name.to_string());
        }
        Ok(())
    }

    fn update_search_area(
        &mut self,
        catalog: &mut ProgramCatalog,
        macros: &mut Vec<Macro>,
        new_name: &str,
        overwrite: bool,
    ) -> Result<(), sqyre_persist::PersistError> {
        let prog = self
            .selected_program
            .clone()
            .ok_or_else(|| sqyre_persist::PersistError::Message("no program".into()))?;
        let sa = ProgramSearchArea {
            name: new_name.to_string(),
            left_x: parse_scalar(&self.form_left),
            top_y: parse_scalar(&self.form_top),
            right_x: parse_scalar(&self.form_right),
            bottom_y: parse_scalar(&self.form_bottom),
        };
        if let Some(old) = self.selected_entity.clone() {
            if old != new_name {
                if overwrite {
                    let _ = catalog.delete_search_area(&prog, new_name);
                }
                catalog.rename_search_area(&prog, &old, new_name)?;
                for m in macros.iter_mut() {
                    m.rename_program_entity(ProgramEntityKind::SearchArea, &prog, &old, new_name);
                }
                self.selected_entity = Some(new_name.to_string());
            }
            catalog.upsert_search_area(&prog, sa)?;
        } else {
            catalog.upsert_search_area(&prog, sa)?;
            self.selected_entity = Some(new_name.to_string());
        }
        Ok(())
    }

    fn update_mask(
        &mut self,
        catalog: &mut ProgramCatalog,
        new_name: &str,
        overwrite: bool,
    ) -> Result<(), sqyre_persist::PersistError> {
        let prog = self
            .selected_program
            .clone()
            .ok_or_else(|| sqyre_persist::PersistError::Message("no program".into()))?;
        let shape = if self.form_shape == "circle" {
            "circle"
        } else {
            "rectangle"
        };
        let mask = ProgramMask {
            name: new_name.to_string(),
            shape: shape.into(),
            center_x: self.form_center_x.trim().to_string(),
            center_y: self.form_center_y.trim().to_string(),
            base: self.form_base.trim().to_string(),
            height: self.form_height.trim().to_string(),
            radius: self.form_radius.trim().to_string(),
            inverse: self.form_inverse,
        };
        if let Some(old) = self.selected_entity.clone() {
            if old != new_name {
                if overwrite {
                    let _ = catalog.delete_mask(&prog, new_name);
                }
                catalog.rename_mask(&prog, &old, new_name)?;
                self.selected_entity = Some(new_name.to_string());
            }
            catalog.upsert_mask(&prog, mask)?;
        } else {
            catalog.upsert_mask(&prog, mask)?;
            self.selected_entity = Some(new_name.to_string());
        }
        Ok(())
    }

    fn update_collection(
        &mut self,
        catalog: &mut ProgramCatalog,
        macros: &mut Vec<Macro>,
        new_name: &str,
        overwrite: bool,
    ) -> Result<(), sqyre_persist::PersistError> {
        let prog = self
            .selected_program
            .clone()
            .ok_or_else(|| sqyre_persist::PersistError::Message("no program".into()))?;
        let rows = parse_i32(&self.form_rows).unwrap_or(1).max(1);
        let cols = parse_i32(&self.form_cols).unwrap_or(1).max(1);
        let col = ProgramCollection {
            name: new_name.to_string(),
            search_area: self.form_search_area.trim().to_string(),
            rows,
            cols,
        };
        if let Some(old) = self.selected_entity.clone() {
            if old != new_name {
                if overwrite {
                    let _ = catalog.delete_collection(&prog, new_name);
                }
                catalog.rename_collection(&prog, &old, new_name)?;
                for m in macros.iter_mut() {
                    m.rename_program_entity(ProgramEntityKind::Collection, &prog, &old, new_name);
                }
                self.selected_entity = Some(new_name.to_string());
            }
            catalog.upsert_collection(&prog, col)?;
        } else {
            catalog.upsert_collection(&prog, col)?;
            self.selected_entity = Some(new_name.to_string());
        }
        Ok(())
    }

    fn on_delete(
        &mut self,
        db: &mut Database,
        macros: &mut Vec<Macro>,
        catalog: &mut ProgramCatalog,
        previews: &mut PreviewTooltipCache,
    ) {
        self.clear_status();
        let deleted_name = self.selected_entity.clone();
        let result = match self.tab {
            EditorTab::Programs => {
                let Some(name) = self.selected_program.clone() else {
                    return;
                };
                catalog.delete_program(&name).map(|_| {
                    self.selected_program = None;
                    self.form_name.clear();
                })
            }
            EditorTab::Items => {
                let (Some(prog), Some(name)) = (
                    self.selected_program.clone(),
                    self.selected_entity.clone(),
                ) else {
                    return;
                };
                catalog.delete_item(&prog, &name).map(|_| {
                    self.selected_entity = None;
                    self.reset_item_form();
                })
            }
            EditorTab::Points => {
                let (Some(prog), Some(name)) = (
                    self.selected_program.clone(),
                    self.selected_entity.clone(),
                ) else {
                    return;
                };
                catalog.delete_point(&prog, &name).map(|_| {
                    self.selected_entity = None;
                    self.form_name.clear();
                })
            }
            EditorTab::SearchAreas => {
                let (Some(prog), Some(name)) = (
                    self.selected_program.clone(),
                    self.selected_entity.clone(),
                ) else {
                    return;
                };
                catalog.delete_search_area(&prog, &name).map(|_| {
                    self.selected_entity = None;
                    self.form_name.clear();
                })
            }
            EditorTab::Masks => {
                let (Some(prog), Some(name)) = (
                    self.selected_program.clone(),
                    self.selected_entity.clone(),
                ) else {
                    return;
                };
                catalog.delete_mask(&prog, &name).map(|_| {
                    self.selected_entity = None;
                    self.reset_mask_form();
                })
            }
            EditorTab::Collections => {
                let (Some(prog), Some(name)) = (
                    self.selected_program.clone(),
                    self.selected_entity.clone(),
                ) else {
                    return;
                };
                catalog.delete_collection(&prog, &name).map(|_| {
                    self.selected_entity = None;
                    self.reset_collection_form();
                })
            }
        };
        match result {
            Ok(()) => {
                if matches!(self.tab, EditorTab::Points | EditorTab::SearchAreas) {
                    if let Some(name) = deleted_name.as_deref() {
                        previews.invalidate_entity(name);
                    }
                }
                if let Err(e) = self.persist(db, macros, catalog) {
                    self.set_err(e);
                } else {
                    self.set_ok("Deleted.");
                }
            }
            Err(e) => self.set_err(e.to_string()),
        }
    }

    fn persist(
        &mut self,
        db: &mut Database,
        macros: &[Macro],
        catalog: &mut ProgramCatalog,
    ) -> Result<(), String> {
        db.set_programs_from_catalog(catalog);
        db.replace_macros(macros.iter().cloned());
        db.save_default().map_err(|e| e.to_string())?;
        *catalog = db.program_catalog().map_err(|e| e.to_string())?;
        Ok(())
    }

    fn set_ok(&mut self, msg: impl Into<String>) {
        self.status = Some(msg.into());
        self.status_error = false;
    }

    fn set_err(&mut self, msg: impl Into<String>) {
        self.status = Some(msg.into());
        self.status_error = true;
    }

    fn clear_status(&mut self) {
        self.status = None;
        self.status_error = false;
    }
}

fn scalar_to_edit(v: &ScalarValue) -> String {
    v.as_display()
}

fn parse_scalar(s: &str) -> ScalarValue {
    let s = s.trim();
    if s.is_empty() {
        return ScalarValue::Null;
    }
    if let Ok(i) = s.parse::<i64>() {
        return ScalarValue::Int(i);
    }
    if let Ok(f) = s.parse::<f64>() {
        return ScalarValue::Float(f);
    }
    ScalarValue::String(s.to_string())
}

fn parse_i32(s: &str) -> Option<i32> {
    s.trim().parse().ok()
}

fn unique_name(base: &str, exists: impl Fn(&str) -> bool) -> String {
    if !exists(base) {
        return base.to_string();
    }
    for i in 2..10_000 {
        let candidate = format!("{base} {i}");
        if !exists(&candidate) {
            return candidate;
        }
    }
    format!("{base} {}", uuid_simple())
}

fn uuid_simple() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let n = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    format!("{n}")
}

fn form_coord_i32(s: &str) -> i32 {
    let s = s.trim();
    if let Ok(i) = s.parse::<i32>() {
        return i;
    }
    if let Ok(f) = s.parse::<f64>() {
        return f as i32;
    }
    0
}

fn paint_preview_toolbar(ui: &mut egui::Ui) -> bool {
    ui.add_space(8.0);
    ui.separator();
    let mut force = false;
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new("Preview").strong());
        if ui.button("Refresh").clicked() {
            force = true;
        }
    });
    force
}

fn paint_disk_preview(
    ui: &mut egui::Ui,
    icons: &mut IconCache,
    path: Option<&std::path::Path>,
    fallback: Option<egui::TextureHandle>,
    title: &str,
    grid: Option<(i32, i32)>,
    replace_clicked: Option<&mut bool>,
) {
    ui.add_space(8.0);
    ui.separator();
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new(title).strong());
        if let Some(path) = path {
            if ui.button("Refresh").clicked() {
                icons.invalidate_path(path);
            }
        }
        if let Some(flag) = replace_clicked {
            if ui.button("Replace Image").clicked() {
                *flag = true;
            }
        }
    });
    let tex = path
        .and_then(|p| icons.for_path(ui.ctx(), p))
        .or(fallback);
    match tex {
        Some(tex) => {
            let [tw, th] = tex.size();
            let size = fit_panel(tw as f32, th as f32);
            let resp = ui.add(egui::Image::new((tex.id(), size)));
            if let Some((rows, cols)) = grid {
                paint_grid_overlay(ui, resp.rect, rows, cols);
            }
            if let Some(path) = path {
                if path.is_file() {
                    ui.weak(path.display().to_string());
                }
            }
        }
        None => {
            ui.weak("No image on disk.");
        }
    }
}

fn show_file_hover(
    ui: &mut egui::Ui,
    response: &egui::Response,
    icons: &mut IconCache,
    path: &std::path::Path,
    label: &str,
) {
    if !response.hovered() {
        return;
    }
    match icons.for_path(ui.ctx(), path) {
        Some(tex) => {
            response.clone().on_hover_ui(|ui| {
                let [tw, th] = tex.size();
                let size = fit_panel(tw as f32, th as f32);
                ui.add(egui::Image::new((tex.id(), size)));
                ui.label(label);
            });
        }
        None => {
            response.clone().on_hover_text(label);
        }
    }
}

fn fit_panel(w: f32, h: f32) -> egui::Vec2 {
    const MAX_W: f32 = 520.0;
    const MAX_H: f32 = 360.0;
    let w = w.max(1.0);
    let h = h.max(1.0);
    let scale = (MAX_W / w).min(MAX_H / h).min(1.0);
    egui::vec2(w * scale, h * scale)
}

fn paint_grid_overlay(ui: &mut egui::Ui, rect: egui::Rect, rows: i32, cols: i32) {
    let rows = rows.max(1) as f32;
    let cols = cols.max(1) as f32;
    let stroke = egui::Stroke::new(1.0, egui::Color32::from_rgb(255, 80, 80));
    let painter = ui.painter();
    for i in 1..rows as i32 {
        let y = rect.top() + rect.height() * (i as f32) / rows;
        painter.hline(rect.x_range(), y, stroke);
    }
    for i in 1..cols as i32 {
        let x = rect.left() + rect.width() * (i as f32) / cols;
        painter.vline(x, rect.y_range(), stroke);
    }
    painter.rect_stroke(rect, 0.0, stroke, egui::StrokeKind::Outside);
}
