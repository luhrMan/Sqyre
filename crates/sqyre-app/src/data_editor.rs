//! Floating Data Editor: Programs / Items / Points / Search Areas / Masks / Collections / AutoPic.

use crate::collection_capture::capture_and_save_collection_image;
use crate::icon_cache::IconCache;
use crate::icon_variants::{self, AddVariantError};
use crate::image_view::{self, ImageViewTransform};
use crate::preview_tooltip::{PreviewKind, PreviewTooltipCache};
use crate::theme;
use crate::var_pills;
use eframe::egui;
use sqyre_domain::{
    collect_known_variable_names, Macro, ProgramEntityKind, ScalarValue, PROGRAM_DELIMITER,
};
use sqyre_hotkeys::ScreenClickBridge;
use sqyre_persist::{
    auto_pic_path, Database, OverlayButtonConfig, ProgramCatalog, ProgramCollection, ProgramItem,
    ProgramMask, ProgramPoint, ProgramSearchArea, UserSettings, DEFAULT_OVERLAY_BUTTON_SIZE,
    MAX_OVERLAY_BUTTON_SIZE, MIN_OVERLAY_BUTTON_SIZE,
};
use sqyre_validate::{
    validate_entity_name, validate_item_grid_fields, validate_numeric_expression,
    validate_search_area_literal_bounds, EntryValidation,
};
use sqyre_vision::invalidate_search_masks_under;
use std::collections::HashSet;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::overlay_icons;
use crate::pickers::{self, ActivePicker, PickerResult};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EditorTab {
    Programs,
    Items,
    Points,
    SearchAreas,
    Masks,
    Collections,
    AutoPic,
    Overlay,
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
    DeleteVariant {
        variant: String,
    },
    OverwriteVariant {
        variant: String,
        source: PathBuf,
    },
}

#[derive(Debug, Clone)]
enum VariantPrompt {
    /// Ask for a name before adding a non-first variant.
    Name { source: PathBuf },
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
    form_shape: String,
    form_center_x: String,
    form_center_y: String,
    form_base: String,
    form_height: String,
    form_radius: String,
    form_inverse: bool,
    form_search_area: String,
    /// Overlay button form: target macro name.
    form_overlay_macro: String,
    /// Overlay button form: built-in icon id.
    form_overlay_icon: String,
    /// Overlay button form: desktop X position.
    form_overlay_x: f32,
    /// Overlay button form: desktop Y position.
    form_overlay_y: f32,
    /// Overlay button form: size in points.
    form_overlay_size: f32,
    /// Bound OS process path for the selected Program.
    form_process_path: String,
    /// Bound window title for the selected Program.
    form_window_title: String,
    variant_name_draft: String,
    variant_prompt: Option<VariantPrompt>,
    status: Option<String>,
    status_error: bool,
    confirm: Option<PendingConfirm>,
    /// After New Point/Search Area: auto-arm record and persist on capture.
    save_after_record: bool,
    /// Zoom/pan for the collections-tab image preview.
    collection_preview: ImageViewTransform,
    /// `(program, collection)` last shown; reset transform when this changes.
    collection_preview_key: Option<(String, String)>,
    /// Overlay button id whose icon picker popup is open.
    overlay_icon_picker_for: Option<String>,
    /// Filter text for the overlay icon picker.
    overlay_icon_search: String,
    /// Running-window picker for Program process binding.
    window_picker: ActivePicker,
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
            form_shape: "rectangle".into(),
            form_center_x: "50".into(),
            form_center_y: "50".into(),
            form_base: String::new(),
            form_height: String::new(),
            form_radius: String::new(),
            form_inverse: false,
            form_search_area: String::new(),
            form_overlay_macro: String::new(),
            form_overlay_icon: overlay_icons::DEFAULT_ICON_ID.into(),
            form_overlay_x: 48.0,
            form_overlay_y: 48.0,
            form_overlay_size: DEFAULT_OVERLAY_BUTTON_SIZE,
            form_process_path: String::new(),
            form_window_title: String::new(),
            variant_name_draft: String::new(),
            variant_prompt: None,
            status: None,
            status_error: false,
            confirm: None,
            save_after_record: false,
            collection_preview: ImageViewTransform::default(),
            collection_preview_key: None,
            overlay_icon_picker_for: None,
            overlay_icon_search: String::new(),
            window_picker: ActivePicker::None,
        }
    }
}

impl DataEditor {
    /// Live Overlay-tab form as an on-screen button preview (position, size, icon, label).
    ///
    /// Shown while a button is selected for editing, even before Update is clicked.
    pub fn overlay_edit_preview(&self) -> Option<OverlayButtonConfig> {
        if !self.open || !matches!(self.tab, EditorTab::Overlay) {
            return None;
        }
        let id = self.selected_entity.as_ref()?;
        Some(OverlayButtonConfig {
            id: id.clone(),
            program: self.selected_program.clone().unwrap_or_default(),
            label: self.form_name.clone(),
            macro_name: self.form_overlay_macro.clone(),
            icon: self.form_overlay_icon.clone(),
            x: self.form_overlay_x,
            y: self.form_overlay_y,
            size: self.form_overlay_size,
        })
    }

    pub fn show(
        &mut self,
        ctx: &egui::Context,
        db: &mut Database,
        macros: &mut Vec<Macro>,
        selected_macro: usize,
        catalog: &mut ProgramCatalog,
        icons: &mut IconCache,
        previews: &mut PreviewTooltipCache,
        screen_click: &ScreenClickBridge,
        settings: &mut UserSettings,
    ) {
        if !self.open {
            return;
        }
        self.poll_screen_click(screen_click, previews, db, macros, catalog, settings);
        let mut open = self.open;
        egui::Window::new("Data Editor")
            .open(&mut open)
            .default_size([880.0, 560.0])
            .min_size([520.0, 280.0])
            // No huge max_size — egui auto-expands toward max when content min_size ratchets.
            .resizable(true)
            .constrain(true)
            .show(ctx, |ui| {
                self.ui(
                    ui,
                    db,
                    macros,
                    selected_macro,
                    catalog,
                    icons,
                    previews,
                    screen_click,
                    settings,
                );
            });
        self.open = open;
        self.draw_overlay_icon_picker(ctx, settings);
        self.poll_window_picker(ctx, catalog, icons, previews, macros);
    }

    fn poll_window_picker(
        &mut self,
        ctx: &egui::Context,
        catalog: &ProgramCatalog,
        icons: &mut IconCache,
        previews: &mut PreviewTooltipCache,
        macros: &[Macro],
    ) {
        if !self.window_picker.is_open() {
            return;
        }
        let macro_opts: Vec<(String, Vec<String>)> = macros
            .iter()
            .map(|m| (m.name.clone(), m.tags.clone()))
            .collect();
        match pickers::show_active_picker(
            ctx,
            &mut self.window_picker,
            catalog,
            icons,
            previews,
            &macro_opts,
        ) {
            PickerResult::Window {
                process_path,
                window_title,
            } => {
                self.form_process_path = process_path;
                self.form_window_title = window_title;
            }
            _ => {}
        }
    }

