//! Floating Data Editor: Programs / Items / Points / Search Areas / Masks / Collections / AutoPic.

mod form_state;
mod forms;
mod helpers;
mod lists;
mod overlay;
mod persist;
mod variants;

use crate::data_editor_preview::variant_display_label;
use crate::icon_cache::IconCache;
use crate::image_view::ImageViewTransform;
use crate::overlay_icons;
use crate::paint_ctx::CatalogPaint;
use crate::pickers::{self, ActivePicker, PickerResult};
use crate::preview_tooltip::PreviewTooltipCache;
use crate::status_banner::StatusBanner;
use eframe::egui;
use helpers::{overlay_hex_or_empty, rgba_color};
use sqyre_domain::Macro;
use sqyre_hotkeys::ScreenClickBridge;
use sqyre_persist::{
    Database, OverlayButtonConfig, ProgramCatalog, UserSettings, DEFAULT_OVERLAY_BORDER_WIDTH,
    DEFAULT_OVERLAY_BUTTON_SIZE, DEFAULT_OVERLAY_CORNER_RADIUS,
};
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum EditorTab {
    #[default]
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
pub(crate) enum PendingConfirm {
    Delete { label: String },
    Overwrite { kind: &'static str, name: String },
    DeleteVariant { variant: String },
    OverwriteVariant { variant: String, source: PathBuf },
}

#[derive(Debug, Clone)]
pub(crate) enum VariantPrompt {
    /// Ask for a name before adding a non-first variant.
    Name { source: PathBuf },
}

/// Cached left-list / program-selector data; invalidated via [`ProgramCatalog::generation`].
#[derive(Debug, Clone, Default)]
struct ListCache {
    catalog_generation: u64,
    resolution_key: String,
    tab: EditorTab,
    program_names: Vec<String>,
    entities_by_program: HashMap<String, Vec<String>>,
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
    /// Overlay button form: corner radius.
    form_overlay_corner_radius: f32,
    /// Overlay button form: border stroke width.
    form_overlay_border_width: f32,
    /// Overlay button form: border color (includes alpha).
    form_overlay_border: egui::Color32,
    /// Overlay button form: background fill (includes alpha; 0 = none).
    form_overlay_bg: egui::Color32,
    /// Overlay button form: idle icon color (includes alpha).
    form_overlay_icon_color: egui::Color32,
    /// Overlay button form: hover icon color (alpha follows icon color on save).
    form_overlay_icon_hover: egui::Color32,
    /// Bound OS process path for the selected Program.
    form_process_path: String,
    /// Bound window title for the selected Program.
    form_window_title: String,
    variant_name_draft: String,
    variant_prompt: Option<VariantPrompt>,
    status_banner: StatusBanner,
    confirm: Option<PendingConfirm>,
    /// After New Point/Search Area: auto-arm record and persist on capture.
    save_after_record: bool,
    /// Zoom/pan for the collections-tab image preview.
    collection_preview: ImageViewTransform,
    /// `(program, collection)` last shown; reset transform when this changes.
    collection_preview_key: Option<(String, String)>,
    /// Zoom/pan for point / search-area / AutoPic live capture panels.
    coord_preview: ImageViewTransform,
    /// `(tab, program, entity)` last shown; reset transform when this changes.
    coord_preview_key: Option<(EditorTab, String, String)>,
    /// Overlay button id whose icon picker popup is open.
    overlay_icon_picker_for: Option<String>,
    /// Filter text for the overlay icon picker.
    overlay_icon_search: String,
    /// Running-window picker for Program process binding.
    window_picker: ActivePicker,
    /// Background AutoPic capture+save; polled each frame.
    autopix_pending: Option<std::sync::mpsc::Receiver<Result<String, String>>>,
    /// Cached program/entity name lists keyed by catalog generation.
    list_cache: ListCache,
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
            form_overlay_corner_radius: DEFAULT_OVERLAY_CORNER_RADIUS,
            form_overlay_border_width: DEFAULT_OVERLAY_BORDER_WIDTH,
            form_overlay_border: rgba_color([0xdc, 0x9d, 0x2e, 255]),
            form_overlay_bg: rgba_color([0, 0, 0, 0]),
            form_overlay_icon_color: rgba_color([0xf5, 0xe6, 0xc0, 255]),
            form_overlay_icon_hover: rgba_color([0xdc, 0x9d, 0x2e, 255]),
            form_process_path: String::new(),
            form_window_title: String::new(),
            variant_name_draft: String::new(),
            variant_prompt: None,
            status_banner: StatusBanner::default(),
            confirm: None,
            save_after_record: false,
            collection_preview: ImageViewTransform::default(),
            collection_preview_key: None,
            coord_preview: ImageViewTransform::default(),
            coord_preview_key: None,
            overlay_icon_picker_for: None,
            overlay_icon_search: String::new(),
            window_picker: ActivePicker::None,
            autopix_pending: None,
            list_cache: ListCache::default(),
        }
    }
}

