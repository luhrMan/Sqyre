//! Floating Data Editor: Programs (Overlay) / Items (Masks) / Coordinates / Tools (Screen Cap, Match Check).

const WINDOW_TITLE: &str = "Data Editor";

/// Points per arrow-key press when the pane divider is keyboard-focused.
const SPLIT_RESIZE_KEY_STEP: f32 = 24.0;

mod dialogs;
pub(crate) mod form_fields;
mod form_state;
mod forms;
pub(crate) mod helpers;
mod jobs;
mod lists;
pub(crate) mod mask_preview;
#[cfg(feature = "native-runtime")]
mod match_check;
pub(crate) mod nav;
mod overlay;
mod persist;
mod preview_state;
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
use form_fields::FormDrafts;
use helpers::{default_editor_program, is_editor_listed_program};
use jobs::BackgroundJobs;
use nav::EditorNav;
use preview_state::PreviewState;
use sqyre_domain::Macro;
use sqyre_hotkeys::ScreenClickBridge;
use sqyre_persist::{
    CatalogView, Database, OverlayButtonConfig, ProgramCatalog, UserSettings,
    MAX_DATA_EDITOR_LEFT_FRAC, MIN_DATA_EDITOR_LEFT_FRAC,
};
pub(crate) use tabs::EditorTab;

/// Mutable session host for Data Editor persist/jobs.
/// Paint and list code take [`CatalogView`] / [`CatalogPaint`] instead.
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

#[derive(Default)]
pub struct DataEditor {
    /// Window open state, tab, search/scope, selection, and list cache.
    pub nav: EditorNav,
    /// Editable form buffers for the currently selected entity.
    drafts: FormDrafts,
    /// Confirm / variant-name / icon-picker / window-picker modals.
    prompts: EditorPrompts,
    status_banner: StatusBanner,
    /// Zoom/pan, refresh, and mask-grid state for the form previews.
    previews: PreviewState,
    /// ScreenCap / collection / running-program jobs and related one-shot flags.
    jobs: BackgroundJobs,
    #[cfg(feature = "native-runtime")]
    match_check: match_check::MatchCheckTool,
}

impl DataEditor {
    /// Take the one-shot flag set when a program rename rewrote name tags.
    pub(crate) fn take_hotkey_refresh(&mut self) -> bool {
        self.jobs.take_hotkey_refresh()
    }