    fn poll_screen_click(
        &mut self,
        screen_click: &ScreenClickBridge,
        previews: &mut PreviewTooltipCache,
        db: &mut Database,
        macros: &mut Vec<Macro>,
        catalog: &mut ProgramCatalog,
        settings: &mut UserSettings,
    ) {
        let mut captured = false;
        if let Some((x, y)) = screen_click.take_point() {
            self.form_x = x.to_string();
            self.form_y = y.to_string();
            previews.invalidate_entity(self.form_name.trim());
            self.set_ok(format!("Recorded point ({x}, {y})."));
            captured = true;
        }
        if let Some((lx, ty, rx, by)) = screen_click.take_search_area() {
            self.form_left = lx.to_string();
            self.form_top = ty.to_string();
            self.form_right = rx.to_string();
            self.form_bottom = by.to_string();
            previews.invalidate_entity(self.form_name.trim());
            self.set_ok(format!("Recorded search area ({lx},{ty})–({rx},{by})."));
            captured = true;
        }
        if screen_click.take_cancelled() {
            self.save_after_record = false;
            self.set_ok("Recording cancelled.");
        }
        if captured && self.save_after_record {
            self.save_after_record = false;
            self.on_update(db, macros, catalog, previews, settings);
            if !self.status_error {
                self.set_ok("Recorded and saved.");
            }
        }
    }

