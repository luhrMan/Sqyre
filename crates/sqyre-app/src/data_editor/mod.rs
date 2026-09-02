//! Floating Data Editor: Programs / Items (Masks, ScreenCap, PixelCheck) / Coordinates / Overlay.

const WINDOW_TITLE: &str = "Data Editor";

mod form_state;
mod forms;
mod helpers;
mod lists;
mod overlay;
mod persist;
#[cfg(feature = "native-runtime")]
mod pixel_check;
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
    default_overlay_position, Database, OverlayButtonConfig, ProgramCatalog, UserSettings,
    DEFAULT_OVERLAY_BORDER_WIDTH, DEFAULT_OVERLAY_BUTTON_SIZE, DEFAULT_OVERLAY_CORNER_RADIUS,
    DEFAULT_OVERLAY_FALLBACK_SCREEN_H, DEFAULT_OVERLAY_FALLBACK_SCREEN_W,
};
use std::collections::HashMap;
use std::path::PathBuf;

/// Shared session borrows for Data Editor (egui ctx, persist, catalog, capture).
pub struct DataEditorCtx<'a> {
    pub ctx: &'a egui::Context,
    pub db: &'a mut Database,
    pub macros: &'a mut [Macro],
    pub catalog: &'a mut ProgramCatalog,
    pub icons: &'a mut IconCache,
    pub screen_click: &'a ScreenClickBridge,
    pub settings: &'a mut UserSettings,
}

/// Top-level Data Editor section (tab bar).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum EditorSection {
    Programs,
    Items,
    Coordinates,
    Overlay,
}

impl EditorSection {
    fn of(tab: EditorTab) -> Self {
        match tab {
            EditorTab::Programs => Self::Programs,
            EditorTab::Items | EditorTab::Masks | EditorTab::ScreenCap | EditorTab::PixelCheck => {
                Self::Items
            }
            EditorTab::Points
            | EditorTab::SearchAreas
            | EditorTab::Collections
            | EditorTab::Atlases => Self::Coordinates,
            EditorTab::Overlay => Self::Overlay,
        }
    }

