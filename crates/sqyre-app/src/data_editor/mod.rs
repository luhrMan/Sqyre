//! Floating Data Editor: Programs (Overlay) / Items (Masks) / Coordinates / Tools (Screen Cap, Match Check).

const WINDOW_TITLE: &str = "Data Editor";

/// Points per arrow-key press when the pane divider is keyboard-focused.
const SPLIT_RESIZE_KEY_STEP: f32 = 24.0;

mod dialogs;
mod form_fields;
mod form_state;
mod forms;
pub(crate) mod helpers;
mod lists;
pub(crate) mod mask_preview;
#[cfg(feature = "native-runtime")]
mod match_check;
mod overlay;
mod persist;
#[cfg(feature = "native-runtime")]
mod running_programs;
mod tabs;
mod toolbar;
mod variants;

use crate::help_text as ht;
use crate::icon_cache::IconCache;
use crate::paint_ctx::CatalogPaint;
use crate::pickers::{self, PickerResult};
use crate::preview_tooltip::PreviewTooltipCache;
use crate::status_banner::StatusBanner;
use dialogs::{ConfirmSubject, EditorPrompts, PendingConfirm};
use eframe::egui;
use form_fields::{
    AtlasFields, ItemFields, KeyedPreview, MaskFields, OverlayFields, PointFields, ProgramFields,
    RectFields,
};
use helpers::{default_editor_program, is_editor_listed_program};
use sqyre_domain::Macro;
use sqyre_hotkeys::ScreenClickBridge;
use sqyre_persist::{
    CatalogView, Database, OverlayButtonConfig, ProgramCatalog, UserSettings,
    MAX_DATA_EDITOR_LEFT_FRAC, MIN_DATA_EDITOR_LEFT_FRAC,
};
use std::collections::HashMap;
use std::path::PathBuf;
pub(crate) use tabs::EditorTab;
use tabs::SearchScope;

