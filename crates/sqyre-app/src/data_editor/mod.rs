//! Floating Data Editor: Programs (Overlay) / Items (Masks) / Coordinates / Tools (ScreenCap, PixelCheck).

const WINDOW_TITLE: &str = "Data Editor";

mod form_state;
mod forms;
pub(crate) mod helpers;
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
use helpers::{editor_program_names, is_editor_listed_program, overlay_hex_or_empty, rgba_color};
use sqyre_domain::Macro;
use sqyre_hotkeys::ScreenClickBridge;
use sqyre_persist::{
    default_overlay_position, Database, OverlayButtonConfig, OverlayVisibilityGate,
    OverlayVisibilityMode, ProgramCatalog, UserSettings, DEFAULT_OVERLAY_BORDER_WIDTH,
    DEFAULT_OVERLAY_BUTTON_SIZE, DEFAULT_OVERLAY_CORNER_RADIUS, DEFAULT_OVERLAY_FALLBACK_SCREEN_H,
    DEFAULT_OVERLAY_FALLBACK_SCREEN_W, DEFAULT_OVERLAY_GATE_BLUR, DEFAULT_OVERLAY_GATE_INTERVAL_MS,
    DEFAULT_OVERLAY_GATE_TOLERANCE, MAX_DATA_EDITOR_LEFT_FRAC, MIN_DATA_EDITOR_LEFT_FRAC,
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
    pub pending_scale: Option<&'a crate::widgets::ViewportScaleEvent>,
}

/// Top-level Data Editor section (tab bar).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum EditorSection {
    Programs,
    Items,
    Coordinates,
    Tools,
}

impl EditorSection {
    fn of(tab: EditorTab) -> Self {
        match tab {
            EditorTab::Programs | EditorTab::Overlay => Self::Programs,
            EditorTab::Items | EditorTab::Masks => Self::Items,
            EditorTab::Points
            | EditorTab::SearchAreas
            | EditorTab::Collections
            | EditorTab::Atlases => Self::Coordinates,
            EditorTab::ScreenCap | EditorTab::PixelCheck => Self::Tools,
        }
    }