    fn ui(
        &mut self,
        ui: &mut egui::Ui,
        db: &mut Database,
        macros: &mut Vec<Macro>,
        selected_macro: usize,
        catalog: &mut ProgramCatalog,
        icons: &mut IconCache,
        previews: &mut PreviewTooltipCache,
        screen_click: &ScreenClickBridge,
        settings: &mut UserSettings,
    ) {
        ui.horizontal(|ui| {
            let prev = self.tab;
            ui.selectable_value(&mut self.tab, EditorTab::Programs, "Programs");
            ui.selectable_value(&mut self.tab, EditorTab::Items, "Items");
            ui.selectable_value(&mut self.tab, EditorTab::Points, "Points");
            ui.selectable_value(&mut self.tab, EditorTab::SearchAreas, "Search Areas");
            ui.selectable_value(&mut self.tab, EditorTab::Masks, "Masks");
            ui.selectable_value(&mut self.tab, EditorTab::Collections, "Collections");
            ui.selectable_value(&mut self.tab, EditorTab::AutoPic, "AutoPic");
            ui.selectable_value(&mut self.tab, EditorTab::Overlay, "Overlay");
            if self.tab != prev {
                self.selected_entity = None;
                self.variant_prompt = None;
                self.overlay_icon_picker_for = None;
                self.load_form(catalog, settings);
            }
        });
        ui.separator();

        if let Some(msg) = screen_click.status_label() {
            ui.colored_label(crate::theme::PRIMARY, msg);
            ui.ctx().request_repaint();
        }

        if let Some(msg) = &self.status {
            let color = if self.status_error {
                egui::Color32::from_rgb(220, 80, 80)
            } else {
                egui::Color32::from_rgb(80, 160, 80)
            };
            ui.colored_label(color, msg);
        }

        if let Some(VariantPrompt::Name { source }) = self.variant_prompt.clone() {
            self.draw_variant_name_prompt(ui, catalog, icons, source);
            return;
        }

        if let Some(confirm) = self.confirm.clone() {
            self.draw_confirm(ui, confirm, db, macros, catalog, icons, previews, settings);
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
                        self.draw_left_list(ui, catalog, icons, previews, settings);
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
                                self.draw_form(
                                    ui,
                                    catalog,
                                    icons,
                                    previews,
                                    screen_click,
                                    macros,
                                    macros.get(selected_macro),
                                    settings,
                                );
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
                    let can_new = !matches!(self.tab, EditorTab::AutoPic);
                    if ui
                        .add_enabled(can_new, egui::Button::new("New"))
                        .clicked()
                    {
                        self.on_new(db, macros, catalog, icons, screen_click, settings);
                    }
                    let dirty = self.is_dirty(catalog, settings);
                    let valid = self.form_valid(macros.get(selected_macro));
                    let can_update = !matches!(self.tab, EditorTab::AutoPic);
                    if ui
                        .add_enabled(can_update && dirty && valid, egui::Button::new("Update"))
                        .clicked()
                    {
                        self.on_update(db, macros, catalog, previews, settings);
                    }
                    let can_delete = match self.tab {
                        EditorTab::Programs => self.selected_program.is_some(),
                        EditorTab::AutoPic => false,
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
                            EditorTab::Overlay => format!(
                                "overlay button “{}”",
                                self.selected_entity.as_deref().unwrap_or("")
                            ),
                            EditorTab::AutoPic => String::new(),
                        };
                        if !label.is_empty() {
                            self.confirm = Some(PendingConfirm::Delete { label });
                        }
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
        icons: &mut IconCache,
        previews: &mut PreviewTooltipCache,
        settings: &mut UserSettings,
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
            PendingConfirm::DeleteVariant { variant } => {
                ui.label(format!(
                    "Delete icon variant “{}”? This cannot be undone.",
                    variant_display_label(variant)
                ));
            }
            PendingConfirm::OverwriteVariant { variant, .. } => {
                ui.label(format!(
                    "Variant “{}” already exists. Overwrite it?",
                    variant_display_label(variant)
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
                        self.on_delete(db, macros, catalog, previews, settings);
                    }
                    PendingConfirm::Overwrite { .. } => {
                        self.confirm = None;
                        self.apply_update(db, macros, catalog, true, previews, settings);
                    }
                    PendingConfirm::DeleteVariant { variant } => {
                        self.confirm = None;
                        self.delete_icon_variant(catalog, icons, &variant);
                    }
                    PendingConfirm::OverwriteVariant { variant, source } => {
                        self.confirm = None;
                        self.overwrite_icon_variant(catalog, icons, &variant, &source);
                    }
                }
            }
        });
    }

    fn draw_variant_name_prompt(
        &mut self,
        ui: &mut egui::Ui,
        catalog: &ProgramCatalog,
        icons: &mut IconCache,
        source: PathBuf,
    ) {
        ui.heading("Add Icon Variant");
        ui.label("Variant name");
        ui.add(
            egui::TextEdit::singleline(&mut self.variant_name_draft).desired_width(f32::INFINITY),
        );
        ui.horizontal(|ui| {
            if ui.button("Cancel").clicked() {
                self.variant_prompt = None;
                self.variant_name_draft.clear();
            }
            if ui.button("Add").clicked() {
                let name = self.variant_name_draft.trim().to_string();
                self.variant_prompt = None;
                self.variant_name_draft.clear();
                self.add_icon_variant(catalog, icons, &name, &source);
            }
        });
    }

    fn draw_left_list(
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
                        for prog in catalog.program_names() {
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

    fn entity_names(&self, catalog: &ProgramCatalog, program: &str) -> Vec<String> {
        let Some(p) = catalog.get(program) else {
            return Vec::new();
        };
        let res = catalog.resolution_key();
        let mut keys: Vec<(String, String)> = match self.tab {
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

    fn select_program(&mut self, name: &str, catalog: &ProgramCatalog, settings: &UserSettings) {
        self.selected_program = Some(name.to_string());
        self.selected_entity = None;
        self.load_form(catalog, settings);
    }

    /// Select a program for docs screenshots (Programs tab form populated).
    pub fn select_program_for_docs(&mut self, name: &str, catalog: &ProgramCatalog) {
        self.select_program(name, catalog, &UserSettings::default());
    }

    fn select_entity(
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

    fn load_form(&mut self, catalog: &ProgramCatalog, settings: &UserSettings) {
        self.clear_status();
        match self.tab {
            EditorTab::Programs => {
                self.form_name = self.selected_program.clone().unwrap_or_default();
                if let Some(p) = self
                    .selected_program
                    .as_deref()
                    .and_then(|n| catalog.get(n))
                {
                    self.form_process_path = p.process_path.clone();
                    self.form_window_title = p.window_title.clone();
                } else {
                    self.form_process_path.clear();
                    self.form_window_title.clear();
                }
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
                self.collection_preview_key = Some((prog.to_string(), name.to_string()));
            }
            EditorTab::AutoPic => {
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
            EditorTab::Overlay => {
                self.load_overlay_form(settings);
            }
        }
    }

    fn load_overlay_form(&mut self, settings: &UserSettings) {
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
    }

    fn reset_overlay_form(&mut self) {
        self.form_name.clear();
        self.form_overlay_x = 48.0;
        self.form_overlay_y = 48.0;
        self.form_overlay_macro.clear();
        self.form_overlay_icon = overlay_icons::DEFAULT_ICON_ID.into();
        self.form_overlay_size = DEFAULT_OVERLAY_BUTTON_SIZE;
    }

    fn reset_item_form(&mut self) {
        self.form_name.clear();
        self.form_cols = "1".into();
        self.form_rows = "1".into();
        self.form_stack_max = "0".into();
        self.form_mask.clear();
        self.form_tags.clear();
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
        screen_click: &ScreenClickBridge,
        macros: &[Macro],
        active_macro: Option<&Macro>,
        settings: &mut UserSettings,
    ) {
        let known = active_macro
            .map(collect_known_variable_names)
            .unwrap_or_default();
        let is_dark = ui.visuals().dark_mode;
        match self.tab {
            EditorTab::Programs => {
                ui.heading("Program");
                ui.label("Name");
                ui.add(
                    egui::TextEdit::singleline(&mut self.form_name).desired_width(f32::INFINITY),
                );
                ui.add_space(8.0);
                ui.label("Running program");
                ui.weak(
                    "Overlay buttons for this program show when this process owns the focused window.",
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
                            !self.form_process_path.is_empty() || !self.form_window_title.is_empty(),
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
                    if let (Some(prog), mask) = (
                        self.selected_program.as_deref(),
                        self.form_mask.as_str(),
                    ) {
                        if !mask.is_empty() {
                            if let Some(m) = catalog.get(prog).and_then(|p| p.masks.get(mask)) {
                                let detail = if catalog.mask_image_path(prog, mask).is_file() {
                                    "Image mask on disk".to_string()
                                } else if m.shape == "circle" {
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
                if let (Some(prog), Some(item)) = (
                    self.selected_program.clone(),
                    self.selected_entity.clone(),
                ) {
                    let target = format!("{prog}{PROGRAM_DELIMITER}{item}");
                    self.paint_item_variants_ui(ui, icons, catalog, &target, &item);
                }
            }
            EditorTab::Points => {
                ui.heading("Point");
                self.program_selector(ui, catalog);
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    ui.label("Name");
                    ui.add(
                        egui::TextEdit::singleline(&mut self.form_name)
                            .desired_width(ui.available_width() - 48.0),
                    );
                    let armed = screen_click.is_armed();
                    if theme::record_icon_button(ui, "Click on screen to capture X/Y", !armed)
                        .clicked()
                    {
                        self.save_after_record = false;
                        screen_click.arm_point();
                        self.set_ok("Recording… left-click to capture.");
                    }
                    if armed && ui.button("Cancel").clicked() {
                        self.save_after_record = false;
                        screen_click.disarm();
                    }
                });
                ui.weak("X/Y overlay the preview; integers or ${var}.");
                let x = form_coord_i32(&self.form_x);
                let y = form_coord_i32(&self.form_y);
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
                );
            }
            EditorTab::SearchAreas => {
                ui.heading("Search Area");
                self.program_selector(ui, catalog);
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    ui.label("Name");
                    ui.add(
                        egui::TextEdit::singleline(&mut self.form_name)
                            .desired_width(ui.available_width() - 48.0),
                    );
                    let armed = screen_click.is_armed();
                    if theme::record_icon_button(
                        ui,
                        "Two clicks: opposite corners of the area",
                        !armed,
                    )
                    .clicked()
                    {
                        self.save_after_record = false;
                        screen_click.arm_search_area();
                        self.set_ok("Recording… click two corners.");
                    }
                    if armed && ui.button("Cancel").clicked() {
                        self.save_after_record = false;
                        screen_click.disarm();
                    }
                });
                ui.weak("Bounds overlay the preview edges; integers or ${var}.");
                let lx = form_coord_i32(&self.form_left);
                let ty = form_coord_i32(&self.form_top);
                let rx = form_coord_i32(&self.form_right);
                let by = form_coord_i32(&self.form_bottom);
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
                );
            }
            EditorTab::Masks => {
                ui.heading("Mask");
                self.program_selector(ui, catalog);
                ui.add_space(4.0);
                ui.label("Name");
                ui.add(
                    egui::TextEdit::singleline(&mut self.form_name).desired_width(f32::INFINITY),
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
                        .clicked()
                    {
                        self.upload_mask_image(catalog, icons);
                    }
                    if ui
                        .add_enabled(has_image, egui::Button::new("Remove Image"))
                        .clicked()
                    {
                        self.remove_mask_image(catalog, icons);
                    }
                });
                if has_image {
                    ui.weak("Image mask mode — shape fields hidden while a PNG is on disk.");
                } else {
                    ui.add_space(4.0);
                    ui.label("Shape");
                    ui.horizontal(|ui| {
                        ui.selectable_value(&mut self.form_shape, "rectangle".into(), "Rectangle");
                        ui.selectable_value(&mut self.form_shape, "circle".into(), "Circle");
                    });
                    ui.checkbox(
                        &mut self.form_inverse,
                        "Inverse (shape included, rest excluded)",
                    );
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
                    let lx = form_coord_i32(&self.form_left);
                    let ty = form_coord_i32(&self.form_top);
                    let rx = form_coord_i32(&self.form_right);
                    let by = form_coord_i32(&self.form_bottom);
                    let force = paint_preview_toolbar(ui);
                    previews.paint_search_area_panel(ui, lx, ty, rx, by, force);
                    ui.add_space(8.0);
                    if ui.button("Save").clicked() {
                        self.save_autopix();
                    }
                    ui.weak(format!("Saves to {}", auto_pic_path().display()));
                } else {
                    ui.weak("Select a search area from the list.");
                }
            }
            EditorTab::Overlay => {
                ui.heading("Overlay Button");
                ui.weak(
                    "Buttons appear when this program's bound process owns the focused OS window. Bind a process on the Programs tab.",
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
                    let preview = overlay_icons::icon_glyph_button(ui, icon, false, 48.0);
                    if preview.clicked() {
                        if let Some(id) = self.selected_entity.clone() {
                            self.overlay_icon_search.clear();
                            self.overlay_icon_picker_for = Some(id);
                        }
                    }
                    ui.vertical(|ui| {
                        ui.label(icon.label);
                        ui.weak("Click icon to choose from Phosphor library");
                    });
                });
                ui.add_space(6.0);
                ui.label("Label");
                ui.add(
                    egui::TextEdit::singleline(&mut self.form_name)
                        .desired_width(f32::INFINITY)
                        .hint_text("optional"),
                );
                ui.add_space(4.0);
                ui.label("Macro");
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
                    });
                if selected != before {
                    self.form_overlay_macro = selected;
                }
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    ui.label("X");
                    ui.add(
                        egui::DragValue::new(&mut self.form_overlay_x)
                            .speed(1.0)
                            .suffix(" px"),
                    );
                    ui.label("Y");
                    ui.add(
                        egui::DragValue::new(&mut self.form_overlay_y)
                            .speed(1.0)
                            .suffix(" px"),
                    );
                    ui.label("Size");
                    ui.add(
                        egui::DragValue::new(&mut self.form_overlay_size)
                            .speed(1)
                            .range(MIN_OVERLAY_BUTTON_SIZE..=MAX_OVERLAY_BUTTON_SIZE),
                    );
                });
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

    fn is_dirty(&self, catalog: &ProgramCatalog, settings: &UserSettings) -> bool {
        match self.tab {
            EditorTab::Programs => {
                let Some(sel) = self.selected_program.as_deref() else {
                    return !self.form_name.trim().is_empty()
                        || !self.form_process_path.is_empty()
                        || !self.form_window_title.is_empty();
                };
                let bound = catalog.get(sel);
                let path = bound.map(|p| p.process_path.as_str()).unwrap_or("");
                let title = bound.map(|p| p.window_title.as_str()).unwrap_or("");
                self.form_name.trim() != sel
                    || self.form_process_path != path
                    || self.form_window_title != title
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
            EditorTab::AutoPic => false,
            EditorTab::Overlay => {
                let Some(id) = self.selected_entity.as_deref() else {
                    return false;
                };
                let Some(btn) = settings.overlay_buttons.iter().find(|b| b.id == id) else {
                    return true;
                };
                self.form_name.trim() != btn.label.trim()
                    || self.form_overlay_macro.trim() != btn.macro_name.trim()
                    || self.form_overlay_icon != btn.icon
                    || (self.form_overlay_x - btn.x).abs() > f32::EPSILON
                    || (self.form_overlay_y - btn.y).abs() > f32::EPSILON
                    || (self.form_overlay_size - btn.size).abs() > f32::EPSILON
                    || self.selected_program.as_deref() != Some(btn.program.as_str())
            }
        }
    }

    fn form_valid(&self, active_macro: Option<&Macro>) -> bool {
        if !matches!(self.tab, EditorTab::Overlay)
            && validate_entity_name(self.form_name.trim()).is_err()
        {
            return false;
        }
        match self.tab {
            EditorTab::Programs => true,
            EditorTab::Items => {
                self.selected_program.is_some()
                    && validate_item_grid_fields(
                        &self.form_cols,
                        &self.form_rows,
                        &self.form_stack_max,
                    )
                    .is_ok()
            }
            EditorTab::Points => {
                self.selected_program.is_some()
                    && !self.form_x.trim().is_empty()
                    && !self.form_y.trim().is_empty()
                    && !validate_numeric_expression(&self.form_x, active_macro).blocks_submit()
                    && !validate_numeric_expression(&self.form_y, active_macro).blocks_submit()
            }
            EditorTab::SearchAreas => {
                self.selected_program.is_some()
                    && !self.form_left.trim().is_empty()
                    && !self.form_top.trim().is_empty()
                    && !self.form_right.trim().is_empty()
                    && !self.form_bottom.trim().is_empty()
                    && validate_search_area_literal_bounds(
                        &self.form_left,
                        &self.form_top,
                        &self.form_right,
                        &self.form_bottom,
                    )
                    .is_ok()
                    && !validate_numeric_expression(&self.form_left, active_macro).blocks_submit()
                    && !validate_numeric_expression(&self.form_top, active_macro).blocks_submit()
                    && !validate_numeric_expression(&self.form_right, active_macro).blocks_submit()
                    && !validate_numeric_expression(&self.form_bottom, active_macro).blocks_submit()
            }
            EditorTab::Masks => {
                if self.selected_program.is_none()
                    || (self.form_shape != "rectangle" && self.form_shape != "circle")
                    || self.form_center_x.trim().is_empty()
                    || self.form_center_y.trim().is_empty()
                {
                    return false;
                }
                if validate_numeric_expression(&self.form_center_x, active_macro).blocks_submit()
                    || validate_numeric_expression(&self.form_center_y, active_macro).blocks_submit()
                {
                    return false;
                }
                if self.form_shape == "circle" {
                    !validate_numeric_expression(&self.form_radius, active_macro).blocks_submit()
                } else {
                    !validate_numeric_expression(&self.form_base, active_macro).blocks_submit()
                        && !validate_numeric_expression(&self.form_height, active_macro)
                            .blocks_submit()
                }
            }
            EditorTab::Collections => {
                self.selected_program.is_some()
                    && !self.form_search_area.trim().is_empty()
                    && parse_i32(&self.form_rows).map(|n| n >= 1).unwrap_or(false)
                    && parse_i32(&self.form_cols).map(|n| n >= 1).unwrap_or(false)
            }
            EditorTab::AutoPic => false,
            EditorTab::Overlay => {
                self.selected_program.is_some()
                    && self.selected_entity.is_some()
                    && !self.form_overlay_macro.trim().is_empty()
            }
        }
    }

    fn on_new(
        &mut self,
        db: &mut Database,
        macros: &mut Vec<Macro>,
        catalog: &mut ProgramCatalog,
        icons: &mut IconCache,
        screen_click: &ScreenClickBridge,
        settings: &mut UserSettings,
    ) {
        self.clear_status();
        self.save_after_record = false;
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
                        self.form_process_path.clear();
                        self.form_window_title.clear();
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
                        self.load_form(catalog, settings);
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
                        self.load_form(catalog, settings);
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
                        self.load_form(catalog, settings);
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
                        self.load_form(catalog, settings);
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
                            self.load_form(catalog, settings);
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
            EditorTab::AutoPic => {
                self.set_err("Use Save on the AutoPic tab to capture a search area.");
                return;
            }
            EditorTab::Overlay => {
                let Some(prog) = self.selected_program.clone() else {
                    self.set_err("Select a program first.");
                    return;
                };
                let n = settings
                    .overlay_buttons
                    .iter()
                    .filter(|b| b.program == prog)
                    .count();
                let mut btn = OverlayButtonConfig::new(new_overlay_button_id(), &prog);
                btn.icon = overlay_icons::DEFAULT_ICON_ID.into();
                btn.x = 48.0 + (n as f32) * 60.0;
                btn.y = 48.0;
                btn.size = DEFAULT_OVERLAY_BUTTON_SIZE;
                if let Some(first) = macros.first() {
                    btn.macro_name = first.name.clone();
                    btn.label = first.name.clone();
                } else {
                    btn.label = format!("Button {}", n + 1);
                }
                let id = btn.id.clone();
                settings.overlay_buttons.push(btn);
                self.selected_entity = Some(id);
                self.load_form(catalog, settings);
                self.persist_overlay_settings(settings);
                self.set_ok("Created overlay button.");
                return;
            }
        };
        match created {
            Ok(msg) => {
                if let Err(e) = self.persist(db, macros, catalog) {
                    self.set_err(e);
                } else {
                    match self.tab {
                        EditorTab::Points if !screen_click.is_armed() => {
                            self.save_after_record = true;
                            screen_click.arm_point();
                            self.set_ok(format!(
                                "{msg} Recording… left-click to capture X/Y."
                            ));
                        }
                        EditorTab::SearchAreas if !screen_click.is_armed() => {
                            self.save_after_record = true;
                            screen_click.arm_search_area();
                            self.set_ok(format!(
                                "{msg} Recording… click two corners."
                            ));
                        }
                        _ => self.set_ok(msg),
                    }
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
        settings: &mut UserSettings,
    ) {
        // Check overwrite for renames onto existing keys
        if let Some((kind, name)) = self.would_overwrite(catalog) {
            self.confirm = Some(PendingConfirm::Overwrite { kind, name });
            return;
        }
        self.apply_update(db, macros, catalog, false, previews, settings);
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
            EditorTab::AutoPic => {}
            EditorTab::Overlay => {}
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
        settings: &mut UserSettings,
    ) {
        self.clear_status();
        if matches!(self.tab, EditorTab::Overlay) {
            self.apply_overlay_update(settings);
            return;
        }
        let new_name = self.form_name.trim().to_string();
        if validate_entity_name(&new_name).is_err() {
            self.set_err("Invalid name.");
            return;
        }

        let old_entity = self.selected_entity.clone();
        let mut overlay_program_renamed = false;
        let result = match self.tab {
            EditorTab::Programs => {
                if let Some(old) = self.selected_program.clone() {
                    if old == new_name {
                        catalog
                            .set_process_binding(
                                &old,
                                self.form_process_path.clone(),
                                self.form_window_title.clone(),
                            )
                            .map(|_| ())
                    } else {
                        if overwrite {
                            let _ = catalog.delete_program(&new_name);
                        }
                        catalog.rename_program(&old, &new_name).map(|_| {
                            let _ = catalog.set_process_binding(
                                &new_name,
                                self.form_process_path.clone(),
                                self.form_window_title.clone(),
                            );
                            for m in macros.iter_mut() {
                                m.rename_program(&old, &new_name);
                            }
                            for btn in settings.overlay_buttons.iter_mut() {
                                if btn.program == old {
                                    btn.program = new_name.clone();
                                }
                            }
                            overlay_program_renamed = true;
                            self.selected_program = Some(new_name.clone());
                        })
                    }
                } else {
                    catalog.create_program(&new_name).map(|_| {
                        let _ = catalog.set_process_binding(
                            &new_name,
                            self.form_process_path.clone(),
                            self.form_window_title.clone(),
                        );
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
            EditorTab::AutoPic | EditorTab::Overlay => Ok(()),
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
                    if overlay_program_renamed {
                        self.persist_overlay_settings(settings);
                    }
                    self.load_form(catalog, settings);
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
        settings: &mut UserSettings,
    ) {
        self.clear_status();
        if matches!(self.tab, EditorTab::Overlay) {
            let Some(id) = self.selected_entity.clone() else {
                return;
            };
            settings.overlay_buttons.retain(|b| b.id != id);
            if self.overlay_icon_picker_for.as_deref() == Some(id.as_str()) {
                self.overlay_icon_picker_for = None;
            }
            self.selected_entity = None;
            self.reset_overlay_form();
            self.persist_overlay_settings(settings);
            self.set_ok("Deleted overlay button.");
            return;
        }
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
            EditorTab::AutoPic | EditorTab::Overlay => return,
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


    fn paint_item_variants_ui(
        &mut self,
        ui: &mut egui::Ui,
        icons: &mut IconCache,
        catalog: &ProgramCatalog,
        target: &str,
        item: &str,
    ) {
        let paths = catalog.variant_paths(target);
        let names = icon_variants::variant_names(catalog, self.selected_program.as_deref().unwrap_or(""), item);
        ui.add_space(8.0);
        ui.separator();
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("Icon variants").strong());
            if ui
                .add(egui::Button::new(egui::RichText::new("↻").size(14.0)).small())
                .on_hover_text("Refresh")
                .clicked()
            {
                for path in &paths {
                    icons.invalidate_path(path);
                }
            }
            if ui.button("Add Icon Variant").clicked() {
                self.pick_and_add_variant(catalog, icons);
            }
        });
        if paths.is_empty() {
            let fallback = icons.for_target_or_fallback(ui.ctx(), catalog, target);
            let [tw, th] = fallback.size();
            let size = fit_panel(tw as f32, th as f32);
            ui.add(egui::Image::new((fallback.id(), size)));
            ui.weak("No icon variants on disk.");
            return;
        }
        let can_delete = names.len() > 1;
        ui.horizontal_wrapped(|ui| {
            for path in &paths {
                let variant = variant_name_from_path(path, item);
                ui.vertical(|ui| {
                    ui.set_max_width(112.0);
                    match icons.for_path(ui.ctx(), path) {
                        Some(tex) => {
                            let [tw, th] = tex.size();
                            let size = fit_thumbnail(tw as f32, th as f32);
                            ui.add(egui::Image::new((tex.id(), size)));
                        }
                        None => {
                            ui.weak("Missing");
                        }
                    }
                    ui.small(variant_display_label(&variant));
                    let deny = !can_delete || variant == "Original";
                    if ui
                        .add_enabled(!deny, egui::Button::new("Delete").small())
                        .clicked()
                    {
                        self.confirm = Some(PendingConfirm::DeleteVariant {
                            variant: variant.clone(),
                        });
                    }
                });
                ui.add_space(8.0);
            }
        });
    }

    fn pick_and_add_variant(&mut self, catalog: &ProgramCatalog, icons: &mut IconCache) {
        let Some(path) = crate::file_dialogs::pick_png() else {
            return;
        };
        let (Some(prog), Some(item)) = (
            self.selected_program.clone(),
            self.selected_entity.clone(),
        ) else {
            self.set_err("Select an item first.");
            return;
        };
        let existing = icon_variants::variant_names(catalog, &prog, &item);
        if existing.is_empty() {
            self.add_icon_variant(catalog, icons, "Original", &path);
        } else {
            self.variant_name_draft.clear();
            self.variant_prompt = Some(VariantPrompt::Name { source: path });
        }
    }

    fn add_icon_variant(
        &mut self,
        catalog: &ProgramCatalog,
        icons: &mut IconCache,
        name: &str,
        source: &std::path::Path,
    ) {
        let (Some(prog), Some(item)) = (
            self.selected_program.clone(),
            self.selected_entity.clone(),
        ) else {
            self.set_err("Select an item first.");
            return;
        };
        match icon_variants::add_variant(catalog, &prog, &item, name, source) {
            Ok(added) => {
                let path = icon_variants::variant_path(catalog, &prog, &item, &added);
                icons.invalidate_path(&path);
                self.set_ok(format!("Added variant “{added}”."));
            }
            Err(AddVariantError::Exists(e)) => {
                self.confirm = Some(PendingConfirm::OverwriteVariant {
                    variant: e.variant_name,
                    source: source.to_path_buf(),
                });
            }
            Err(AddVariantError::Other(err)) => self.set_err(err),
        }
    }

    fn overwrite_icon_variant(
        &mut self,
        catalog: &ProgramCatalog,
        icons: &mut IconCache,
        variant: &str,
        source: &std::path::Path,
    ) {
        let (Some(prog), Some(item)) = (
            self.selected_program.clone(),
            self.selected_entity.clone(),
        ) else {
            return;
        };
        match icon_variants::overwrite_variant(catalog, &prog, &item, variant, source) {
            Ok(()) => {
                let path = icon_variants::variant_path(catalog, &prog, &item, variant);
                icons.invalidate_path(&path);
                self.set_ok(format!("Overwrote variant “{variant}”."));
            }
            Err(e) => self.set_err(e),
        }
    }

    fn delete_icon_variant(
        &mut self,
        catalog: &ProgramCatalog,
        icons: &mut IconCache,
        variant: &str,
    ) {
        let (Some(prog), Some(item)) = (
            self.selected_program.clone(),
            self.selected_entity.clone(),
        ) else {
            return;
        };
        let names = icon_variants::variant_names(catalog, &prog, &item);
        if names.len() <= 1 {
            self.set_err("Cannot delete the last icon variant.");
            return;
        }
        if variant == "Original" {
            self.set_err("The 'Original' variant cannot be deleted.");
            return;
        }
        match icon_variants::delete_variant(catalog, &prog, &item, variant) {
            Ok(()) => {
                let path = icon_variants::variant_path(catalog, &prog, &item, variant);
                icons.invalidate_path(&path);
                self.set_ok(format!("Deleted variant “{variant}”."));
            }
            Err(e) => self.set_err(e),
        }
    }

    fn upload_mask_image(&mut self, catalog: &ProgramCatalog, icons: &mut IconCache) {
        let (Some(prog), Some(mask)) = (
            self.selected_program.clone(),
            self.selected_entity.clone(),
        ) else {
            self.set_err("Select a mask first.");
            return;
        };
        let Some(src) = crate::file_dialogs::pick_image() else {
            return;
        };
        let dest = catalog.mask_image_path(&prog, &mask);
        if let Some(parent) = dest.parent() {
            if let Err(e) = std::fs::create_dir_all(parent) {
                self.set_err(format!("create mask dir: {e}"));
                return;
            }
        }
        match copy_image_as_png(&src, &dest) {
            Ok(()) => {
                icons.invalidate_path(&dest);
                invalidate_search_masks_under(&dest);
                self.set_ok("Uploaded mask image.");
            }
            Err(e) => self.set_err(e),
        }
    }

    fn remove_mask_image(&mut self, catalog: &ProgramCatalog, icons: &mut IconCache) {
        let (Some(prog), Some(mask)) = (
            self.selected_program.clone(),
            self.selected_entity.clone(),
        ) else {
            return;
        };
        let path = catalog.mask_image_path(&prog, &mask);
        match std::fs::remove_file(&path) {
            Ok(()) => {
                icons.invalidate_path(&path);
                invalidate_search_masks_under(&path);
                self.set_ok("Removed mask image.");
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                icons.invalidate_path(&path);
                invalidate_search_masks_under(&path);
                self.set_ok("Removed mask image.");
            }
            Err(e) => self.set_err(format!("remove mask image: {e}")),
        }
    }

    fn save_autopix(&mut self) {
        let name = self.form_name.trim();
        if name.is_empty() {
            self.set_err("AutoPic: select a search area first.");
            return;
        }
        let lx = form_coord_i32(&self.form_left);
        let ty = form_coord_i32(&self.form_top);
        let rx = form_coord_i32(&self.form_right);
        let by = form_coord_i32(&self.form_bottom);
        let (lx, rx) = if lx <= rx { (lx, rx) } else { (rx, lx) };
        let (ty, by) = if ty <= by { (ty, by) } else { (by, ty) };
        if rx - lx <= 0 || by - ty <= 0 {
            self.set_err("AutoPic: invalid search area dimensions.");
            return;
        }
        use sqyre_executor::{DesktopRect, ScreenCapturer};
        let mut capturer = match sqyre_capture::X11Capturer::open() {
            Ok(c) => c,
            Err(e) => {
                self.set_err(format!("AutoPic: {e}"));
                return;
            }
        };
        let img = match capturer.capture_rect(DesktopRect {
            x: lx,
            y: ty,
            w: rx - lx,
            h: by - ty,
        }) {
            Ok(i) => i,
            Err(e) => {
                self.set_err(format!("AutoPic: {e} (area: {name})"));
                return;
            }
        };
        let dir = auto_pic_path();
        if let Err(e) = std::fs::create_dir_all(&dir) {
            self.set_err(format!("AutoPic: create dir: {e}"));
            return;
        }
        let stamp = {
            use std::time::{SystemTime, UNIX_EPOCH};
            let dur = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default();
            // Timestamp without chrono: YYYYMMDD_HHMMSS UTC.
            let secs = dur.as_secs() as i64;
            let days = secs.div_euclid(86_400);
            let day_secs = secs.rem_euclid(86_400) as u32;
            let hh = day_secs / 3600;
            let mm = (day_secs % 3600) / 60;
            let ss = day_secs % 60;
            // Civil date from Unix days (algorithm from civil_from_days / Howard Hinnant).
            let z = days + 719_468;
            let era = z.div_euclid(146_097);
            let doe = (z - era * 146_097) as u32;
            let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365;
            let y = yoe as i64 + era * 400;
            let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
            let mp = (5 * doy + 2) / 153;
            let d = doy - (153 * mp + 2) / 5 + 1;
            let m = if mp < 10 { mp + 3 } else { mp - 9 };
            let y = if m <= 2 { y + 1 } else { y };
            format!("{y:04}{m:02}{d:02}_{hh:02}{mm:02}{ss:02}")
        };
        let filename = format!("{stamp}_{name}.png");
        let full = dir.join(&filename);
        if let Err(e) = img.save(&full) {
            self.set_err(format!("AutoPic: save {}: {e}", full.display()));
            return;
        }
        self.set_ok(format!("AutoPic: saved {}", full.display()));
    }

    fn clear_status(&mut self) {
        self.status = None;
        self.status_error = false;
    }

    fn persist_overlay_settings(&mut self, settings: &mut UserSettings) {
        settings.clamp();
        if let Err(e) = settings.save_default() {
            self.set_err(format!("Failed to save overlay settings: {e}"));
        } else {
            self.clear_status();
        }
    }

    fn apply_overlay_update(&mut self, settings: &mut UserSettings) {
        let Some(id) = self.selected_entity.clone() else {
            self.set_err("Select an overlay button first.");
            return;
        };
        let Some(prog) = self.selected_program.clone() else {
            self.set_err("Select a program first.");
            return;
        };
        if self.form_overlay_macro.trim().is_empty() {
            self.set_err("Pick a macro.");
            return;
        }
        let Some(btn) = settings.overlay_buttons.iter_mut().find(|b| b.id == id) else {
            self.set_err("Overlay button not found.");
            return;
        };
        btn.program = prog;
        btn.label = self.form_name.trim().to_string();
        btn.macro_name = self.form_overlay_macro.trim().to_string();
        btn.icon = self.form_overlay_icon.clone();
        btn.x = self.form_overlay_x;
        btn.y = self.form_overlay_y;
        btn.size = self.form_overlay_size;
        self.persist_overlay_settings(settings);
        self.set_ok("Saved overlay button.");
    }

    fn draw_overlay_icon_picker(&mut self, ctx: &egui::Context, settings: &mut UserSettings) {
        let Some(button_id) = self.overlay_icon_picker_for.clone() else {
            return;
        };
        if self.selected_entity.as_deref() != Some(button_id.as_str()) {
            self.overlay_icon_picker_for = None;
            return;
        }
        let current = self.form_overlay_icon.clone();
        let mut open = true;
        let mut close = false;
        egui::Window::new("Choose overlay icon")
            .open(&mut open)
            .collapsible(false)
            .resizable(true)
            .default_size([420.0, 480.0])
            .default_pos(egui::pos2(120.0, 80.0))
            .show(ctx, |ui| {
                ui.weak("Phosphor Icons — search by name, then click to select.");
                ui.add_space(4.0);
                if let Some(id) =
                    overlay_icons::show_icon_picker_grid(ui, &current, &mut self.overlay_icon_search)
                {
                    self.form_overlay_icon = id.to_string();
                    close = true;
                }
            });
        if !open || close {
            self.overlay_icon_picker_for = None;
        }
        let _ = settings; // form-edited; persist via Update
    }
}

fn new_overlay_button_id() -> String {
    let ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    format!("btn-{ms}")
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

/// Sorted unique tags across items in a program.
fn collect_program_item_tags(catalog: &ProgramCatalog, program: &str) -> Vec<String> {
    let mut tags: Vec<String> = catalog
        .get(program)
        .map(|p| {
            p.items
                .values()
                .flat_map(|it| it.tags.iter().cloned())
                .collect()
        })
        .unwrap_or_default();
    tags.sort();
    tags.dedup();
    tags
}

fn item_tag_completion_options(
    search: &str,
    on_item: &[String],
    program_tags: &[String],
    limit: usize,
) -> Vec<String> {
    let search_l = search.trim().to_lowercase();
    if search_l.is_empty() {
        return Vec::new();
    }
    program_tags
        .iter()
        .filter(|t| !on_item.iter().any(|c| c == *t))
        .filter(|t| t.to_lowercase().contains(&search_l))
        .take(limit)
        .cloned()
        .collect()
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

fn copy_image_as_png(src: &std::path::Path, dest: &std::path::Path) -> Result<(), String> {
    let bytes = std::fs::read(src).map_err(|e| format!("read: {e}"))?;
    if bytes.starts_with(&[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]) {
        std::fs::write(dest, &bytes).map_err(|e| format!("write: {e}"))?;
        return Ok(());
    }
    let img = image::load_from_memory(&bytes).map_err(|e| format!("decode: {e}"))?;
    img.save(dest).map_err(|e| format!("save png: {e}"))
}

fn paint_preview_toolbar(ui: &mut egui::Ui) -> bool {
    ui.add_space(8.0);
    ui.separator();
    let mut force = false;
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new("Preview").strong());
        if ui
            .add(egui::Button::new(egui::RichText::new("↻").size(14.0)).small())
            .on_hover_text("Refresh")
            .clicked()
        {
            force = true;
        }
    });
    force
}

#[derive(Clone, Copy)]
enum CardinalEdge {
    Top,
    Bottom,
    Left,
    Right,
}

/// Borderless coord chip overlaid on a preview edge.
fn paint_preview_coord_chip(
    ui: &mut egui::Ui,
    preview: egui::Rect,
    edge: CardinalEdge,
    placeholder: &str,
    value: &mut String,
    known: &HashSet<String>,
    is_dark: bool,
    validation: &EntryValidation,
) {
    const CHIP_W: f32 = 76.0;
    const CHIP_H: f32 = 24.0;
    const PAD: f32 = 6.0;
    let size = egui::vec2(CHIP_W, CHIP_H);
    let center = match edge {
        CardinalEdge::Top => egui::pos2(preview.center().x, preview.top() + PAD + CHIP_H * 0.5),
        CardinalEdge::Bottom => {
            egui::pos2(preview.center().x, preview.bottom() - PAD - CHIP_H * 0.5)
        }
        CardinalEdge::Left => egui::pos2(preview.left() + PAD + CHIP_W * 0.5, preview.center().y),
        CardinalEdge::Right => {
            egui::pos2(preview.right() - PAD - CHIP_W * 0.5, preview.center().y)
        }
    };
    let chip = egui::Rect::from_center_size(center, size);
    ui.painter().rect_filled(
        chip,
        4.0,
        egui::Color32::from_rgba_unmultiplied(16, 16, 16, 170),
    );
    if let Some(stroke) = var_pills::entry_validation_stroke(validation) {
        ui.painter()
            .rect_stroke(chip, 4.0, stroke, egui::StrokeKind::Outside);
    }
    let edit_rect = chip.shrink(3.0);
    let id = ui.id().with(("preview_coord", placeholder));
    let focused = ui.memory(|m| m.has_focus(id));
    let show_overlay =
        !focused && !value.is_empty() && sqyre_varref::contains(value.as_str());
    let resp = if show_overlay {
        let plain_fg = egui::Color32::from_gray(230);
        ui.scope_builder(egui::UiBuilder::new().max_rect(edit_rect), |ui| {
            ui.set_min_size(edit_rect.size());
            ui.centered_and_justified(|ui| {
                var_pills::paint_var_ref_content(ui, value, known, is_dark, plain_fg);
            });
        })
        .response
        .interact(egui::Sense::click())
    } else {
        ui.put(
            edit_rect,
            egui::TextEdit::singleline(value)
                .id(id)
                .frame(egui::Frame::NONE)
                .hint_text(placeholder)
                .desired_width(edit_rect.width()),
        )
    };
    if show_overlay && resp.clicked() {
        ui.memory_mut(|m| m.request_focus(id));
    }
    if let Some(tip) = var_pills::entry_validation_tip(validation) {
        resp.on_hover_text(tip);
    }
}

fn variant_name_from_path(path: &std::path::Path, item: &str) -> String {
    let Some(stem) = path.file_stem().and_then(|s| s.to_str()) else {
        return String::new();
    };
    if stem == item {
        return String::new();
    }
    let prefix = format!("{item}{PROGRAM_DELIMITER}");
    stem.strip_prefix(&prefix)
        .unwrap_or(stem)
        .to_string()
}

fn variant_display_label(name: &str) -> &str {
    if name.is_empty() {
        "(default)"
    } else {
        name
    }
}

fn fit_thumbnail(w: f32, h: f32) -> egui::Vec2 {
    const MAX: f32 = 96.0;
    let w = w.max(1.0);
    let h = h.max(1.0);
    let scale = (MAX / w).min(MAX / h).min(1.0);
    egui::vec2(w * scale, h * scale)
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
            if ui
                .add(egui::Button::new(egui::RichText::new("↻").size(14.0)).small())
                .on_hover_text("Refresh")
                .clicked()
            {
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

/// Collection-tab preview with wheel zoom / drag pan.
fn paint_zoomable_collection_preview(
    ui: &mut egui::Ui,
    icons: &mut IconCache,
    path: &std::path::Path,
    rows: i32,
    cols: i32,
    view: &mut ImageViewTransform,
    replace_clicked: &mut bool,
) {
    ui.add_space(8.0);
    ui.separator();
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new("Collection image").strong());
        if ui
            .add(egui::Button::new(egui::RichText::new("↻").size(14.0)).small())
            .on_hover_text("Refresh")
            .clicked()
        {
            icons.invalidate_path(path);
        }
        if ui.button("Replace Image").clicked() {
            *replace_clicked = true;
        }
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui
                .add_enabled(view.needs_reset_button(), egui::Button::new("Reset view"))
                .on_hover_text("Fit image in viewport")
                .clicked()
            {
                view.reset();
            }
            if view.zoom != 1.0 {
                ui.weak(format!("{:.0}%", view.zoom * 100.0));
            }
        });
    });
    ui.weak("Scroll to zoom; drag to pan when zoomed.");

    let tex = icons.for_path(ui.ctx(), path);
    let avail = ui.available_width().min(520.0);
    let image_size = match &tex {
        Some(t) => {
            let [tw, th] = t.size();
            egui::vec2(tw as f32, th as f32)
        }
        None => egui::vec2(avail, avail * 0.75),
    };
    let fit = fit_panel(image_size.x, image_size.y);
    let scale = (avail / fit.x).min(1.0);
    let desired = egui::vec2((fit.x * scale).max(160.0), (fit.y * scale).max(120.0));
    let (viewport, resp) =
        ui.allocate_exact_size(desired, egui::Sense::click_and_drag());

    image_view::handle_scroll_zoom(ui, viewport, image_size, view, resp.hovered());
    let content =
        image_view::image_content_rect(viewport, image_size, view.zoom, view.pan);

    {
        let painter = ui.painter_at(viewport);
        if let Some(tex) = &tex {
            painter.image(
                tex.id(),
                content,
                egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                egui::Color32::WHITE,
            );
        } else {
            painter.rect_filled(viewport, 0.0, egui::Color32::from_gray(40));
            painter.text(
                viewport.center(),
                egui::Align2::CENTER_CENTER,
                "No image on disk",
                egui::FontId::proportional(14.0),
                egui::Color32::LIGHT_GRAY,
            );
        }
        paint_grid_overlay_painter(&painter, content, rows, cols);
    }
    let _ = image_view::handle_pan_drag(&resp, viewport, image_size, view);

    if path.is_file() {
        ui.weak(path.display().to_string());
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
    const MAX_W: f32 = 340.0;
    const MAX_H: f32 = 240.0;
    let w = w.max(1.0);
    let h = h.max(1.0);
    let scale = (MAX_W / w).min(MAX_H / h).min(1.0);
    egui::vec2(w * scale, h * scale)
}

fn paint_grid_overlay(ui: &mut egui::Ui, rect: egui::Rect, rows: i32, cols: i32) {
    paint_grid_overlay_painter(ui.painter(), rect, rows, cols);
}

fn paint_grid_overlay_painter(painter: &egui::Painter, rect: egui::Rect, rows: i32, cols: i32) {
    let rows = rows.max(1) as f32;
    let cols = cols.max(1) as f32;
    let stroke = egui::Stroke::new(1.0, egui::Color32::from_rgb(255, 80, 80));
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