    fn default_tab(self) -> EditorTab {
        match self {
            Self::Programs => EditorTab::Programs,
            Self::Items => EditorTab::Items,
            Self::Coordinates => EditorTab::Points,
            Self::Overlay => EditorTab::Overlay,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum EditorTab {
    #[default]
    Programs,
    Items,
    Masks,
    ScreenCap,
    PixelCheck,
    Points,
    SearchAreas,
    Collections,
    Atlases,
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
    /// Atlas member Collection names (ordered).
    form_atlas_members: Vec<String>,
    /// Draft Collection name to add to the Atlas member list.
    form_atlas_add: String,
    /// Overlay button form: target macro name.
    form_overlay_macro: String,
    /// Overlay button form: whether the button is drawn on screen.
    form_overlay_enabled: bool,
    /// Overlay button form: built-in icon id.
    form_overlay_icon: String,
    /// Overlay button form: optional catalog point (`program~name`) for location.
    form_overlay_point: String,
    /// Overlay button form: desktop X position (fallback when no point).
    form_overlay_x: f32,
    /// Overlay button form: desktop Y position (fallback when no point).
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
    /// `(program, atlas)` last shown for plane preview; reset transform when this changes.
    atlas_preview_key: Option<(String, String)>,
    /// Zoom/pan for the atlas plane preview.
    atlas_preview: ImageViewTransform,
    /// Zoom/pan for point / search-area / ScreenCap live capture panels.
    coord_preview: ImageViewTransform,
    /// `(tab, program, entity)` last shown; reset transform when this changes.
    coord_preview_key: Option<(EditorTab, String, String)>,
    /// Overlay button id whose icon picker popup is open.
    overlay_icon_picker_for: Option<String>,
    /// Filter text for the overlay icon picker.
    overlay_icon_search: String,
    /// Running-window picker for Program process binding.
    window_picker: ActivePicker,
    /// Background ScreenCap capture+save; polled each frame.
    screen_cap_pending: Option<std::sync::mpsc::Receiver<Result<String, String>>>,
    /// Background collection image capture+save; polled each frame.
    collection_capture_pending: Option<CollectionCapturePending>,
    /// Cached program/entity name lists keyed by catalog generation.
    list_cache: ListCache,
    /// PixelCheck match settings (session-local).
    #[cfg(feature = "native-runtime")]
    pixel_check: pixel_check::PixelCheckSettings,
    /// Background PixelCheck match job.
    #[cfg(feature = "native-runtime")]
    pixel_check_pending:
        Option<std::sync::mpsc::Receiver<Result<pixel_check::PixelCheckResult, String>>>,
    /// Cached heatmap + MatchMap for the current inputs.
    #[cfg(feature = "native-runtime")]
    pixel_check_cache: Option<pixel_check::PixelCheckCache>,
}

pub(super) struct CollectionCapturePending {
    pub path: PathBuf,
    /// When set, delete this collection if capture fails (new-collection flow).
    pub rollback_collection: Option<(String, String)>,
    pub rx: std::sync::mpsc::Receiver<Result<(), String>>,
}

impl Default for DataEditor {
    fn default() -> Self {
        let (overlay_x, overlay_y) = default_overlay_position(
            0.0,
            0.0,
            DEFAULT_OVERLAY_FALLBACK_SCREEN_W,
            DEFAULT_OVERLAY_FALLBACK_SCREEN_H,
            DEFAULT_OVERLAY_BUTTON_SIZE,
            0,
        );
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
            form_atlas_members: Vec::new(),
            form_atlas_add: String::new(),
            form_overlay_macro: String::new(),
            form_overlay_enabled: true,
            form_overlay_icon: overlay_icons::DEFAULT_ICON_ID.into(),
            form_overlay_point: String::new(),
            form_overlay_x: overlay_x,
            form_overlay_y: overlay_y,
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
            atlas_preview: ImageViewTransform::default(),
            atlas_preview_key: None,
            coord_preview: ImageViewTransform::default(),
            coord_preview_key: None,
            overlay_icon_picker_for: None,
            overlay_icon_search: String::new(),
            window_picker: ActivePicker::None,
            screen_cap_pending: None,
            collection_capture_pending: None,
            list_cache: ListCache::default(),
            #[cfg(feature = "native-runtime")]
            pixel_check: pixel_check::PixelCheckSettings::default(),
            #[cfg(feature = "native-runtime")]
            pixel_check_pending: None,
            #[cfg(feature = "native-runtime")]
            pixel_check_cache: None,
        }
    }
}

impl DataEditor {
    /// Open the editor, expanding it if it was collapsed to the title bar.
    pub fn request_open(&mut self, ctx: &egui::Context) {
        self.open = true;
        let area_id = egui::Id::new(WINDOW_TITLE);
        let mut collapsing = egui::collapsing_header::CollapsingState::load_with_default_open(
            ctx,
            area_id.with("collapsing"),
            true,
        );
        collapsing.set_open(true);
        collapsing.store(ctx);
        ctx.move_to_top(egui::LayerId::new(egui::Order::Middle, area_id));
    }

    /// Live Overlay-tab form as an on-screen button preview (position, size, icon, style).
    ///
    /// Shown while a button is selected for editing, even before Update is clicked.
    /// Called from `sync_macro_overlay` when `overlay-buttons` is enabled.
    #[cfg_attr(not(feature = "overlay-buttons"), allow(dead_code))]
    pub fn overlay_edit_preview(&self) -> Option<OverlayButtonConfig> {
        if !self.open || !matches!(self.tab, EditorTab::Overlay) {
            return None;
        }
        let id = self.selected_entity.as_ref()?;
        let mut btn = OverlayButtonConfig::new(
            id.clone(),
            self.selected_program.clone().unwrap_or_default(),
        );
        btn.macro_name = self.form_overlay_macro.clone();
        btn.enabled = self.form_overlay_enabled;
        btn.icon = self.form_overlay_icon.clone();
        btn.point = self.form_overlay_point.clone();
        btn.x = self.form_overlay_x;
        btn.y = self.form_overlay_y;
        btn.size = self.form_overlay_size;
        self.apply_overlay_style_to_config(&mut btn);
        Some(btn)
    }

    /// True while the Data Editor Overlay tab is open (drag-to-relocate mode).
    #[cfg_attr(not(feature = "overlay-buttons"), allow(dead_code))]
    pub fn overlay_relocate_mode(&self) -> bool {
        self.open && matches!(self.tab, EditorTab::Overlay)
    }

    /// Apply desktop positions from overlay drag-relocate; clears catalog point refs.
    #[cfg_attr(not(feature = "overlay-buttons"), allow(dead_code))]
    pub fn apply_overlay_relocations(
        &mut self,
        settings: &mut UserSettings,
        moves: impl IntoIterator<Item = (String, i32, i32)>,
    ) {
        let mut dirty = false;
        for (id, x, y) in moves {
            let xf = x as f32;
            let yf = y as f32;
            if let Some(btn) = settings.overlay_buttons.iter_mut().find(|b| b.id == id) {
                btn.point.clear();
                btn.x = xf;
                btn.y = yf;
                dirty = true;
            }
            if self.selected_entity.as_deref() == Some(id.as_str())
                && matches!(self.tab, EditorTab::Overlay)
            {
                self.form_overlay_point.clear();
                self.form_overlay_x = xf;
                self.form_overlay_y = yf;
            }
        }
        if dirty {
            let _ = self.persist_overlay_settings(settings);
        }
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

    fn clear_entity_selection(&mut self) {
        self.selected_entity = None;
        self.variant_prompt = None;
        self.overlay_icon_picker_for = None;
    }

    fn switch_tab(&mut self, tab: EditorTab, catalog: &ProgramCatalog, settings: &UserSettings) {
        if self.tab != tab {
            self.tab = tab;
            self.clear_entity_selection();
        }
        self.load_form(catalog, settings);
    }

    /// Open the editor on `tab`, selecting a program when the tab needs one.
    pub(crate) fn open_tab(
        &mut self,
        ctx: &egui::Context,
        tab: EditorTab,
        catalog: &ProgramCatalog,
        settings: &UserSettings,
    ) {
        self.request_open(ctx);
        self.switch_tab(tab, catalog, settings);
        if !matches!(tab, EditorTab::Programs) && self.selected_program.is_none() {
            if let Some(name) = catalog.program_names().next() {
                self.select_program(name, catalog, settings);
            }
        }
    }

    /// Open on `tab`, ensure a program is selected, then create a new entity.
    pub(crate) fn open_new(&mut self, tab: EditorTab, env: &mut DataEditorCtx<'_>) {
        self.open_tab(env.ctx, tab, env.catalog, env.settings);
        self.form_name.clear();
        self.on_new(env);
    }

    pub(crate) fn open_program(
        &mut self,
        ctx: &egui::Context,
        name: &str,
        catalog: &ProgramCatalog,
        settings: &UserSettings,
    ) {
        self.request_open(ctx);
        self.switch_tab(EditorTab::Programs, catalog, settings);
        self.select_program(name, catalog, settings);
    }

    pub(crate) fn open_entity(
        &mut self,
        ctx: &egui::Context,
        tab: EditorTab,
        program: &str,
        entity: &str,
        catalog: &ProgramCatalog,
        settings: &UserSettings,
    ) {
        self.request_open(ctx);
        self.switch_tab(tab, catalog, settings);
        self.select_entity(program, entity, catalog, settings);
    }

    pub fn show(
        &mut self,
        env: &mut DataEditorCtx<'_>,
        selected_macro: usize,
        previews: &mut PreviewTooltipCache,
    ) {
        if !self.open {
            return;
        }
        self.poll_screen_click(env, previews);
        let mut open = self.open;
        let ctx = env.ctx;
        crate::widgets::fit_dialog_window(
            egui::Window::new(WINDOW_TITLE)
                .open(&mut open)
                .default_size([880.0, 560.0])
                .min_size([520.0, 280.0])
                // No huge max_size — egui auto-expands toward max when content min_size ratchets.
                .resizable(true),
            ctx,
        )
        .show(ctx, |ui| {
                self.ui(ui, env, selected_macro, previews);
            });
        self.open = open;
        self.draw_variant_name_prompt(ctx, env.catalog, env.icons, env.settings);
        self.draw_confirm(env, previews);
        self.draw_overlay_icon_picker(ctx, env.settings);
        self.poll_form_picker(env, previews);
        self.poll_screen_cap(ctx);
        self.poll_collection_capture(ctx, env.catalog, env.icons);
        #[cfg(feature = "native-runtime")]
        self.poll_pixel_check(ctx, env.catalog, previews);
    }

    fn poll_form_picker(
        &mut self,
        env: &mut DataEditorCtx<'_>,
        previews: &mut PreviewTooltipCache,
    ) {
        if !self.window_picker.is_open() {
            return;
        }
        let macro_opts: Vec<(String, Vec<String>)> = env
            .macros
            .iter()
            .map(|m| (m.name.clone(), m.tags.clone()))
            .collect();
        match pickers::show_active_picker(
            env.ctx,
            &mut self.window_picker,
            &mut CatalogPaint {
                catalog: env.catalog,
                icons: env.icons,
                previews,
            },
            &macro_opts,
            env.settings.compact_program_headers,
        ) {
            PickerResult::Window {
                process_path,
                window_title,
            } => {
                self.form_process_path = process_path;
                self.form_window_title = window_title;
            }
            PickerResult::Point(coord) => {
                if let Ok((x, y)) = env
                    .catalog
                    .resolve_point(&coord, &Macro::new("", 0, vec![]))
                {
                    self.form_overlay_x = x as f32;
                    self.form_overlay_y = y as f32;
                }
                self.form_overlay_point = coord.0;
                if matches!(self.tab, EditorTab::Overlay) {
                    if let Some(id) = self.selected_entity.clone() {
                        if let Some(btn) =
                            env.settings.overlay_buttons.iter_mut().find(|b| b.id == id)
                        {
                            btn.point = self.form_overlay_point.clone();
                            btn.x = self.form_overlay_x;
                            btn.y = self.form_overlay_y;
                            self.persist_overlay_settings(env.settings);
                        }
                    }
                }
            }
            PickerResult::SearchArea(coord)
                if matches!(self.tab, EditorTab::ScreenCap | EditorTab::PixelCheck) =>
            {
                self.apply_screen_cap_reference(env.catalog, coord);
            }
            _ => {}
        }
    }

    fn poll_screen_click(
        &mut self,
        env: &mut DataEditorCtx<'_>,
        previews: &mut PreviewTooltipCache,
    ) {
        let mut captured = false;
        if let Some((x, y)) = env.screen_click.take_point() {
            self.form_x = x.to_string();
            self.form_y = y.to_string();
            previews.invalidate_entity(self.form_name.trim());
            self.set_ok(format!("Recorded point ({x}, {y})."));
            captured = true;
        }
        if let Some((lx, ty, rx, by)) = env.screen_click.take_search_area() {
            self.form_left = lx.to_string();
            self.form_top = ty.to_string();
            self.form_right = rx.to_string();
            self.form_bottom = by.to_string();
            previews.invalidate_entity(self.form_name.trim());
            self.set_ok(format!("Recorded search area ({lx},{ty})–({rx},{by})."));
            captured = true;
        }
        if env.screen_click.take_cancelled() {
            self.save_after_record = false;
            self.set_ok("Recording cancelled.");
        }
        if captured && self.save_after_record {
            self.save_after_record = false;
            self.on_update(env, previews);
            if !self.status_banner.status_error {
                self.set_ok("Recorded and saved.");
            }
        }
    }

    fn ui(
        &mut self,
        ui: &mut egui::Ui,
        env: &mut DataEditorCtx<'_>,
        selected_macro: usize,
        previews: &mut PreviewTooltipCache,
    ) {
        ui.horizontal(|ui| {
            let section = EditorSection::of(self.tab);
            for (sec, label) in [
                (EditorSection::Programs, "Programs"),
                (EditorSection::Items, "Items"),
                (EditorSection::Coordinates, "Coordinates"),
                (EditorSection::Overlay, "Overlay"),
            ] {
                if ui.selectable_label(section == sec, label).clicked() && section != sec {
                    self.switch_tab(sec.default_tab(), env.catalog, env.settings);
                }
            }
        });
        let section = EditorSection::of(self.tab);
        if matches!(section, EditorSection::Items | EditorSection::Coordinates) {
            ui.horizontal(|ui| {
                let prev = self.tab;
                match section {
                    EditorSection::Items => {
                        ui.selectable_value(&mut self.tab, EditorTab::Items, "Items");
                        ui.selectable_value(&mut self.tab, EditorTab::Masks, "Masks");
                        ui.selectable_value(&mut self.tab, EditorTab::ScreenCap, "ScreenCap");
                        ui.selectable_value(&mut self.tab, EditorTab::PixelCheck, "PixelCheck");
                    }
                    EditorSection::Coordinates => {
                        ui.label(egui::RichText::new("Basic").weak().small());
                        ui.selectable_value(&mut self.tab, EditorTab::Points, "Points");
                        ui.selectable_value(&mut self.tab, EditorTab::SearchAreas, "Search Areas");
                        ui.add_space(12.0);
                        ui.label(egui::RichText::new("Advanced").weak().small());
                        ui.selectable_value(&mut self.tab, EditorTab::Collections, "Collections");
                        ui.selectable_value(&mut self.tab, EditorTab::Atlases, "Atlases");
                    }
                    _ => {}
                }
                if self.tab != prev {
                    self.clear_entity_selection();
                    self.load_form(env.catalog, env.settings);
                }
            });
        }
        ui.separator();

        if let Some(msg) = env.screen_click.status_label() {
            ui.colored_label(crate::theme::PRIMARY, msg);
            ui.ctx().request_repaint();
        }

        self.status_banner.paint(ui);

        // Claim exactly the remaining window area once (body + footer).
        // Allocating body then drawing footer separately made min_size > window size,
        // so egui's Resize auto-expand ratcheted toward max every frame.
        let rem = ui.available_size();
        let (outer, _) = ui.allocate_exact_size(rem, egui::Sense::hover());

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
                        self.draw_left_list(ui, env.catalog, env.icons, previews, env.settings);
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
                        let fill_tab =
                            matches!(self.tab, EditorTab::ScreenCap | EditorTab::PixelCheck);
                        let mut paint_form = |ui: &mut egui::Ui| {
                            ui.set_max_width(ui.available_width());
                            let macros: &[Macro] = env.macros;
                            self.draw_form(
                                ui,
                                &mut CatalogPaint {
                                    catalog: env.catalog,
                                    icons: env.icons,
                                    previews,
                                },
                                env.screen_click,
                                macros,
                                macros.get(selected_macro),
                                env.settings,
                            );
                        };
                        // ScreenCap fills remaining height; a ScrollArea would always overflow.
                        if fill_tab {
                            paint_form(ui);
                        } else {
                            pickers::scroll_vertical()
                                .id_salt("data_editor_form")
                                .auto_shrink([false, false])
                                .max_height(body_h)
                                .show(ui, paint_form);
                        }
                    },
                );
            });
        });

        ui.scope_builder(egui::UiBuilder::new().max_rect(footer_rect), |ui| {
            ui.set_clip_rect(footer_rect);
            ui.vertical(|ui| {
                ui.separator();
                ui.horizontal(|ui| {
                    let can_new = !matches!(self.tab, EditorTab::ScreenCap | EditorTab::PixelCheck);
                    if ui
                        .add_enabled(
                            can_new,
                            egui::Button::new(
                                egui::RichText::new("New").color(crate::theme::MACRO_START),
                            ),
                        )
                        .clicked()
                    {
                        self.on_new(env);
                    }
                    let dirty = self.is_dirty(env.catalog, env.settings);
                    let valid = self.form_valid(env.macros.get(selected_macro));
                    let can_update =
                        !matches!(self.tab, EditorTab::ScreenCap | EditorTab::PixelCheck);
                    if crate::theme::dirty_action_button(ui, "Update", can_update && dirty && valid)
                        .clicked()
                    {
                        self.on_update(env, previews);
                    }
                    let can_delete = match self.tab {
                        EditorTab::Programs => self.selected_program.is_some(),
                        EditorTab::ScreenCap | EditorTab::PixelCheck => false,
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
                            EditorTab::Atlases => {
                                format!("atlas “{}”", self.selected_entity.as_deref().unwrap_or(""))
                            }
                            EditorTab::Overlay => format!(
                                "overlay button “{}”",
                                self.selected_entity.as_deref().unwrap_or("")
                            ),
                            EditorTab::ScreenCap | EditorTab::PixelCheck => String::new(),
                        };
                        if !label.is_empty() {
                            self.confirm = Some(PendingConfirm::Delete { label });
                        }
                    }
                });
            });
        });
    }

    fn draw_confirm(&mut self, env: &mut DataEditorCtx<'_>, previews: &mut PreviewTooltipCache) {
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
        let ctx = env.ctx;
        let open = crate::widgets::confirm_window(ctx, title, |ui| {
            match &confirm {
                PendingConfirm::Delete { label } => {
                    ui.horizontal(|ui| {
                        if let Some(prog) = self.selected_program.as_deref() {
                            crate::icon_cache::paint_program_icon(ui, env.catalog, env.icons, prog);
                        }
                        ui.label(format!("Delete {label}? This cannot be undone."));
                    });
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
                        self.on_delete(env, previews);
                    }
                    PendingConfirm::Overwrite { .. } => {
                        self.confirm = None;
                        self.apply_update(env, true, previews);
                    }
                    PendingConfirm::DeleteVariant { variant } => {
                        self.confirm = None;
                        self.delete_icon_variant(env.catalog, env.icons, env.settings, &variant);
                    }
                    PendingConfirm::OverwriteVariant { variant, source } => {
                        self.confirm = None;
                        self.overwrite_icon_variant(env.catalog, env.icons, &variant, &source);
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
        ctx: &egui::Context,
        catalog: &ProgramCatalog,
        icons: &mut IconCache,
        settings: &UserSettings,
    ) {
        let Some(VariantPrompt::Name { source }) = self.variant_prompt.clone() else {
            return;
        };
        let mut submit = false;
        let mut cancel = false;
        let open = crate::widgets::confirm_window(ctx, "Add Icon Variant", |ui| {
            ui.label("Variant name");
            ui.add(egui::TextEdit::singleline(&mut self.variant_name_draft).desired_width(220.0));
            ui.horizontal(|ui| {
                if ui.button("Cancel").clicked() {
                    cancel = true;
                }
                if ui
                    .button(egui::RichText::new("Add").color(crate::theme::MACRO_START))
                    .clicked()
                {
                    submit = true;
                }
            });
            match crate::widgets::poll_confirm_keys(ui) {
                crate::widgets::ConfirmCancel::Cancel => cancel = true,
                crate::widgets::ConfirmCancel::Confirm => submit = true,
                crate::widgets::ConfirmCancel::None => {}
            }
        });
        if !open || cancel {
            self.variant_prompt = None;
            self.variant_name_draft.clear();
            return;
        }
        if submit {
            let name = self.variant_name_draft.trim().to_string();
            self.variant_prompt = None;
            self.variant_name_draft.clear();
            self.add_icon_variant(catalog, icons, settings, &name, &source);
        }
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

    #[cfg(feature = "native-runtime")]
    fn invalidate_pixel_check(&mut self) {
        self.pixel_check_cache = None;
        self.pixel_check_pending = None;
        self.pixel_check.show_many_match_boxes = false;
    }

    #[cfg(feature = "native-runtime")]
    pub(crate) fn stop_pixel_check_compute(&mut self) {
        self.pixel_check_pending = None;
        self.pixel_check_cache = None;
        self.pixel_check.last_inputs.clear();
        self.pixel_check.paused = true;
    }

    #[cfg(feature = "native-runtime")]
    fn poll_pixel_check(
        &mut self,
        ctx: &egui::Context,
        catalog: &ProgramCatalog,
        previews: &mut PreviewTooltipCache,
    ) {
        if !matches!(self.tab, EditorTab::PixelCheck) {
            if self.pixel_check_pending.is_some() {
                self.stop_pixel_check_compute();
            }
            return;
        }
        use helpers::form_coord_literal;
        let coords_ok = match (
            self.selected_program.as_deref(),
            self.selected_entity.as_deref(),
        ) {
            (Some(prog), Some(item)) => pixel_check::can_compute_pixel_check(
                catalog,
                prog,
                item,
                &self.pixel_check.variant,
                form_coord_literal(&self.form_left),
                form_coord_literal(&self.form_top),
                form_coord_literal(&self.form_right),
                form_coord_literal(&self.form_bottom),
            ),
            _ => false,
        };
        if !coords_ok && self.pixel_check_pending.is_some() {
            self.stop_pixel_check_compute();
        }
        if let Some(rx) = self.pixel_check_pending.as_ref() {
            match rx.try_recv() {
                Ok(Ok(result)) => {
                    self.pixel_check_pending = None;
                    if result.fingerprint == self.pixel_check.last_inputs {
                        if result.tolerance_matches.len() > pixel_check::MANY_MATCH_BOX_THRESHOLD {
                            self.pixel_check.show_many_match_boxes = false;
                        }
                        self.pixel_check_cache = Some(pixel_check::finish_cache(ctx, result));
                    }
                }
                Ok(Err(e)) => {
                    self.pixel_check_pending = None;
                    self.pixel_check.paused = true;
                    self.set_err(format!("PixelCheck: {e}"));
                }
                Err(std::sync::mpsc::TryRecvError::Empty) => {
                    ctx.request_repaint();
                }
                Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                    self.pixel_check_pending = None;
                    self.pixel_check.paused = true;
                    self.set_err("PixelCheck: match failed");
                }
            }
        }
        let _ = previews;
        let _ = catalog;
    }
}