/// Shared session borrows for Data Editor (egui ctx, persist, catalog, capture).
pub struct DataEditorCtx<'a> {
    pub ctx: &'a egui::Context,
    pub db: &'a mut Database,
    pub macros: &'a mut [Macro],
    pub catalog: &'a mut ProgramCatalog,
    pub icons: &'a mut IconCache,
    pub screen_click: &'a ScreenClickBridge,
    pub settings: &'a mut UserSettings,
    /// Live hotkey tag selection. A program rename rewrites name tags here too.
    pub hotkey_tag_filters: &'a mut Vec<String>,
    pub pending_scale: Option<&'a crate::widgets::ViewportScaleEvent>,
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
    pub(crate) tab: EditorTab,
    /// Header search query — applies to the left list while `search_open`,
    /// and is preserved (unfiltered) while the field is hidden.
    search: String,
    /// Header search field is expanded (magnifier toggle).
    search_open: bool,
    /// Header program selector is expanded (program-icon toggle). Exclusive
    /// with `search_open` — the two pairs share the same header slot.
    program_open: bool,
    /// Left-list scope for entity tabs (flat selected program vs all programs).
    search_scope: SearchScope,
    /// Draft for the Items / Match Check tag-priority chip editor (ephemeral).
    items_tag_priority_draft: String,
    /// Width of the left list pane (drag-adjustable).
    left_width: f32,
    /// Set when the pane divider was moved via arrow keys; persisted on key release.
    split_key_moved: bool,
    /// Selected program name (all tabs).
    selected_program: Option<String>,
    /// Focused catalog program (from overlay last-foreign / active window).
    /// When set, New entity actions target this program.
    focused_program_hint: Option<String>,
    /// Selected entity within program (items / points / search areas).
    selected_entity: Option<String>,
    /// Expand + scroll the left list to [`Self::selected_program`] (armed on tab switch).
    scroll_left_list_to_selection: bool,
    // Form buffers
    form_name: String,
    /// 1-based monitor slot for Points / Search Areas (relative coords in form_*).
    form_monitor: u32,
    form_point: PointFields,
    form_rect: RectFields,
    form_cols: String,
    form_rows: String,
    form_item: ItemFields,
    form_tags: Vec<String>,
    tag_draft: String,
    form_mask: MaskFields,
    /// Masks-tab items grid filter (session-only).
    mask_items_filter: mask_preview::MaskItemsFilter,
    /// Cached scrim texture for the Masks items preview.
    mask_scrim_cache: Option<mask_preview::MaskScrimCache>,
    form_search_area: String,
    form_atlas: AtlasFields,
    /// Overlay button form draft (placement, style, visibility gate).
    form_overlay: OverlayFields,
    form_program: ProgramFields,
    /// Confirm / variant-name / icon-picker / window-picker modals.
    prompts: EditorPrompts,
    status_banner: StatusBanner,
    /// After New Point/Search Area: auto-arm record and persist on capture.
    save_after_record: bool,
    /// Program rename changed macro or hotkey tags; refresh bindings after paint.
    hotkey_refresh: bool,
    /// Zoom/pan for the collections-tab image preview (keyed by program+collection).
    collection_preview: KeyedPreview<(String, String)>,
    /// Zoom/pan for the atlas plane preview (keyed by program+atlas).
    atlas_preview: KeyedPreview<(String, String)>,
    /// Zoom/pan for point / search-area / ScreenCap panels (keyed by tab+program+entity).
    coord_preview: KeyedPreview<(EditorTab, String, String)>,
    /// One-frame `force` armed by the preview's floating refresh button.
    preview_force: bool,
    /// Background ScreenCap capture+save; polled each frame.
    screen_cap_pending: Option<std::sync::mpsc::Receiver<Result<String, String>>>,
    /// Create a catalog item from the current ScreenCap preview (processed after paint).
    screen_cap_new_item: bool,
    /// Background collection image capture+save; polled each frame.
    collection_capture_pending: Option<CollectionCapturePending>,
    /// Cached program/entity name lists keyed by catalog generation.
    list_cache: ListCache,
    /// MatchCheck match settings (session-local).
    #[cfg(feature = "native-runtime")]
    match_check: match_check::MatchCheckSettings,
    /// Background MatchCheck match job.
    #[cfg(feature = "native-runtime")]
    match_check_pending:
        Option<std::sync::mpsc::Receiver<Result<match_check::MatchCheckResult, String>>>,
    /// Cached heatmap + MatchMap for the current inputs.
    #[cfg(feature = "native-runtime")]
    match_check_cache: Option<match_check::MatchCheckCache>,
    /// Background open-window fetch for Programs "Add running".
    #[cfg(feature = "native-runtime")]
    running_programs_pending: Option<running_programs::RunningProgramsRx>,
}

pub(super) struct CollectionCapturePending {
    pub path: PathBuf,
    /// When set, delete this collection if capture fails (new-collection flow).
    pub rollback_collection: Option<(String, String)>,
    pub rx: std::sync::mpsc::Receiver<Result<(), String>>,
}

impl Default for DataEditor {
    fn default() -> Self {
        Self {
            open: false,
            tab: EditorTab::Programs,
            search: String::new(),
            search_open: false,
            program_open: false,
            search_scope: SearchScope::default(),
            items_tag_priority_draft: String::new(),
            left_width: 0.0,
            split_key_moved: false,
            selected_program: None,
            focused_program_hint: None,
            selected_entity: None,
            scroll_left_list_to_selection: false,
            form_name: String::new(),
            form_monitor: 1,
            form_point: PointFields::default(),
            form_rect: RectFields::default(),
            form_cols: "1".into(),
            form_rows: "1".into(),
            form_item: ItemFields::default(),
            form_tags: Vec::new(),
            tag_draft: String::new(),
            form_mask: MaskFields::default(),
            mask_items_filter: mask_preview::MaskItemsFilter::default(),
            mask_scrim_cache: None,
            form_search_area: String::new(),
            form_atlas: AtlasFields::default(),
            form_overlay: OverlayFields::default(),
            form_program: ProgramFields::default(),
            prompts: EditorPrompts::default(),
            status_banner: StatusBanner::default(),
            save_after_record: false,
            hotkey_refresh: false,
            collection_preview: KeyedPreview::default(),
            atlas_preview: KeyedPreview::default(),
            coord_preview: KeyedPreview::default(),
            preview_force: false,
            screen_cap_pending: None,
            screen_cap_new_item: false,
            collection_capture_pending: None,
            list_cache: ListCache::default(),
            #[cfg(feature = "native-runtime")]
            match_check: match_check::MatchCheckSettings::default(),
            #[cfg(feature = "native-runtime")]
            match_check_pending: None,
            #[cfg(feature = "native-runtime")]
            match_check_cache: None,
            #[cfg(feature = "native-runtime")]
            running_programs_pending: None,
        }
    }
}