impl DataEditor {
    /// Live Overlay-tab form as an on-screen button preview (position, size, icon, label, style).
    ///
    /// Shown while a button is selected for editing, even before Update is clicked.
    pub fn overlay_edit_preview(&self) -> Option<OverlayButtonConfig> {
        if !self.open || !matches!(self.tab, EditorTab::Overlay) {
            return None;
        }
        let id = self.selected_entity.as_ref()?;
        let mut btn = OverlayButtonConfig::new(
            id.clone(),
            self.selected_program.clone().unwrap_or_default(),
        );
        btn.label = self.form_name.clone();
        btn.macro_name = self.form_overlay_macro.clone();
        btn.icon = self.form_overlay_icon.clone();
        btn.x = self.form_overlay_x;
        btn.y = self.form_overlay_y;
        btn.size = self.form_overlay_size;
        self.apply_overlay_style_to_config(&mut btn);
        Some(btn)
    }

    pub(crate) fn apply_overlay_style_to_config(&self, btn: &mut OverlayButtonConfig) {
        btn.corner_radius = self.form_overlay_corner_radius;
        btn.border_width = self.form_overlay_border_width;
        btn.border_color = overlay_hex_or_empty(
            self.form_overlay_border,
            sqyre_persist::DEFAULT_OVERLAY_ACCENT_HEX,
        );
        btn.border_alpha = self.form_overlay_border.a();
        btn.bg_color = if self.form_overlay_bg.a() == 0 {
            String::new()
        } else {
            overlay_hex_or_empty(self.form_overlay_bg, "#000000")
        };
        btn.bg_alpha = self.form_overlay_bg.a();
        btn.icon_color = overlay_hex_or_empty(
            self.form_overlay_icon_color,
            sqyre_persist::DEFAULT_OVERLAY_ICON_HEX,
        );
        btn.icon_alpha = self.form_overlay_icon_color.a();
        btn.icon_hover_color = overlay_hex_or_empty(
            self.form_overlay_icon_hover,
            sqyre_persist::DEFAULT_OVERLAY_ACCENT_HEX,
        );
    }

    pub(crate) fn load_overlay_style_from_config(&mut self, btn: &OverlayButtonConfig) {
        self.form_overlay_corner_radius = btn.corner_radius;
        self.form_overlay_border_width = btn.border_width;
        self.form_overlay_border = rgba_color(btn.border_rgba());
        self.form_overlay_bg = rgba_color(btn.bg_rgba());
        self.form_overlay_icon_color = rgba_color(btn.icon_rgba());
        self.form_overlay_icon_hover = rgba_color(btn.icon_hover_rgba());
    }

    pub(crate) fn reset_overlay_style_form(&mut self) {
        let defaults = OverlayButtonConfig::new("", "");
        self.load_overlay_style_from_config(&defaults);
    }

    #[allow(clippy::too_many_arguments)]
    pub fn show(
        &mut self,
        ctx: &egui::Context,
        db: &mut Database,
        macros: &mut [Macro],
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
        self.draw_confirm(ctx, db, macros, catalog, icons, previews, settings);
        self.draw_overlay_icon_picker(ctx, settings);
        self.poll_window_picker(ctx, catalog, icons, previews, macros);
        self.poll_autopix(ctx);
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
        if let PickerResult::Window {
            process_path,
            window_title,
        } = pickers::show_active_picker(
            ctx,
            &mut self.window_picker,
            &mut CatalogPaint {
                catalog,
                icons,
                previews,
            },
            &macro_opts,
        ) {
            self.form_process_path = process_path;
            self.form_window_title = window_title;
        }
    }