    fn default_tab(self) -> EditorTab {
        match self {
            Self::Programs => EditorTab::Programs,
            Self::Items => EditorTab::Items,
            Self::Coordinates => EditorTab::Points,
            Self::Tools => EditorTab::ScreenCap,
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
    /// Sort for Items / Pixel Check left-list icon grids (UI-only).
    items_list_sort: sqyre_domain::CatalogItemSort,
    /// Tag priority when [`Self::items_list_sort`] is Tags.
    items_tag_priority: Vec<String>,
    items_tag_priority_draft: String,
    /// Width of the left list pane (drag-adjustable).
    left_width: f32,
    /// Selected program name (all tabs).
    selected_program: Option<String>,
    /// Selected entity within program (items / points / search areas).
    selected_entity: Option<String>,
    /// Expand + scroll the left list to [`Self::selected_program`] (armed on tab switch).
    scroll_left_list_to_selection: bool,
    // Form buffers
    form_name: String,
    /// 1-based monitor slot for Points / Search Areas (relative coords in form_*).
    form_monitor: u32,
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
    /// Overlay visibility gate: show button only when image search finds a match.
    form_overlay_gate_enabled: bool,
    form_overlay_gate_targets: Vec<String>,
    form_overlay_gate_search_area: String,
    form_overlay_gate_tolerance: f64,
    form_overlay_gate_blur: i32,
    form_overlay_gate_match_method: sqyre_domain::MatchMethod,
    form_overlay_gate_interval_ms: u64,
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
    /// Create a catalog item from the current ScreenCap preview (processed after paint).
    screen_cap_new_item: bool,
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
            items_list_sort: sqyre_domain::CatalogItemSort::default(),
            items_tag_priority: Vec::new(),
            items_tag_priority_draft: String::new(),
            left_width: 0.0,
            selected_program: None,
            selected_entity: None,
            scroll_left_list_to_selection: false,
            form_name: String::new(),
            form_monitor: 1,
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
            form_overlay_gate_enabled: false,
            form_overlay_gate_targets: Vec::new(),
            form_overlay_gate_search_area: String::new(),
            form_overlay_gate_tolerance: DEFAULT_OVERLAY_GATE_TOLERANCE,
            form_overlay_gate_blur: DEFAULT_OVERLAY_GATE_BLUR,
            form_overlay_gate_match_method: sqyre_domain::MatchMethod::CcoeffNormed,
            form_overlay_gate_interval_ms: DEFAULT_OVERLAY_GATE_INTERVAL_MS,
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
            screen_cap_new_item: false,
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
        btn.visibility_gate = self.overlay_gate_from_form();
        self.apply_overlay_style_to_config(&mut btn);
        Some(btn)
    }

    pub(crate) fn overlay_gate_from_form(&self) -> OverlayVisibilityGate {
        let mut gate = OverlayVisibilityGate {
            mode: if self.form_overlay_gate_enabled {
                OverlayVisibilityMode::ShowWhenFound
            } else {
                OverlayVisibilityMode::Off
            },
            targets: self.form_overlay_gate_targets.clone(),
            search_area: self.form_overlay_gate_search_area.trim().to_string(),
            tolerance: self.form_overlay_gate_tolerance,
            blur: self.form_overlay_gate_blur,
            match_method: self.form_overlay_gate_match_method,
            interval_ms: self.form_overlay_gate_interval_ms,
        };
        gate.clamp();
        gate
    }

    pub(crate) fn load_overlay_gate_from_config(&mut self, gate: &OverlayVisibilityGate) {
        self.form_overlay_gate_enabled = gate.is_active();
        self.form_overlay_gate_targets = gate.targets.clone();
        self.form_overlay_gate_search_area = gate.search_area.clone();
        self.form_overlay_gate_tolerance = gate.tolerance;
        self.form_overlay_gate_blur = gate.blur;
        self.form_overlay_gate_match_method = gate.match_method;
        self.form_overlay_gate_interval_ms = gate.interval_ms;
    }

    pub(crate) fn reset_overlay_gate_form(&mut self) {
        self.load_overlay_gate_from_config(&OverlayVisibilityGate::default());
    }

    /// True while the Data Editor Overlay tab is open (drag-to-relocate mode).
    #[cfg_attr(not(feature = "overlay-buttons"), allow(dead_code))]
    pub fn overlay_relocate_mode(&self) -> bool {
        self.open && matches!(self.tab, EditorTab::Overlay)
    }

    /// Program whose overlay buttons are shown while relocate mode is active.
    ///
    /// `None` when the Overlay tab is closed, or open with no program selected
    /// (then no settings-backed buttons are hosted for drag).
    #[cfg_attr(not(feature = "overlay-buttons"), allow(dead_code))]
    pub fn overlay_relocate_program(&self) -> Option<&str> {
        if self.overlay_relocate_mode() {
            self.selected_program.as_deref()
        } else {
            None
        }
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
            self.scroll_left_list_to_selection = self.selected_program.is_some();
            if tab == EditorTab::ScreenCap {
                self.reset_item_param_fields();
            }
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
            if let Some(name) = editor_program_names(catalog).next() {
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
        if self
            .selected_program
            .as_deref()
            .is_some_and(|n| !is_editor_listed_program(n))
        {
            self.selected_program = None;
            self.clear_entity_selection();
        }
        self.poll_screen_click(env, previews);
        let mut open = self.open;
        let ctx = env.ctx;
        // Bounds only — this pane allocates body/footer and owns its ScrollAreas.
        // An outer window scroll (fit_dialog_window) caused intermittent H bars and
        // clipped header buttons when children claimed available_width as min_width.
        crate::widgets::fit_dialog_popup(
            egui::Window::new(WINDOW_TITLE)
                .open(&mut open)
                .default_size([880.0, 560.0])
                .min_size([520.0, 280.0])
                // No huge max_size — egui auto-expands toward max when content min_size ratchets.
                .resizable(true),
            ctx,
            egui::Id::new(WINDOW_TITLE),
            env.pending_scale,
        )
        .show(ctx, |ui| {
            self.ui(ui, env, selected_macro, previews);
        });
        self.open = open;
        self.draw_variant_name_prompt(ctx, env.catalog, env.icons, env.settings, env.pending_scale);
        self.draw_confirm(env, previews);
        self.draw_overlay_icon_picker(ctx, env.settings, env.pending_scale);
        self.poll_form_picker(env, previews);
        self.poll_screen_cap(ctx);
        if self.screen_cap_new_item {
            self.screen_cap_new_item = false;
            self.create_item_from_screen_cap(env, previews);
        }
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
            env.pending_scale,
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
                if matches!(
                    self.tab,
                    EditorTab::ScreenCap | EditorTab::PixelCheck | EditorTab::Overlay
                ) =>
            {
                if matches!(self.tab, EditorTab::Overlay) {
                    self.form_overlay_gate_search_area = coord.0;
                } else {
                    self.apply_screen_cap_reference(env.catalog, coord);
                }
            }
            PickerResult::Items(targets) if matches!(self.tab, EditorTab::Overlay) => {
                self.form_overlay_gate_targets = targets;
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
            let live = helpers::sync_live_monitor_rects(env.catalog);
            let (monitor, rx, ry) = sqyre_persist::absolute_point_to_relative(&live, x, y);
            self.form_monitor = monitor;
            self.form_x = rx.to_string();
            self.form_y = ry.to_string();
            previews.invalidate_entity(self.form_name.trim());
            self.set_ok(format!("Recorded point monitor {monitor} ({rx}, {ry})."));
            captured = true;
        }
        if let Some((px, py, ax, ay, bx, by)) = env.screen_click.take_search_area() {
            let live = helpers::sync_live_monitor_rects(env.catalog);
            let (monitor, lx, ty, rx, by) =
                sqyre_persist::absolute_area_to_relative(&live, px, py, ax, ay, bx, by);
            self.form_monitor = monitor;
            self.form_left = lx.to_string();
            self.form_top = ty.to_string();
            self.form_right = rx.to_string();
            self.form_bottom = by.to_string();
            previews.invalidate_entity(self.form_name.trim());
            self.set_ok(format!(
                "Recorded search area monitor {monitor} ({lx},{ty})–({rx},{by})."
            ));
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
}

impl DataEditor {
    fn ui(
        &mut self,
        ui: &mut egui::Ui,
        env: &mut DataEditorCtx<'_>,
        selected_macro: usize,
        previews: &mut PreviewTooltipCache,
    ) {
        // Claim exactly the painted window size, then draw in a child that does
        // *not* advance the parent by content min_rect (scope_builder would —
        // that's what pushed the right edge off-screen when the left pane was wide).
        let size = crate::widgets::visible_size(ui).max(egui::vec2(1.0, 1.0));
        let (rect, _) = ui.allocate_exact_size(size, egui::Sense::hover());
        let mut body = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(rect)
                .layout(egui::Layout::top_down(egui::Align::Min)),
        );
        body.set_clip_rect(rect.intersect(ui.clip_rect()));
        body.set_max_size(rect.size());
        self.ui_body(&mut body, env, selected_macro, previews);
    }

    fn ui_body(
        &mut self,
        ui: &mut egui::Ui,
        env: &mut DataEditorCtx<'_>,
        selected_macro: usize,
        previews: &mut PreviewTooltipCache,
    ) {
        ui.horizontal_wrapped(|ui| {
            let section = EditorSection::of(self.tab);
            for (sec, label) in [
                (EditorSection::Programs, "Programs"),
                (EditorSection::Items, "Items"),
                (EditorSection::Coordinates, "Coordinates"),
                (EditorSection::Tools, "Tools"),
            ] {
                if ui.selectable_label(section == sec, label).clicked() && section != sec {
                    self.switch_tab(sec.default_tab(), env.catalog, env.settings);
                }
            }
        });
        let section = EditorSection::of(self.tab);
        if matches!(
            section,
            EditorSection::Programs
                | EditorSection::Items
                | EditorSection::Coordinates
                | EditorSection::Tools
        ) {
            ui.horizontal_wrapped(|ui| {
                let prev = self.tab;
                match section {
                    EditorSection::Programs => {
                        ui.selectable_value(&mut self.tab, EditorTab::Programs, "Programs");
                        ui.selectable_value(&mut self.tab, EditorTab::Overlay, "Overlay");
                    }
                    EditorSection::Items => {
                        ui.selectable_value(&mut self.tab, EditorTab::Items, "Items");
                        ui.selectable_value(&mut self.tab, EditorTab::Masks, "Masks");
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
                    EditorSection::Tools => {
                        ui.selectable_value(&mut self.tab, EditorTab::ScreenCap, "ScreenCap");
                        ui.selectable_value(&mut self.tab, EditorTab::PixelCheck, "PixelCheck");
                    }
                }
                if self.tab != prev {
                    self.clear_entity_selection();
                    self.scroll_left_list_to_selection = self.selected_program.is_some();
                    if self.tab == EditorTab::ScreenCap {
                        self.reset_item_param_fields();
                    }
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

        // Leftover inside the pinned Resize body (not room toward max_size).
        let rem = ui.available_size();
        let (outer, _) = ui.allocate_exact_size(rem, egui::Sense::hover());

        let footer_h = (ui.spacing().interact_size.y + ui.spacing().item_spacing.y * 3.0 + 8.0)
            .min(rem.y * 0.4);
        let body_h = (rem.y - footer_h).max(40.0);
        let body_rect = egui::Rect::from_min_size(outer.min, egui::vec2(rem.x, body_h));
        let footer_rect =
            egui::Rect::from_min_max(egui::pos2(outer.min.x, outer.min.y + body_h), outer.max);

        const SPLITTER_W: f32 = 6.0;
        let avail_w = body_rect.width();
        let min_left = avail_w * MIN_DATA_EDITOR_LEFT_FRAC;
        let max_left = avail_w * MAX_DATA_EDITOR_LEFT_FRAC;
        let frac = env
            .settings
            .data_editor_left_split
            .clamp(MIN_DATA_EDITOR_LEFT_FRAC, MAX_DATA_EDITOR_LEFT_FRAC);
        self.left_width = (avail_w * frac).clamp(min_left, max_left);
        // Keep splitter + right inside body_rect — never allocate past the frame.
        let left_w = self.left_width.min((avail_w - SPLITTER_W).max(0.0));
        let left_rect = egui::Rect::from_min_size(body_rect.min, egui::vec2(left_w, body_h));
        let split_rect = egui::Rect::from_min_size(
            egui::pos2(left_rect.right(), body_rect.top()),
            egui::vec2(SPLITTER_W, body_h),
        );
        let right_rect = egui::Rect::from_min_max(
            egui::pos2(split_rect.right(), body_rect.top()),
            body_rect.max,
        );

        let mut tag_submit = false;

        // `new_child` (not scope_builder): do not advance parent by form min_size.
        {
            let mut left_ui = ui.new_child(
                egui::UiBuilder::new()
                    .max_rect(left_rect)
                    .layout(egui::Layout::top_down(egui::Align::Min)),
            );
            left_ui.set_clip_rect(left_rect.intersect(ui.clip_rect()));
            left_ui.set_max_size(left_rect.size());
            self.draw_left_list(&mut left_ui, env.catalog, env.icons, previews, env.settings);
        }

        let split_resp =
            ui.interact(split_rect, ui.id().with("de_split"), egui::Sense::click_and_drag());
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
                self.left_width = (pos.x - body_rect.left()).clamp(min_left, max_left);
                if avail_w > 1.0 {
                    env.settings.data_editor_left_split = (self.left_width / avail_w)
                        .clamp(MIN_DATA_EDITOR_LEFT_FRAC, MAX_DATA_EDITOR_LEFT_FRAC);
                }
            }
        }
        if split_resp.drag_stopped() {
            env.settings.clamp();
            if let Err(e) = env.settings.save_default() {
                self.set_err(format!("Failed to save data editor layout: {e}"));
            }
        }

        {
            let mut right_ui = ui.new_child(
                egui::UiBuilder::new()
                    .max_rect(right_rect)
                    .layout(egui::Layout::top_down(egui::Align::Min)),
            );
            right_ui.set_clip_rect(right_rect.intersect(ui.clip_rect()));
            right_ui.set_max_size(right_rect.size());
            let fill_tab = matches!(self.tab, EditorTab::ScreenCap | EditorTab::PixelCheck);
            let mut paint_form = |ui: &mut egui::Ui| {
                // Cap content width so a vertical-only ScrollArea cannot report a
                // wider content_size (that expands the pane / window).
                ui.set_max_width(right_rect.width());
                let macros: &[Macro] = env.macros;
                tag_submit = self.draw_form(
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
            if fill_tab {
                paint_form(&mut right_ui);
            } else {
                // Enable horizontal scroll so width stays at the viewport
                // (`auto_shrink` false + vertical-only expands to content width).
                pickers::dialog_scroll(right_rect.width(), body_h)
                    .id_salt("data_editor_form")
                    .show(&mut right_ui, paint_form);
            }
        }

        {
            let mut footer_ui = ui.new_child(
                egui::UiBuilder::new()
                    .max_rect(footer_rect)
                    .layout(egui::Layout::top_down(egui::Align::Min)),
            );
            footer_ui.set_clip_rect(footer_rect.intersect(ui.clip_rect()));
            footer_ui.set_max_size(footer_rect.size());
            footer_ui.vertical(|ui| {
                ui.separator();
                ui.horizontal_wrapped(|ui| {
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
                    let update_enabled = can_update && dirty && valid;
                    let update_clicked =
                        crate::theme::dirty_action_button(ui, "Update", update_enabled).clicked();
                    // Enter submits when Update is able: this window is in front,
                    // and no confirm / picker / combo is using the key.
                    let update_enter = update_enabled
                        && self.enter_commits_update(ui)
                        && ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Enter));
                    if update_clicked || update_enter || (tag_submit && update_enabled) {
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
        }
    }

    /// Enter → Update only when this window is in front and no overlay owns the key.
    fn enter_commits_update(&self, ui: &egui::Ui) -> bool {
        self.confirm.is_none()
            && self.variant_prompt.is_none()
            && !self.window_picker.is_open()
            && self.overlay_icon_picker_for.is_none()
            && !ui.ctx().any_popup_open()
            && ui.ctx().top_layer_id() == Some(ui.layer_id())
            && ui
                .ctx()
                .memory(|m| m.areas().top_layer_id(egui::Order::Foreground).is_none())
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
        let open = crate::widgets::confirm_window(ctx, title, env.pending_scale, |ui| {
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
        pending_scale: Option<&crate::widgets::ViewportScaleEvent>,
    ) {
        let Some(VariantPrompt::Name { source }) = self.variant_prompt.clone() else {
            return;
        };
        let mut submit = false;
        let mut cancel = false;
        let open = crate::widgets::confirm_window(ctx, "Add Icon Variant", pending_scale, |ui| {
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
        use helpers::form_desktop_area;
        let (lx, ty, rx, by) = form_desktop_area(
            catalog,
            self.form_monitor,
            &self.form_left,
            &self.form_top,
            &self.form_right,
            &self.form_bottom,
        );
        let coords_ok = match (
            self.selected_program.as_deref(),
            self.selected_entity.as_deref(),
        ) {
            (Some(prog), Some(item)) => pixel_check::can_compute_pixel_check(
                catalog,
                prog,
                item,
                &self.pixel_check.variant,
                lx,
                ty,
                rx,
                by,
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