    /// Open the editor, expanding it if it was collapsed to the title bar.
    pub fn request_open(&mut self, ctx: &egui::Context) {
        self.nav.open = true;
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
        if !self.nav.open || !matches!(self.nav.tab, EditorTab::Overlay) {
            return None;
        }
        let id = self.nav.selected_entity.as_ref()?;
        let mut btn = OverlayButtonConfig::new(
            id.clone(),
            self.nav.selected_program.clone().unwrap_or_default(),
        );
        btn.macro_name = self.drafts.overlay.macro_name.clone();
        btn.enabled = self.drafts.overlay.enabled;
        btn.icon = self.drafts.overlay.icon.clone();
        btn.point = self.drafts.overlay.point.clone();
        btn.x = self.drafts.overlay.x;
        btn.y = self.drafts.overlay.y;
        btn.size = self.drafts.overlay.size;
        btn.visibility_gate = self.drafts.overlay.gate.to_config();
        self.drafts.overlay.apply_style_to_config(&mut btn);
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
        self.nav.open && matches!(self.nav.tab, EditorTab::Overlay)
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
            self.nav.selected_program.as_deref()
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
            if self.nav.selected_entity.as_deref() == Some(id.as_str())
                && matches!(self.nav.tab, EditorTab::Overlay)
            {
                self.drafts.overlay.point.clear();
                self.drafts.overlay.x = xf;
                self.drafts.overlay.y = yf;
            }
        }
        if dirty {
            let _ = self.persist_overlay_settings(settings);
        }
    }

    fn clear_entity_selection(&mut self) {
        self.nav.selected_entity = None;
        self.prompts.clear_entity_prompts();
    }

    fn switch_tab(&mut self, tab: EditorTab, catalog: CatalogView<'_>, settings: &UserSettings) {
        if self.nav.tab != tab {
            self.nav.tab = tab;
            self.clear_entity_selection();
            self.nav.scroll_left_list_to_selection = self.nav.selected_program.is_some();
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
        self.switch_tab(tab, CatalogView::from(catalog), settings);
        if !matches!(tab, EditorTab::Programs) && self.nav.selected_program.is_none() {
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
        self.nav.open = true;
        self.switch_tab(tab, CatalogView::from(catalog), settings);
        if !matches!(tab, EditorTab::Programs) && self.nav.selected_program.is_none() {
            if let Some(name) = default_editor_program(CatalogView::from(catalog)) {
                self.select_program(&name, CatalogView::from(catalog), settings);
            }
        }
    }

    /// Prefer the focused application when creating a new model, if known.
    pub(crate) fn set_focused_program_hint(&mut self, program: Option<String>) {
        self.nav.focused_program_hint = program.filter(|p| is_editor_listed_program(p));
    }

    /// Select [`Self::focused_program_hint`] when creating under an entity tab.
    pub(crate) fn prefer_focused_program_for_new(
        &mut self,
        catalog: CatalogView<'_>,
        settings: &UserSettings,
    ) {
        if matches!(self.nav.tab, EditorTab::Programs) {
            return;
        }
        let Some(name) = self.nav.focused_program_hint.clone() else {
            return;
        };
        if catalog.get(&name).is_none() {
            return;
        }
        if self.nav.selected_program.as_deref() == Some(name.as_str()) {
            return;
        }
        self.select_program(&name, catalog, settings);
    }

    /// Open on `tab`, ensure a program is selected, then create a new entity.
    pub(crate) fn open_new(&mut self, tab: EditorTab, env: &mut DataEditorCtx<'_>) {
        self.open_tab(env.ctx, tab, env.catalog, env.settings);
        self.prefer_focused_program_for_new(CatalogView::new(env.catalog), env.settings);
        self.drafts.name.clear();
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
        self.switch_tab(EditorTab::Programs, CatalogView::from(catalog), settings);
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
        self.switch_tab(tab, CatalogView::from(catalog), settings);
        self.select_entity(program, entity, CatalogView::from(catalog), settings);
    }

    pub fn show(
        &mut self,
        env: &mut DataEditorCtx<'_>,
        selected_macro: usize,
        previews: &mut PreviewTooltipCache,
    ) {
        if !self.nav.open {
            return;
        }
        if self
            .nav
            .selected_program
            .as_deref()
            .is_some_and(|n| !is_editor_listed_program(n))
        {
            self.nav.selected_program = None;
            self.clear_entity_selection();
        }
        self.poll_screen_click(env, previews);
        let mut open = self.nav.open;
        let ctx = env.ctx;
        let title = format!("{WINDOW_TITLE} ({})", self.nav.tab.label());
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
                self.ui_tab_bar(ui, env.catalog, env.icons, env.settings);
                self.ui_body(ui, env, selected_macro, previews);
            });
            // Delete matches the footer remove button: confirm, then delete.
            // Consume even with nothing selected so the key does not fall
            // through to the macro tree while this window is in front.
            self.consume_delete_selection(ui);
            if crate::widgets::consume_escape(ui) {
                self.nav.open = false;
            }
        });
        // `.open` reports the titlebar close; `self.nav.open` the Esc path.
        self.nav.open = open && self.nav.open;
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
            program: self.nav.selected_program.as_deref(),
            entity: self.nav.selected_entity.as_deref(),
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
        if self.jobs.screen_cap_new_item {
            self.jobs.screen_cap_new_item = false;
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
                self.drafts.program.process_path = process_path;
                self.drafts.program.window_title = window_title;
            }
            PickerResult::Point(coord) => {
                if let Ok((x, y)) = env
                    .catalog
                    .resolve_point(&coord, &Macro::new("", 0, vec![]))
                {
                    self.drafts.overlay.x = x as f32;
                    self.drafts.overlay.y = y as f32;
                }
                self.drafts.overlay.point = coord.0;
                if matches!(self.nav.tab, EditorTab::Overlay) {
                    if let Some(id) = self.nav.selected_entity.clone() {
                        if let Some(btn) = env
                            .settings
                            .overlay
                            .overlay_buttons
                            .iter_mut()
                            .find(|b| b.id == id)
                        {
                            btn.point = self.drafts.overlay.point.clone();
                            btn.x = self.drafts.overlay.x;
                            btn.y = self.drafts.overlay.y;
                            self.persist_overlay_settings(env.settings);
                        }
                    }
                }
            }
            PickerResult::SearchArea(coord)
                if matches!(
                    self.nav.tab,
                    EditorTab::ScreenCap | EditorTab::MatchCheck | EditorTab::Overlay
                ) =>
            {
                if matches!(self.nav.tab, EditorTab::Overlay) {
                    self.drafts.overlay.gate.search_area = coord.0;
                } else if matches!(self.nav.tab, EditorTab::MatchCheck) {
                    // Keep the selected item; only retarget the preview bounds.
                    if self.apply_search_area_bounds(CatalogView::new(env.catalog), &coord) {
                        #[cfg(feature = "native-runtime")]
                        self.match_check.invalidate();
                    }
                } else {
                    self.apply_screen_cap_reference(CatalogView::new(env.catalog), coord);
                }
            }
            PickerResult::Items {
                targets,
                target_tags: _,
            } if matches!(self.nav.tab, EditorTab::Overlay) => {
                self.drafts.overlay.gate.targets = targets;
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
            self.drafts.monitor = monitor;
            self.drafts.point.x = rx.to_string();
            self.drafts.point.y = ry.to_string();
            previews.invalidate_entity(self.drafts.name.trim());
            self.set_ok(format!("Recorded point monitor {monitor} ({rx}, {ry})."));
            captured = true;
        }
        if let Some((px, py, ax, ay, bx, by)) = env.screen_click.take_search_area() {
            let live = helpers::sync_live_monitor_rects(env.catalog);
            let (monitor, lx, ty, rx, by) =
                sqyre_persist::absolute_area_to_relative(&live, px, py, ax, ay, bx, by);
            self.drafts.monitor = monitor;
            self.drafts.rect.load_displays(
                lx.to_string(),
                ty.to_string(),
                rx.to_string(),
                by.to_string(),
            );
            previews.invalidate_entity(self.drafts.name.trim());
            self.set_ok(format!(
                "Recorded search area monitor {monitor} ({lx},{ty})–({rx},{by})."
            ));
            captured = true;
        }
        if env.screen_click.take_cancelled() {
            self.jobs.save_after_record = false;
            self.set_ok("Recording cancelled.");
        }
        if captured && self.jobs.save_after_record {
            self.jobs.save_after_record = false;
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
        // Re-derive from the saved fraction only when idle. Mid-drag / mid-key
        // resize keeps the absolute left_width so a gesture is not reset from
        // stale settings (persisted-ui: write settings only when settled).
        if !self.nav.split_gesture_active && !self.nav.split_key_moved {
            self.nav.left_width = EditorNav::left_width_from_frac(
                avail_w,
                env.settings.ui.data_editor_left_split,
                MIN_DATA_EDITOR_LEFT_FRAC,
                MAX_DATA_EDITOR_LEFT_FRAC,
            );
        }
        let crate::widgets::SplitView {
            left: mut left_ui,
            right: mut right_ui,
            splitter: split_rect,
        } = crate::widgets::split_view(ui, body_rect, self.nav.left_width);

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
            self.nav.split_gesture_active = true;
            if let Some(pos) = split_resp.interact_pointer_pos() {
                self.nav.left_width = (pos.x - body_rect.left()).clamp(min_left, max_left);
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
                let next = (self.nav.left_width + dx).clamp(min_left, max_left);
                if next != self.nav.left_width {
                    self.nav.left_width = next;
                    self.nav.split_key_moved = true;
                }
            }
            if self.nav.split_key_moved
                && ui.input(|i| {
                    i.key_released(egui::Key::ArrowLeft) || i.key_released(egui::Key::ArrowRight)
                })
            {
                if avail_w > 1.0 {
                    env.settings.ui.data_editor_left_split = EditorNav::frac_from_left_width(
                        avail_w,
                        self.nav.left_width,
                        MIN_DATA_EDITOR_LEFT_FRAC,
                        MAX_DATA_EDITOR_LEFT_FRAC,
                    );
                }
                self.nav.split_key_moved = false;
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
            if avail_w > 1.0 {
                env.settings.ui.data_editor_left_split = EditorNav::frac_from_left_width(
                    avail_w,
                    self.nav.left_width,
                    MIN_DATA_EDITOR_LEFT_FRAC,
                    MAX_DATA_EDITOR_LEFT_FRAC,
                );
            }
            self.nav.split_gesture_active = false;
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
                // Fill widgets (preview panel, TextEdit∞) that claim exact
                // `available_width` can land a sub-pixel past the scroll
                // limit after content_max_rect pixel-rounding — egui then
                // paints a purposeless H-bar on image-preview tabs.
                let form_w = (ui.available_width() - 1.0).max(1.0);
                ui.set_max_width(form_w);
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
        if !matches!(self.nav.tab, EditorTab::ScreenCap | EditorTab::MatchCheck) {
            let right_pane = egui::Rect::from_min_max(
                egui::pos2(split_rect.right(), body_rect.top()),
                body_rect.max,
            );
            crate::widgets::float_bottom_right(ui, right_pane, |ui| {
                let dirty = self.is_dirty(CatalogView::new(env.catalog), env.settings);
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
                    .nav
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
                    .nav
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
        let subject = self.nav.tab.entity_noun()?;
        let name = if self.nav.tab == EditorTab::Programs {
            self.nav.selected_program.as_deref().unwrap_or("")
        } else if self
            .nav
            .selected_program
            .as_deref()
            .is_none_or(str::is_empty)
        {
            ""
        } else {
            self.nav.selected_entity.as_deref().unwrap_or("")
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
        let prog = self.nav.selected_program.clone();
        if prog.is_none() {
            self.set_err("Select a program first.");
        }
        prog
    }

    pub(crate) fn clear_status(&mut self) {
        self.status_banner.clear();
    }
}