impl DataEditor {
    /// Take the one-shot flag set when a program rename rewrote name tags.
    pub(crate) fn take_hotkey_refresh(&mut self) -> bool {
        std::mem::take(&mut self.hotkey_refresh)
    }

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
    /// Shown while a button is selected for editing, even before Save is clicked.
    /// Called from `sync_macro_overlay` when `overlay-buttons` is enabled.
    #[cfg_attr(
        not(feature = "overlay-buttons"),
        allow(dead_code, reason = "used only by live overlay buttons")
    )]
    pub fn overlay_edit_preview(&self) -> Option<OverlayButtonConfig> {
        if !self.open || !matches!(self.tab, EditorTab::Overlay) {
            return None;
        }
        let id = self.selected_entity.as_ref()?;
        let mut btn = OverlayButtonConfig::new(
            id.clone(),
            self.selected_program.clone().unwrap_or_default(),
        );
        btn.macro_name = self.form_overlay.macro_name.clone();
        btn.enabled = self.form_overlay.enabled;
        btn.icon = self.form_overlay.icon.clone();
        btn.point = self.form_overlay.point.clone();
        btn.x = self.form_overlay.x;
        btn.y = self.form_overlay.y;
        btn.size = self.form_overlay.size;
        btn.visibility_gate = self.form_overlay.gate.to_config();
        self.form_overlay.apply_style_to_config(&mut btn);
        if sqyre_persist::is_quick_access_hub_id(id) {
            btn.program = sqyre_domain::GENERAL_PROGRAM.into();
            btn.macro_name = sqyre_persist::QUICK_ACCESS_HUB_ID.into();
            btn.icon = sqyre_persist::QUICK_ACCESS_HUB_ICON.into();
            btn.point.clear();
            btn.border_width = 0.0;
            btn.border_alpha = 0;
            btn.bg_alpha = 0;
            btn.visibility_gate = Default::default();
            btn.corner_radius = (btn.size * 0.5).clamp(
                sqyre_persist::MIN_OVERLAY_CORNER_RADIUS,
                sqyre_persist::MAX_OVERLAY_CORNER_RADIUS,
            );
        }
        Some(btn)
    }

    /// True while the Data Editor Overlay tab is open (drag-to-relocate mode).
    #[cfg_attr(
        not(feature = "overlay-buttons"),
        allow(dead_code, reason = "used only by live overlay buttons")
    )]
    pub fn overlay_relocate_mode(&self) -> bool {
        self.open && matches!(self.tab, EditorTab::Overlay)
    }

    /// Program whose overlay buttons are shown while relocate mode is active.
    ///
    /// `None` when the Overlay tab is closed, or open with no program selected
    /// (then no settings-backed buttons are hosted for drag).
    #[cfg_attr(
        not(feature = "overlay-buttons"),
        allow(dead_code, reason = "used only by live overlay buttons")
    )]
    pub fn overlay_relocate_program(&self) -> Option<&str> {
        if self.overlay_relocate_mode() {
            self.selected_program.as_deref()
        } else {
            None
        }
    }

    /// Apply desktop positions from overlay drag-relocate; clears catalog point refs.
    #[cfg_attr(
        not(feature = "overlay-buttons"),
        allow(dead_code, reason = "used only by live overlay buttons")
    )]
    pub fn apply_overlay_relocations(
        &mut self,
        settings: &mut UserSettings,
        moves: impl IntoIterator<Item = (String, i32, i32)>,
    ) {
        let mut dirty = false;
        for (id, x, y) in moves {
            let xf = x as f32;
            let yf = y as f32;
            if let Some(btn) = settings
                .overlay
                .overlay_buttons
                .iter_mut()
                .find(|b| b.id == id)
            {
                btn.point.clear();
                btn.x = xf;
                btn.y = yf;
                dirty = true;
            }
            if self.selected_entity.as_deref() == Some(id.as_str())
                && matches!(self.tab, EditorTab::Overlay)
            {
                self.form_overlay.point.clear();
                self.form_overlay.x = xf;
                self.form_overlay.y = yf;
            }
        }
        if dirty {
            let _ = self.persist_overlay_settings(settings);
        }
    }

    fn clear_entity_selection(&mut self) {
        self.selected_entity = None;
        self.prompts.clear_entity_prompts();
    }

    fn switch_tab(&mut self, tab: EditorTab, catalog: &ProgramCatalog, settings: &UserSettings) {
        #[cfg(target_arch = "wasm32")]
        let tab = if tab.is_desktop_only() {
            EditorTab::Programs
        } else {
            tab
        };
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
            if let Some(name) = default_editor_program(CatalogView::from(catalog)) {
                self.select_program(&name, CatalogView::from(catalog), settings);
            }
        }
    }

    pub(crate) fn open_tab_for_docs(
        &mut self,
        tab: EditorTab,
        catalog: &ProgramCatalog,
        settings: &UserSettings,
    ) {
        self.open = true;
        self.switch_tab(tab, catalog, settings);
        if !matches!(tab, EditorTab::Programs) && self.selected_program.is_none() {
            if let Some(name) = default_editor_program(CatalogView::from(catalog)) {
                self.select_program(&name, CatalogView::from(catalog), settings);
            }
        }
    }

    /// Prefer the focused application when creating a new model, if known.
    pub(crate) fn set_focused_program_hint(&mut self, program: Option<String>) {
        self.focused_program_hint = program.filter(|p| is_editor_listed_program(p));
    }

    /// Select [`Self::focused_program_hint`] when creating under an entity tab.
    pub(crate) fn prefer_focused_program_for_new(
        &mut self,
        catalog: &ProgramCatalog,
        settings: &UserSettings,
    ) {
        if matches!(self.tab, EditorTab::Programs) {
            return;
        }
        let Some(name) = self.focused_program_hint.clone() else {
            return;
        };
        if catalog.get(&name).is_none() {
            return;
        }
        if self.selected_program.as_deref() == Some(name.as_str()) {
            return;
        }
        self.select_program(&name, CatalogView::from(catalog), settings);
    }

    /// Open on `tab`, ensure a program is selected, then create a new entity.
    pub(crate) fn open_new(&mut self, tab: EditorTab, env: &mut DataEditorCtx<'_>) {
        self.open_tab(env.ctx, tab, env.catalog, env.settings);
        self.prefer_focused_program_for_new(env.catalog, env.settings);
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
        self.select_program(name, CatalogView::from(catalog), settings);
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
        self.select_entity(program, entity, CatalogView::from(catalog), settings);
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
        let title = format!("{WINDOW_TITLE} ({})", self.tab.label());
        // Bounds only — this pane allocates body/footer and owns its ScrollAreas.
        // An outer window scroll (fit_dialog_window) caused intermittent H bars and
        // clipped header buttons when children claimed available_width as min_width.
        crate::widgets::fit_dialog_popup(
            egui::Window::new(title)
                .open(&mut open)
                .default_size([880.0, 560.0])
                .min_size(crate::widgets::FLOATER_MIN_EDITOR)
                // No huge max_size — egui auto-expands toward max when content min_size ratchets.
                .resizable(true),
            ctx,
            egui::Id::new(WINDOW_TITLE),
            env.pending_scale,
        )
        .show(ctx, |ui| {
            crate::widgets::fill_resize_body(ui, |ui| {
                self.ui_tab_bar(ui, env);
                self.ui_body(ui, env, selected_macro, previews);
            });
            // Delete matches the footer remove button: confirm, then delete.
            // Consume even with nothing selected so the key does not fall
            // through to the macro tree while this window is in front.
            self.consume_delete_selection(ui);
            if crate::widgets::consume_escape(ui) {
                self.open = false;
            }
        });
        // `.open` reports the titlebar close; `self.open` the Esc path.
        self.open = open && self.open;
        if let Some((name, source)) = self
            .prompts
            .draw_variant_name_prompt(ctx, env.pending_scale)
        {
            self.add_icon_variant(
                CatalogView::new(env.catalog),
                env.icons,
                env.settings,
                &name,
                &source,
            );
        }
        let subject = ConfirmSubject {
            program: self.selected_program.as_deref(),
            entity: self.selected_entity.as_deref(),
        };
        if let Some(confirmed) = self.prompts.draw_confirm(
            ctx,
            CatalogView::new(env.catalog),
            env.icons,
            subject,
            env.pending_scale,
        ) {
            self.apply_confirm(confirmed, env, previews);
        }
        self.draw_overlay_icon_picker(ctx, env.settings, env.pending_scale);
        self.poll_form_picker(env, previews);
        self.poll_screen_cap(ctx);
        if self.screen_cap_new_item {
            self.screen_cap_new_item = false;
            self.create_item_from_screen_cap(env, previews);
        }
        self.poll_collection_capture(ctx, env.catalog, env.icons);
        #[cfg(feature = "native-runtime")]
        self.poll_match_check(ctx, env.catalog, previews);
        #[cfg(feature = "native-runtime")]
        self.poll_running_programs(env);
    }

    fn poll_form_picker(
        &mut self,
        env: &mut DataEditorCtx<'_>,
        previews: &mut PreviewTooltipCache,
    ) {
        if !self.prompts.window_picker.is_open() {
            return;
        }
        let macro_opts: Vec<(String, Vec<String>)> = env
            .macros
            .iter()
            .map(|m| (m.name.clone(), m.tags.clone()))
            .collect();
        match pickers::show_active_picker(
            env.ctx,
            &mut self.prompts.window_picker,
            &mut CatalogPaint {
                catalog: sqyre_persist::CatalogView::new(env.catalog),
                icons: env.icons,
                previews,
            },
            &macro_opts,
            env.settings.ui.compact_program_headers,
            env.pending_scale,
        ) {
            PickerResult::Window {
                process_path,
                window_title,
            } => {
                self.form_program.process_path = process_path;
                self.form_program.window_title = window_title;
            }
            PickerResult::Point(coord) => {
                if let Ok((x, y)) = env
                    .catalog
                    .resolve_point(&coord, &Macro::new("", 0, vec![]))
                {
                    self.form_overlay.x = x as f32;
                    self.form_overlay.y = y as f32;
                }
                self.form_overlay.point = coord.0;
                if matches!(self.tab, EditorTab::Overlay) {
                    if let Some(id) = self.selected_entity.clone() {
                        if let Some(btn) = env
                            .settings
                            .overlay
                            .overlay_buttons
                            .iter_mut()
                            .find(|b| b.id == id)
                        {
                            btn.point = self.form_overlay.point.clone();
                            btn.x = self.form_overlay.x;
                            btn.y = self.form_overlay.y;
                            self.persist_overlay_settings(env.settings);
                        }
                    }
                }
            }
            PickerResult::SearchArea(coord)
                if matches!(
                    self.tab,
                    EditorTab::ScreenCap | EditorTab::MatchCheck | EditorTab::Overlay
                ) =>
            {
                if matches!(self.tab, EditorTab::Overlay) {
                    self.form_overlay.gate.search_area = coord.0;
                } else if matches!(self.tab, EditorTab::MatchCheck) {
                    // Keep the selected item; only retarget the preview bounds.
                    if self.apply_search_area_bounds(CatalogView::new(env.catalog), &coord) {
                        #[cfg(feature = "native-runtime")]
                        self.invalidate_match_check();
                    }
                } else {
                    self.apply_screen_cap_reference(CatalogView::new(env.catalog), coord);
                }
            }
            PickerResult::Items {
                targets,
                target_tags: _,
            } if matches!(self.tab, EditorTab::Overlay) => {
                self.form_overlay.gate.targets = targets;
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
            self.form_point.x = rx.to_string();
            self.form_point.y = ry.to_string();
            previews.invalidate_entity(self.form_name.trim());
            self.set_ok(format!("Recorded point monitor {monitor} ({rx}, {ry})."));
            captured = true;
        }
        if let Some((px, py, ax, ay, bx, by)) = env.screen_click.take_search_area() {
            let live = helpers::sync_live_monitor_rects(env.catalog);
            let (monitor, lx, ty, rx, by) =
                sqyre_persist::absolute_area_to_relative(&live, px, py, ax, ay, bx, by);
            self.form_monitor = monitor;
            self.form_rect.load_displays(
                lx.to_string(),
                ty.to_string(),
                rx.to_string(),
                by.to_string(),
            );
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

    fn ui_body(
        &mut self,
        ui: &mut egui::Ui,
        env: &mut DataEditorCtx<'_>,
        selected_macro: usize,
        previews: &mut PreviewTooltipCache,
    ) {
        let avail_h = ui.available_height();
        let min_stack = ui.spacing().interact_size.y * 8.0;
        if avail_h + 1.0 < min_stack {
            let w = crate::widgets::visible_width(ui).max(1.0);
            crate::widgets::show_dialog_scroll_mod(
                ui,
                w,
                avail_h.max(1.0),
                |s| s.id_salt("data_editor_short"),
                |ui| {
                    self.ui_body_inner(ui, env, selected_macro, previews, true);
                },
            );
        } else {
            self.ui_body_inner(ui, env, selected_macro, previews, false);
        }
    }

    fn ui_body_inner(
        &mut self,
        ui: &mut egui::Ui,
        env: &mut DataEditorCtx<'_>,
        selected_macro: usize,
        previews: &mut PreviewTooltipCache,
        short: bool,
    ) {
        let header_sep = crate::widgets::section_separator(ui);

        if let Some(msg) = env.screen_click.status_label() {
            ui.colored_label(crate::theme::accent(), msg);
            ui.ctx().request_repaint();
        }

        self.status_banner.paint(ui);

        // Leftover inside the pinned Resize body (not room toward max_size).
        let rem = ui.available_size();
        let min_stack = ui.spacing().interact_size.y * 8.0;
        let used_h = if short { rem.y.max(min_stack) } else { rem.y };
        let (outer, _) = ui.allocate_exact_size(egui::vec2(rem.x, used_h), egui::Sense::hover());

        let body_rect = egui::Rect::from_min_size(outer.min, egui::vec2(rem.x, used_h.max(40.0)));

        let avail_w = body_rect.width();
        let min_left = avail_w * MIN_DATA_EDITOR_LEFT_FRAC;
        let max_left = avail_w * MAX_DATA_EDITOR_LEFT_FRAC;
        let frac = env
            .settings
            .ui
            .data_editor_left_split
            .clamp(MIN_DATA_EDITOR_LEFT_FRAC, MAX_DATA_EDITOR_LEFT_FRAC);
        self.left_width = (avail_w * frac).clamp(min_left, max_left);
        let crate::widgets::SplitView {
            left: mut left_ui,
            right: mut right_ui,
            splitter: split_rect,
        } = crate::widgets::split_view(ui, body_rect, self.left_width);

        let mut tag_submit = false;

        self.draw_left_list(
            &mut left_ui,
            CatalogView::new(env.catalog),
            env.icons,
            previews,
            env.settings,
        );

        let split_resp = ui.interact(
            split_rect,
            ui.id().with("de_split"),
            egui::Sense::click_and_drag(),
        );
        split_resp.widget_info(|| {
            egui::WidgetInfo::labeled(
                egui::WidgetType::ResizeHandle,
                true,
                "Pane divider — drag, or press Left/Right arrows while focused, to resize the list and editor panes",
            )
        });
        let split_resp = split_resp.on_hover_text(ht::ED_RESIZE_PANES);
        if split_resp.is_pointer_button_down_on() || split_resp.clicked() {
            split_resp.request_focus();
        }
        let stroke = if split_resp.hovered() || split_resp.dragged() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeHorizontal);
            ui.visuals().widgets.active.fg_stroke
        } else if split_resp.has_focus() {
            egui::Stroke::new(1.0, crate::theme::accent())
        } else {
            crate::theme::panel_split_stroke()
        };
        // Painted edge-to-edge: the header separator down to the window's
        // bottom frame edge. The drag hit area stays `split_rect`.
        let vline_top = header_sep.rect.center().y;
        let vline_bottom = crate::widgets::frame_outer_rect(ui)
            .map(|r| r.bottom())
            .unwrap_or(split_rect.bottom());
        ui.painter().vline(
            split_rect.center().x,
            vline_top..=vline_bottom.max(vline_top),
            egui::Stroke::new(1.0, stroke.color),
        );
        if split_resp.dragged() {
            if let Some(pos) = split_resp.interact_pointer_pos() {
                self.left_width = (pos.x - body_rect.left()).clamp(min_left, max_left);
                if avail_w > 1.0 {
                    env.settings.ui.data_editor_left_split = (self.left_width / avail_w)
                        .clamp(MIN_DATA_EDITOR_LEFT_FRAC, MAX_DATA_EDITOR_LEFT_FRAC);
                }
            }
        }
        // Keyboard equivalent of the drag: arrows nudge the divider while the
        // splitter holds focus; persist once when the key is released.
        if split_resp.has_focus() {
            ui.memory_mut(|m| {
                m.set_focus_lock_filter(
                    split_resp.id,
                    egui::EventFilter {
                        horizontal_arrows: true,
                        ..Default::default()
                    },
                );
            });
            let dx = if ui.input(|i| i.key_pressed(egui::Key::ArrowLeft)) {
                -SPLIT_RESIZE_KEY_STEP
            } else if ui.input(|i| i.key_pressed(egui::Key::ArrowRight)) {
                SPLIT_RESIZE_KEY_STEP
            } else {
                0.0
            };
            if dx != 0.0 {
                let next = (self.left_width + dx).clamp(min_left, max_left);
                if next != self.left_width {
                    self.left_width = next;
                    if avail_w > 1.0 {
                        env.settings.ui.data_editor_left_split = (self.left_width / avail_w)
                            .clamp(MIN_DATA_EDITOR_LEFT_FRAC, MAX_DATA_EDITOR_LEFT_FRAC);
                    }
                    self.split_key_moved = true;
                }
            }
            if self.split_key_moved
                && ui.input(|i| {
                    i.key_released(egui::Key::ArrowLeft) || i.key_released(egui::Key::ArrowRight)
                })
            {
                self.split_key_moved = false;
                env.settings.clamp();
                if let Err(e) = env.settings.save_default() {
                    self.set_err(format!(
                        "Failed to save data editor layout: {}",
                        crate::status_banner::user_error_detail(&e)
                    ));
                }
            }
        }
        if split_resp.drag_stopped() {
            env.settings.clamp();
            if let Err(e) = env.settings.save_default() {
                self.set_err(format!(
                    "Failed to save data editor layout: {}",
                    crate::status_banner::user_error_detail(&e)
                ));
            }
        }

        {
            let content_w = right_ui.max_rect().width();
            let paint_form = |ui: &mut egui::Ui| {
                // Cap soft-wrap to the painted clip (the reserved scrollbar
                // lane insets it past `max_rect`); dialog_scroll (both axes)
                // still H-scrolls when a hard min-width exceeds the viewport.
                crate::widgets::clamp_row_right_to_clip(ui);
                let macros: &[Macro] = env.macros;
                tag_submit = self.draw_form(
                    ui,
                    &mut CatalogPaint {
                        catalog: sqyre_persist::CatalogView::new(env.catalog),
                        icons: env.icons,
                        previews,
                    },
                    env.screen_click,
                    macros,
                    macros.get(selected_macro),
                    env.settings,
                );
            };
            // The footer chip floats over the pane's bottom-right corner; the
            // form scrolls beneath it — no strip is reserved (see baseline).
            crate::widgets::show_dialog_scroll_mod(
                &mut right_ui,
                content_w,
                body_rect.height().max(40.0),
                |s| s.id_salt("data_editor_form"),
                paint_form,
            );
        }

        // Footer chrome floats over the form's bottom-right instead of
        // reserving a full-width strip. Tools tabs (ScreenCap / MatchCheck)
        // draw their own action row inside the form — no floating chip.
        if !matches!(self.tab, EditorTab::ScreenCap | EditorTab::MatchCheck) {
            let right_pane = egui::Rect::from_min_max(
                egui::pos2(split_rect.right(), body_rect.top()),
                body_rect.max,
            );
            crate::widgets::float_bottom_right(ui, right_pane, |ui| {
                let dirty = self.is_dirty(env.catalog, env.settings);
                let valid = self.form_valid(env.macros.get(selected_macro));
                let save_enabled = dirty && valid;
                let save_clicked =
                    crate::widgets::dirty_action_button(ui, "Save", save_enabled).clicked();
                // Enter submits when Save is able: this window is in front,
                // and no confirm / picker / combo is using the key.
                let save_enter = save_enabled
                    && self.enter_commits_update(ui)
                    && ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Enter));
                if save_clicked || save_enter || (tag_submit && save_enabled) {
                    self.on_update(env, previews);
                }
                let new_tip = self
                    .tab
                    .entity_noun()
                    .map(|noun| format!("New {noun}"))
                    .unwrap_or_else(|| "New".to_owned());
                if crate::widgets::add_button(ui, &new_tip, true).clicked() {
                    self.on_new(env);
                }
                // Leftmost: remove the current selection via the same confirm
                // the list's right-click Delete uses.
                let delete_tip = self
                    .tab
                    .entity_noun()
                    .map(|noun| format!("Delete {noun}"))
                    .unwrap_or_else(|| "Delete".to_owned());
                if crate::widgets::remove_icon_button(
                    ui,
                    &delete_tip,
                    self.selected_delete_label().is_some(),
                )
                .clicked()
                {
                    self.request_delete_selected();
                }
            });
        }
    }

    /// `{subject} “{name}”` for the delete confirm when the current selection
    /// is deletable on this tab; `None` on Tools tabs or with nothing selected.
    fn selected_delete_label(&self) -> Option<String> {
        let subject = self.tab.entity_noun()?;
        let name = if self.tab == EditorTab::Programs {
            self.selected_program.as_deref().unwrap_or("")
        } else if self.selected_program.as_deref().is_none_or(str::is_empty) {
            ""
        } else {
            self.selected_entity.as_deref().unwrap_or("")
        };
        if name.is_empty() {
            return None;
        }
        Some(format!("{subject} “{name}”"))
    }

    /// Ask to delete the selected program / entity (list row right-click → Delete).
    pub(crate) fn request_delete_selected(&mut self) {
        if let Some(label) = self.selected_delete_label() {
            self.prompts.confirm = Some(PendingConfirm::Delete { label });
        }
    }

    /// Delete key activates the same remove as the footer button, when this
    /// window is in front and a text field is not using the key.
    fn consume_delete_selection(&mut self, ui: &mut egui::Ui) {
        if !self.owns_delete_key(ui) {
            return;
        }
        let pressed = ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Delete));
        if pressed {
            self.request_delete_selected();
        }
    }

    fn owns_delete_key(&self, ui: &egui::Ui) -> bool {
        !self.prompts.any_open()
            && !ui.ctx().any_popup_open()
            && !ui.ctx().text_edit_focused()
            && crate::focus_nav::is_top_layer(ui.ctx(), ui.layer_id())
    }

    /// Enter → Update only when this window is in front and no overlay owns the key.
    fn enter_commits_update(&self, ui: &egui::Ui) -> bool {
        !self.prompts.any_open()
            && !ui.ctx().any_popup_open()
            && !crate::focus_nav::focused_in_layer(ui.ctx(), ui.layer_id())
            && crate::focus_nav::is_top_layer(ui.ctx(), ui.layer_id())
    }

    pub(crate) fn set_ok(&mut self, msg: impl Into<String>) {
        self.status_banner.set_ok(msg);
    }

    pub(crate) fn set_err(&mut self, msg: impl Into<String>) {
        self.status_banner.set_err(msg);
    }

    /// Selected program name; sets the UI error and returns `None` when unset.
    pub(crate) fn selected_program_or_err(&mut self) -> Option<String> {
        let prog = self.selected_program.clone();
        if prog.is_none() {
            self.set_err("Select a program first.");
        }
        prog
    }

    pub(crate) fn clear_status(&mut self) {
        self.status_banner.clear();
    }

    #[cfg(feature = "native-runtime")]
    fn invalidate_match_check(&mut self) {
        self.match_check_cache = None;
        self.match_check_pending = None;
        self.match_check.show_many_match_boxes = false;
    }

    #[cfg(feature = "native-runtime")]
    pub(crate) fn stop_match_check_compute(&mut self) {
        self.match_check_pending = None;
        self.match_check_cache = None;
        self.match_check.last_inputs.clear();
        self.match_check.paused = true;
        self.match_check.capture_once = false;
        self.match_check.awaiting_panel = false;
        self.match_check.rematch_once = false;
    }
}
