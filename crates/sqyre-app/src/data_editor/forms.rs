//! Form buffers: load, draw, dirty, valid.
//!
//! [`DataEditor::draw_form`] dispatches on the active tab; each tab's body
//! lives in a submodule here.

mod collections;
mod coords;
mod overlay_form;
mod programs;

use super::form_state;
use super::helpers::{
    collect_program_item_tags, form_absolute_xy, form_coord_literal, form_desktop_area, parse_i32,
};
use super::{DataEditor, EditorTab};
use crate::action_tooltip::help;
use crate::data_editor_preview::{
    catalog_search_area_pixel_size, paint_preview_coord_chip, paint_preview_floaters,
    paint_search_area_preview_sizes, paint_zoomable_atlas_preview,
    paint_zoomable_collection_preview, show_file_hover, CardinalEdge, RecordFloater,
};
#[cfg(feature = "native-runtime")]
use crate::help_text as ht;
use crate::overlay_icons;
use crate::paint_ctx::CatalogPaint;
use crate::paint_ctx::VarTheme;
use crate::pickers;
use crate::preview_tooltip::PreviewKind;
use crate::widgets::{match_settings, SearchableComboOpts};
use eframe::egui;
use sqyre_domain::{
    collect_known_variable_names, CoordinateRef, KnownVariableNames, Macro, PROGRAM_DELIMITER,
};
use sqyre_hotkeys::ScreenClickBridge;
use sqyre_persist::{
    screen_cap_path, CatalogView, OverlayButtonConfig, ProgramCollection, UserSettings,
    DEFAULT_OVERLAY_BUTTON_SIZE, MAX_OVERLAY_BORDER_WIDTH, MAX_OVERLAY_BUTTON_SIZE,
    MAX_OVERLAY_CORNER_RADIUS, MAX_OVERLAY_GATE_INTERVAL_MS, MIN_OVERLAY_BORDER_WIDTH,
    MIN_OVERLAY_BUTTON_SIZE, MIN_OVERLAY_CORNER_RADIUS, MIN_OVERLAY_GATE_INTERVAL_MS,
};
use sqyre_validate::{validate_entity_name, validate_numeric_expression};

/// Read-only inputs every data-editor form shares.
///
/// Bundled so each `draw_*_form` takes a handful of arguments instead of
/// re-threading the same five through every tab.
#[derive(Clone, Copy)]
pub(super) struct FormCtx<'a> {
    pub screen_click: &'a ScreenClickBridge,
    pub macros: &'a [Macro],
    pub active_macro: Option<&'a Macro>,
    pub known: &'a KnownVariableNames,
    /// `ui.visuals().dark_mode`, sampled once per frame.
    pub is_dark: bool,
}

pub(super) fn paint_fs_name_hint(ui: &mut egui::Ui, name: &str) {
    let name = name.trim();
    if name.is_empty() {
        return;
    }
    if let Err(e) = validate_entity_name(name) {
        crate::widgets::error_icon(ui, &e.to_string());
    }
}

/// Name label + content-sized edit + `DE_NAME` tip — the body of every
/// catalog-entity Name row. Standalone rows go through [`paint_name_field`].
pub(super) fn name_field_body(ui: &mut egui::Ui, name: &mut String) {
    crate::widgets::pair_labeled(ui, egui_phosphor::regular::TEXT_T, "Name", |ui| {
        let name_w = crate::widgets::name_edit_width(ui, name, "");
        help::tip(
            ui.add(egui::TextEdit::singleline(name).desired_width(name_w)),
            help::DE_NAME,
        );
    });
}

/// [`name_field_body`] as a standalone unit plus the name-validation hint line.
fn paint_name_field(ui: &mut egui::Ui, name: &mut String) {
    name_field_body(ui, name);
    paint_fs_name_hint(ui, name);
}

fn color_alpha_drag(ui: &mut egui::Ui, label: &str, color: &mut egui::Color32) {
    ui.label(label);
    ui.color_edit_button_srgba(color);
    let mut alpha = color.a();
    if help::tip(
        ui.add(
            crate::widgets::num_entry(&mut alpha)
                .range(0..=255)
                .prefix("α "),
        ),
        help::DE_OVERLAY_ALPHA_NONE,
    )
    .changed()
    {
        *color = egui::Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), alpha);
    }
}