    fn poll_screen_click(
        &mut self,
        screen_click: &ScreenClickBridge,
        previews: &mut PreviewTooltipCache,
        db: &mut Database,
        macros: &mut [Macro],
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
            if !self.status_banner.status_error {
                self.set_ok("Recorded and saved.");
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn ui(
        &mut self,
        ui: &mut egui::Ui,
        db: &mut Database,
        macros: &mut [Macro],
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

        self.status_banner.paint(ui);

        // Claim exactly the remaining window area once (body + footer).
        // Allocating body then drawing footer separately made min_size > window size,
        // so egui's Resize auto-expand ratcheted toward max every frame.
        // Variant name prompts must also claim this area — otherwise the window
        // shrinks to the small dialog content. Delete/overwrite confirms are separate
        // popup windows (see draw_confirm).
        let rem = ui.available_size();
        let (outer, _) = ui.allocate_exact_size(rem, egui::Sense::hover());

        if let Some(VariantPrompt::Name { source }) = self.variant_prompt.clone() {
            ui.scope_builder(egui::UiBuilder::new().max_rect(outer), |ui| {
                self.draw_variant_name_prompt(ui, catalog, icons, settings, source);
            });
            return;
        }
        let footer_h = (ui.spacing().interact_size.y + ui.spacing().item_spacing.y * 3.0 + 8.0)
            .min(rem.y * 0.4);
        let body_h = (rem.y - footer_h).max(40.0);
        let body_rect = egui::Rect::from_min_size(outer.min, egui::vec2(rem.x, body_h));
        let footer_rect =
            egui::Rect::from_min_max(egui::pos2(outer.min.x, outer.min.y + body_h), outer.max);

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
                        self.left_width = (pos.x - body_left - item_gap).clamp(MIN_LEFT, max_left);
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
                                    &mut CatalogPaint {
                                        catalog,
                                        icons,
                                        previews,
                                    },
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
                        .add_enabled(
                            can_new,
                            egui::Button::new(
                                egui::RichText::new("New").color(crate::theme::MACRO_START),
                            ),
                        )
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
                        .add_enabled(
                            can_delete,
                            egui::Button::new(
                                egui::RichText::new("Delete").color(crate::theme::MACRO_STOP),
                            ),
                        )
                        .clicked()
                    {
                        let label = match self.tab {
                            EditorTab::Programs => format!(
                                "program “{}”",
                                self.selected_program.as_deref().unwrap_or("")
                            ),
                            EditorTab::Items => {
                                format!("item “{}”", self.selected_entity.as_deref().unwrap_or(""))
                            }
                            EditorTab::Points => {
                                format!("point “{}”", self.selected_entity.as_deref().unwrap_or(""))
                            }
                            EditorTab::SearchAreas => format!(
                                "search area “{}”",
                                self.selected_entity.as_deref().unwrap_or("")
                            ),
                            EditorTab::Masks => {
                                format!("mask “{}”", self.selected_entity.as_deref().unwrap_or(""))
                            }
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

    #[allow(clippy::too_many_arguments)]
    fn draw_confirm(
        &mut self,
        ctx: &egui::Context,
        db: &mut Database,
        macros: &mut [Macro],
        catalog: &mut ProgramCatalog,
        icons: &mut IconCache,
        previews: &mut PreviewTooltipCache,
        settings: &mut UserSettings,
    ) {
        let Some(confirm) = self.confirm.clone() else {
            return;
        };
        let title = match &confirm {
            PendingConfirm::Delete { .. } | PendingConfirm::DeleteVariant { .. } => {
                "Confirm Delete"
            }
            PendingConfirm::Overwrite { .. } | PendingConfirm::OverwriteVariant { .. } => {
                "Confirm Overwrite"
            }
        };
        let mut open = true;
        egui::Window::new(title)
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .order(egui::Order::Foreground)
            .open(&mut open)
            .show(ctx, |ui| {
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
                match crate::widgets::confirm_cancel_row(ui) {
                    crate::widgets::ConfirmCancel::Cancel => {
                        self.confirm = None;
                    }
                    crate::widgets::ConfirmCancel::Confirm => match confirm {
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
                            self.delete_icon_variant(catalog, icons, settings, &variant);
                        }
                        PendingConfirm::OverwriteVariant { variant, source } => {
                            self.confirm = None;
                            self.overwrite_icon_variant(catalog, icons, &variant, &source);
                        }
                    },
                    crate::widgets::ConfirmCancel::None => {}
                }
            });
        if !open {
            self.confirm = None;
        }
    }

    fn draw_variant_name_prompt(
        &mut self,
        ui: &mut egui::Ui,
        catalog: &ProgramCatalog,
        icons: &mut IconCache,
        settings: &UserSettings,
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
            if ui
                .button(egui::RichText::new("Add").color(crate::theme::MACRO_START))
                .clicked()
            {
                let name = self.variant_name_draft.trim().to_string();
                self.variant_prompt = None;
                self.variant_name_draft.clear();
                self.add_icon_variant(catalog, icons, settings, &name, &source);
            }
        });
    }

    pub(crate) fn set_ok(&mut self, msg: impl Into<String>) {
        self.status_banner.set_ok(msg);
    }

    pub(crate) fn set_err(&mut self, msg: impl Into<String>) {
        self.status_banner.set_err(msg);
    }

    pub(crate) fn clear_status(&mut self) {
        self.status_banner.clear();
    }
}