/// Overlay validated coord chips on a preview rect.
fn paint_coord_chips(
    ui: &mut egui::Ui,
    rect: egui::Rect,
    known: &KnownVariableNames,
    is_dark: bool,
    active_macro: Option<&Macro>,
    chips: &mut [(&mut String, CardinalEdge, &str, &str)],
) {
    for (value, edge, placeholder, help_text) in chips {
        let validation = validate_numeric_expression(value, active_macro);
        paint_preview_coord_chip(
            ui,
            rect,
            *edge,
            placeholder,
            value,
            known,
            is_dark,
            &validation,
            help_text,
        );
    }
}

impl DataEditor {
    fn sync_coord_preview_view(&mut self) {
        let key = (
            self.nav.tab,
            self.nav.selected_program.clone().unwrap_or_default(),
            self.nav.selected_entity.clone().unwrap_or_default(),
        );
        self.previews.coord.sync_key(key);
    }

    /// Preview chrome for a coord panel: refresh / fit / zoom / record on one
    /// chip at `rect`'s top-left. Refresh re-captures on the next pass; record
    /// arms `screen_click` (cancellation is keyboard-only — the app is not the
    /// click target while armed, and `status_label` teaches Esc).
    fn paint_preview_controls(
        &mut self,
        ui: &mut egui::Ui,
        rect: egui::Rect,
        screen_click: &ScreenClickBridge,
        record_tip: &'static str,
        recording_msg: &'static str,
        arm: impl FnOnce(&ScreenClickBridge),
    ) {
        let clicks = paint_preview_floaters(
            ui,
            rect,
            true,
            Some(&mut self.previews.coord.transform),
            "Fit image in viewport",
            Some(RecordFloater {
                tip: record_tip,
                enabled: !screen_click.is_armed(),
            }),
        );
        if clicks.refresh {
            self.previews.request_force();
        }
        if clicks.record {
            self.jobs.save_after_record = false;
            arm(screen_click);
            self.set_ok(recording_msg);
        }
    }

    /// Two-corner record controls on a search-area preview.
    fn paint_area_record(
        &mut self,
        ui: &mut egui::Ui,
        rect: egui::Rect,
        screen_click: &ScreenClickBridge,
        record_tip: &'static str,
        recording_msg: &'static str,
    ) {
        self.paint_preview_controls(
            ui,
            rect,
            screen_click,
            record_tip,
            recording_msg,
            ScreenClickBridge::arm_search_area,
        );
    }

    /// Size badge and edge chips under a search-area preview.
    #[expect(
        clippy::too_many_arguments,
        reason = "preview rect + edges + var theme; shared by three area forms"
    )]
    fn paint_area_chips(
        &mut self,
        ui: &mut egui::Ui,
        rect: egui::Rect,
        preview_image_size: egui::Vec2,
        edges: (Option<i32>, Option<i32>, Option<i32>, Option<i32>),
        known: &KnownVariableNames,
        is_dark: bool,
        active_macro: Option<&Macro>,
    ) {
        let (lx, ty, rx, by) = edges;
        paint_search_area_preview_sizes(ui, rect, lx, ty, rx, by, preview_image_size);
        paint_coord_chips(
            ui,
            rect,
            known,
            is_dark,
            active_macro,
            &mut self.area_edge_fields(),
        );
    }

    /// Point record control and the X/Y chips on the preview.
    fn paint_point_tail(
        &mut self,
        ui: &mut egui::Ui,
        rect: egui::Rect,
        screen_click: &ScreenClickBridge,
        known: &KnownVariableNames,
        is_dark: bool,
        active_macro: Option<&Macro>,
    ) {
        self.paint_preview_controls(
            ui,
            rect,
            screen_click,
            crate::help_text::ED_REC_POINT,
            "Recording… left-click to capture.",
            ScreenClickBridge::arm_point,
        );
        paint_coord_chips(
            ui,
            rect,
            known,
            is_dark,
            active_macro,
            &mut [
                (
                    &mut self.drafts.point.x,
                    CardinalEdge::Left,
                    "X",
                    help::DE_POINT_X,
                ),
                (
                    &mut self.drafts.point.y,
                    CardinalEdge::Bottom,
                    "Y",
                    help::DE_POINT_Y,
                ),
            ],
        );
    }

    /// Record controls, size badge, and edge chips. Pixel check paints its heatmap
    /// between [`Self::paint_area_record`] and [`Self::paint_area_chips`] instead.
    #[expect(
        clippy::too_many_arguments,
        reason = "record tips + edge chips + var theme; shared by search-area forms"
    )]
    fn paint_search_area_tail(
        &mut self,
        ui: &mut egui::Ui,
        rect: egui::Rect,
        preview_image_size: egui::Vec2,
        edges: (Option<i32>, Option<i32>, Option<i32>, Option<i32>),
        screen_click: &ScreenClickBridge,
        record_tip: &'static str,
        recording_msg: &'static str,
        known: &KnownVariableNames,
        is_dark: bool,
        active_macro: Option<&Macro>,
    ) {
        self.paint_area_record(ui, rect, screen_click, record_tip, recording_msg);
        self.paint_area_chips(
            ui,
            rect,
            preview_image_size,
            edges,
            known,
            is_dark,
            active_macro,
        );
    }

    fn paint_monitor_slot(&mut self, ui: &mut egui::Ui) {
        crate::widgets::drag_field(
            ui,
            egui_phosphor::regular::MONITOR,
            "Monitor",
            help::DE_MONITOR,
            &mut self.drafts.monitor,
            |d| d.speed(1).range(1..=8),
        );
    }

    pub(crate) fn load_form(&mut self, catalog: CatalogView<'_>, settings: &UserSettings) {
        self.clear_status();
        form_state::load_tab(self.nav.tab, self, catalog, settings);
    }

    pub(crate) fn load_overlay_form(&mut self, settings: &UserSettings) {
        let Some(id) = self.nav.selected_entity.as_deref() else {
            self.reset_overlay_form();
            return;
        };
        let Some(btn) = settings.overlay.overlay_buttons.iter().find(|b| b.id == id) else {
            self.reset_overlay_form();
            return;
        };
        self.drafts.overlay.point = btn.point.clone();
        self.drafts.overlay.x = btn.x;
        self.drafts.overlay.y = btn.y;
        self.drafts.overlay.macro_name = btn.macro_name.clone();
        self.drafts.overlay.enabled = btn.enabled;
        self.drafts.overlay.icon = if btn.icon.trim().is_empty() {
            overlay_icons::DEFAULT_ICON_ID.into()
        } else {
            btn.icon.clone()
        };
        self.drafts.overlay.size = if btn.size > 0.0 {
            btn.size
        } else {
            DEFAULT_OVERLAY_BUTTON_SIZE
        };
        self.drafts.overlay.load_style_from_config(btn);
        self.drafts.overlay.gate =
            super::form_fields::OverlayGateFields::from_config(&btn.visibility_gate);
    }

    pub(crate) fn reset_overlay_form(&mut self) {
        self.drafts.overlay = super::form_fields::OverlayFields::default();
    }

    pub(crate) fn reset_item_form(&mut self) {
        self.drafts.name.clear();
        self.reset_item_param_fields();
    }

    /// Tags / grid / mask buffers used by Items and ScreenCap New Item (keeps Name).
    pub(crate) fn reset_item_param_fields(&mut self) {
        self.drafts.cols = "1".into();
        self.drafts.rows = "1".into();
        self.drafts.item.stack_max = "0".into();
        self.drafts.item.mask.clear();
        self.drafts.tags.clear();
        self.drafts.tag_draft.clear();
    }

    pub(crate) fn reset_mask_form(&mut self) {
        self.drafts.name.clear();
        self.drafts.mask.shape = "rectangle".into();
        self.drafts.mask.center_x = "50".into();
        self.drafts.mask.center_y = "50".into();
        self.drafts.mask.base.clear();
        self.drafts.mask.height.clear();
        self.drafts.mask.radius.clear();
        self.drafts.mask.inverse = false;
    }

    pub(crate) fn reset_collection_form(&mut self) {
        self.drafts.search_area.clear();
        self.drafts.rows = "1".into();
        self.drafts.cols = "1".into();
    }

    pub(crate) fn reset_atlas_form(&mut self) {
        self.drafts.name.clear();
        self.drafts.atlas.members.clear();
        self.drafts.atlas.add.clear();
    }

    /// Returns true when a tag was committed this frame (auto-Update).
    pub(crate) fn draw_form(
        &mut self,
        ui: &mut egui::Ui,
        paint: &mut CatalogPaint<'_>,
        screen_click: &ScreenClickBridge,
        macros: &[Macro],
        active_macro: Option<&Macro>,
        settings: &mut UserSettings,
    ) -> bool {
        let known = active_macro
            .map(collect_known_variable_names)
            .unwrap_or_default();
        let ctx = FormCtx {
            screen_click,
            macros,
            active_macro,
            known: &known,
            is_dark: ui.visuals().dark_mode,
        };
        match self.nav.tab {
            EditorTab::Programs => self.draw_programs_form(ui, paint, ctx),
            EditorTab::Items => self.draw_items_form(ui, paint),
            EditorTab::Points => {
                self.draw_points_form(ui, paint, ctx);
                false
            }
            EditorTab::SearchAreas => {
                self.draw_search_areas_form(ui, paint, ctx);
                false
            }
            EditorTab::Masks => {
                self.draw_masks_form(ui, paint, ctx);
                false
            }
            EditorTab::Collections => {
                self.draw_collections_form(ui, paint);
                false
            }
            EditorTab::Atlases => {
                self.draw_atlases_form(ui, paint);
                false
            }
            EditorTab::ScreenCap => {
                self.draw_screen_cap_form(ui, paint, ctx);
                false
            }
            EditorTab::MatchCheck => {
                self.paint_match_check_form(ui, paint, ctx, settings);
                false
            }
            EditorTab::Overlay => self.draw_overlay_form(ui, paint, ctx),
        }
    }

    pub(super) fn paint_match_check_form(
        &mut self,
        ui: &mut egui::Ui,
        paint: &mut CatalogPaint<'_>,
        ctx: FormCtx<'_>,
        settings: &UserSettings,
    ) {
        let catalog = paint.catalog;
        let CatalogPaint {
            icons, previews, ..
        } = paint;
        let FormCtx {
            screen_click,
            active_macro,
            known,
            is_dark,
            ..
        } = ctx;
        #[cfg(not(feature = "native-runtime"))]
        {
            let _ = (
                catalog,
                icons,
                previews,
                screen_click,
                active_macro,
                known,
                is_dark,
                settings,
            );
            crate::widgets::error_icon(
                ui,
                &format!(
                    "{} requires the desktop app.",
                    EditorTab::MatchCheck.label()
                ),
            );
        }
        #[cfg(feature = "native-runtime")]
        {
            use crate::data_editor_preview::variant_display_label;
            use crate::pickers::CoordKind;
            use crate::widgets::match_settings;
            use sqyre_domain::CoordinateRef;

            if self.nav.selected_entity.is_none() {
                self.match_check.stop_compute();
                return;
            }
            paint_name_field(ui, &mut self.drafts.name);
            let ref_val = self.drafts.search_area.clone();
            let reference = CoordinateRef(ref_val.clone());
            pickers::coord_ref_field(
                ui,
                catalog,
                icons,
                &mut self.prompts.window_picker,
                CoordKind::SearchArea,
                &ref_val,
                pickers::CoordRefFieldOpts {
                    icon: crate::model_icons::SEARCH_AREA,
                    pick_tip: "Pick search area or collection cell…",
                    display: reference.as_str(),
                    program: reference.program(),
                    color: None,
                    truncate: false,
                    clear_tip: None,
                },
                |ui, resp| {
                    if !reference.is_empty() {
                        let kind = if reference.is_collection() {
                            PreviewKind::Collection
                        } else {
                            PreviewKind::SearchArea
                        };
                        previews.show_for_coordinate_ref(
                            ui,
                            resp,
                            catalog,
                            &reference,
                            kind,
                            help::DE_SCREENCAP_REF,
                        );
                    } else {
                        help::tip(resp.clone(), help::DE_SCREENCAP_REF);
                    }
                },
            );
            if let (Some(prog), Some(item)) = (
                self.nav.selected_program.as_deref(),
                self.nav.selected_entity.as_deref(),
            ) {
                let variants = super::match_check::variant_options(catalog, prog, item);
                if !variants.is_empty() {
                    let mut label =
                        variant_display_label(&self.match_check.settings.variant).to_string();
                    let labels: Vec<&str> = variants.iter().map(|(_, l)| l.as_str()).collect();
                    crate::widgets::combo_str(
                        ui,
                        egui_phosphor::regular::STACK,
                        "Variant",
                        "",
                        &mut label,
                        &labels,
                    );
                    if let Some((name, _)) = variants.iter().find(|(_, l)| l == &label) {
                        if name != &self.match_check.settings.variant {
                            self.match_check.settings.variant = name.clone();
                            self.match_check.invalidate();
                        }
                    }
                }
            }
            match_settings::paint_match_settings(
                ui,
                &mut self.match_check.settings.tolerance,
                &mut self.match_check.settings.blur,
                &mut self.match_check.settings.match_scale_percent,
                &mut self.match_check.settings.match_method,
                false,
            );
            crate::widgets::advanced_section(ui, "de_matchcheck_match", |ui| {
                match_settings::paint_match_method(ui, &mut self.match_check.settings.match_method);
                match_settings::paint_plane_flip(ui, &mut self.match_check.settings.plane_flip);
            });
            help::tip(
                crate::widgets::wrap_unit(ui, |ui| {
                    self.paint_monitor_slot(ui);
                })
                .response,
                help::DE_MATCHCHECK_BOUNDS,
            );
            let (lx, ty, rx, by) = form_desktop_area(
                catalog,
                self.drafts.monitor,
                &self.drafts.rect.left,
                &self.drafts.rect.top,
                &self.drafts.rect.right,
                &self.drafts.rect.bottom,
            );
            let (Some(prog), Some(item)) = (
                self.nav.selected_program.as_deref(),
                self.nav.selected_entity.as_deref(),
            ) else {
                self.match_check.stop_compute();
                return;
            };
            let can_compute = super::match_check::can_compute_match_check(
                catalog,
                prog,
                item,
                &self.match_check.settings.variant,
                lx,
                ty,
                rx,
                by,
            );
            if !can_compute {
                self.match_check.stop_compute();
            }
            self.sync_coord_preview_view();
            let now = web_time::Instant::now();
            // Soft recapture / rematch mid-gesture drops egui's drag target and
            // can resize the underlay — wait until the pointer is up.
            let preview_interacting =
                self.previews.coord_panning() || ui.input(|i| i.pointer.primary_down());
            let manual_force = self.previews.take_force();
            if manual_force {
                // ↻: hard recapture + rematch once the new display is ready.
                self.match_check.settings.awaiting_panel = true;
                self.match_check.settings.rematch_once = false;
                self.match_check.settings.paused = false;
            }
            if can_compute
                && !preview_interacting
                && super::match_check::live_refresh_due(
                    &self.match_check.settings,
                    self.match_check.pending.is_some(),
                    now,
                )
            {
                super::match_check::arm_live_cycle(&mut self.match_check.settings, now);
            }
            ui.horizontal(|ui| {
                let mut live = !self.match_check.settings.paused;
                let tip = if live {
                    "Pause live preview"
                } else {
                    "Resume live preview"
                };
                if crate::widgets::icon_toggle(
                    ui,
                    &mut live,
                    tip,
                    egui_phosphor::regular::PLAY,
                    egui_phosphor::regular::PAUSE,
                )
                .changed()
                {
                    self.match_check.settings.paused = !live;
                    if live {
                        super::match_check::arm_live_cycle(
                            &mut self.match_check.settings,
                            web_time::Instant::now(),
                        );
                    } else {
                        self.match_check.settings.capture_once = false;
                        self.match_check.settings.awaiting_panel = false;
                        self.match_check.settings.rematch_once = false;
                    }
                }
                if live {
                    ui.weak("Live");
                } else {
                    ui.weak("Paused");
                }
                // Only show the spinner on the first compute (no prior heatmap).
                if self.match_check.pending.is_some()
                    && can_compute
                    && self.match_check.cache.is_none()
                {
                    crate::widgets::brand_spinner(ui);
                    ui.weak("Computing heatmap…");
                }
            });
            // Keep one-shot flags parked while the user is dragging.
            let soft = if preview_interacting {
                false
            } else {
                std::mem::take(&mut self.match_check.settings.capture_once)
            };
            let refresh = if manual_force {
                crate::preview_tooltip::PreviewRefresh::Hard
            } else if soft {
                crate::preview_tooltip::PreviewRefresh::Soft
            } else {
                crate::preview_tooltip::PreviewRefresh::None
            };
            // Freeze underlay swaps for the whole paint so an in-flight soft
            // capture cannot resize the texture under an active drag.
            previews.set_hold_pending_swaps(preview_interacting);
            let preview_h = ui.available_height().max(120.0);
            let mut hover: Option<super::match_check::MatchCheckHover> = None;
            let preview_w = crate::widgets::visible_width(ui);
            ui.allocate_ui_with_layout(
                egui::vec2(preview_w, preview_h),
                egui::Layout::top_down(egui::Align::Min),
                |ui| {
                    ui.set_max_width(preview_w);
                    let (rect, preview_image_size) = previews.paint_search_area_panel(
                        ui,
                        lx,
                        ty,
                        rx,
                        by,
                        refresh,
                        &mut self.previews.coord.transform,
                    );
                    self.paint_area_record(
                        ui,
                        rect,
                        screen_click,
                        ht::ED_REC_SEARCH,
                        "Recording… click two corners.",
                    );
                    if let Some(cache) = &self.match_check.cache {
                        let inputs = super::match_check::inputs_key(
                            self.nav.selected_program.as_deref().unwrap_or(""),
                            self.nav.selected_entity.as_deref().unwrap_or(""),
                            &self.match_check.settings.variant,
                            lx,
                            ty,
                            rx,
                            by,
                            self.match_check.settings.blur,
                            self.match_check.settings.match_scale_percent,
                            self.match_check.settings.match_method,
                            self.match_check.settings.plane_flip,
                            self.match_check.settings.tolerance,
                            self.match_check.settings.refresh_gen,
                            settings.runtime.image_search_close_matches_distance,
                        );
                        // Keep the prior overlay while capture/rematch is in flight.
                        if cache.data.fingerprint == inputs
                            || self.match_check.pending.is_some()
                            || self.match_check.settings.awaiting_panel
                        {
                            let image_size =
                                egui::vec2(cache.data.image_w as f32, cache.data.image_h as f32);
                            super::match_check::paint_heatmap_overlay(
                                ui,
                                rect,
                                image_size,
                                &self.previews.coord.transform,
                                cache,
                                &mut hover,
                                self.match_check.settings.match_method,
                                self.match_check.settings.tolerance as f32,
                            );
                        }
                    }
                    self.paint_area_chips(
                        ui,
                        rect,
                        preview_image_size,
                        (lx, ty, rx, by),
                        known,
                        is_dark,
                        active_macro,
                    );
                },
            );
            previews.set_hold_pending_swaps(false);
            if let Some(cache) = &self.match_check.cache {
                super::match_check::paint_legend(
                    ui,
                    &cache.data.summary,
                    cache.data.tolerance_matches.len(),
                    hover.as_ref(),
                    self.match_check.settings.match_method,
                    self.match_check.settings.tolerance as f32,
                    cache.data.tmpl_w,
                    cache.data.tmpl_h,
                );
                if cache.data.mask_missing {
                    crate::widgets::warn_icon(
                        ui,
                        "Mask image missing or unreadable — preview ran unmasked.",
                    );
                }
            }
            if can_compute {
                let panel_pending = match (lx, ty, rx, by) {
                    (Some(l), Some(t), Some(r), Some(b)) => {
                        previews.panel_search_area_pending(l, t, r, b)
                    }
                    _ => false,
                };
                let source = match (lx, ty, rx, by) {
                    (Some(l), Some(t), Some(r), Some(b)) => previews.panel_match_source(l, t, r, b),
                    _ => None,
                };
                // Rematch only after the soft/hard panel capture has a ready frame.
                if !preview_interacting
                    && self.match_check.settings.awaiting_panel
                    && source.is_some()
                    && !panel_pending
                {
                    self.match_check.settings.awaiting_panel = false;
                    self.match_check.settings.rematch_once = true;
                }
                let force_match = !preview_interacting && self.match_check.settings.rematch_once;
                if !preview_interacting || self.match_check.cache.is_none() {
                    self.request_match_check(
                        catalog,
                        lx,
                        ty,
                        rx,
                        by,
                        force_match,
                        settings.runtime.image_search_close_matches_distance,
                        source,
                    );
                }
                if self.match_check.pending.is_some() {
                    self.match_check.settings.rematch_once = false;
                }
            }
            if can_compute {
                if preview_interacting
                    || refresh != crate::preview_tooltip::PreviewRefresh::None
                    || self.match_check.pending.is_some()
                    || self.match_check.settings.rematch_once
                    || self.match_check.settings.awaiting_panel
                {
                    ui.ctx().request_repaint();
                } else if !self.match_check.settings.paused {
                    ui.ctx()
                        .request_repaint_after(super::match_check::LIVE_INTERVAL);
                }
            }
        }
    }

    pub(crate) fn is_dirty(&self, catalog: CatalogView<'_>, settings: &UserSettings) -> bool {
        form_state::dirty_tab(self.nav.tab, self, catalog, settings)
    }

    pub(crate) fn form_valid(&self, active_macro: Option<&Macro>) -> bool {
        form_state::valid_tab(self.nav.tab, self, active_macro)
    }
}
