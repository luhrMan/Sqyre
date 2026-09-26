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
    area_size_from_opts, catalog_search_area_pixel_size, collection_size_label, paint_disk_preview,
    paint_preview_coord_chip, paint_preview_toolbar, paint_search_area_preview_sizes,
    paint_zoomable_atlas_preview, paint_zoomable_collection_preview, pixel_size_text,
    show_file_hover, CardinalEdge,
};
use crate::overlay_icons;
use crate::paint_ctx::CatalogPaint;
use crate::paint_ctx::VarTheme;
use crate::pickers;
use crate::preview_tooltip::PreviewKind;
use crate::theme;
use crate::var_pills::{self, VarFieldOpts};
use crate::widgets::{match_settings, searchable_combo_width, searchable_combo_with};
use eframe::egui;
use sqyre_domain::{
    collect_known_variable_names, CoordinateRef, KnownVariableNames, Macro, PROGRAM_DELIMITER,
};
use sqyre_hotkeys::ScreenClickBridge;
use sqyre_persist::{
    default_overlay_position, screen_cap_path, OverlayButtonConfig, ProgramCatalog,
    ProgramCollection, UserSettings, DEFAULT_OVERLAY_BUTTON_SIZE,
    DEFAULT_OVERLAY_FALLBACK_SCREEN_H, DEFAULT_OVERLAY_FALLBACK_SCREEN_W, MAX_OVERLAY_BORDER_WIDTH,
    MAX_OVERLAY_BUTTON_SIZE, MAX_OVERLAY_CORNER_RADIUS, MAX_OVERLAY_GATE_INTERVAL_MS,
    MIN_OVERLAY_BORDER_WIDTH, MIN_OVERLAY_BUTTON_SIZE, MIN_OVERLAY_CORNER_RADIUS,
    MIN_OVERLAY_GATE_INTERVAL_MS,
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

fn paint_fs_name_hint(ui: &mut egui::Ui, name: &str) {
    let name = name.trim();
    if name.is_empty() {
        return;
    }
    if let Err(e) = validate_entity_name(name) {
        ui.colored_label(crate::theme::error_fg(), e.to_string());
    }
}

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
            self.tab,
            self.selected_program.clone().unwrap_or_default(),
            self.selected_entity.clone().unwrap_or_default(),
        );
        if self.coord_preview_key.as_ref() != Some(&key) {
            self.coord_preview.reset();
            self.coord_preview_key = Some(key);
        }
    }

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
                    egui::TextEdit::singleline(&mut self.form_name).desired_width(f32::INFINITY),
                ),
                help::DE_NAME,
            );
            let armed = screen_click.is_armed();
            if crate::widgets::record_icon_button(ui, record_tip, !armed).clicked() {
                self.save_after_record = false;
                arm(screen_click);
                self.set_ok(recording_msg);
            }
            if armed && ui.button("Cancel").clicked() {
                self.save_after_record = false;
                screen_click.disarm();
            }
        });
        paint_fs_name_hint(ui, &self.form_name);
    }

    fn paint_monitor_slot(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label("Monitor");
            ui.add(
                egui::DragValue::new(&mut self.form_monitor)
                    .speed(1)
                    .range(1..=8),
            );
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
        self.form_overlay_point = btn.point.clone();
        self.form_overlay_x = btn.x;
        self.form_overlay_y = btn.y;
        self.form_overlay_macro = btn.macro_name.clone();
        self.form_overlay_enabled = btn.enabled;
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
        self.load_overlay_gate_from_config(&btn.visibility_gate);
    }

    pub(crate) fn reset_overlay_form(&mut self) {
        self.form_overlay_point.clear();
        let (x, y) = default_overlay_position(
            0.0,
            0.0,
            DEFAULT_OVERLAY_FALLBACK_SCREEN_W,
            DEFAULT_OVERLAY_FALLBACK_SCREEN_H,
            DEFAULT_OVERLAY_BUTTON_SIZE,
            0,
        );
        self.form_overlay_x = x;
        self.form_overlay_y = y;
        self.form_overlay_macro.clear();
        self.form_overlay_enabled = true;
        self.form_overlay_icon = overlay_icons::DEFAULT_ICON_ID.into();
        self.form_overlay_size = DEFAULT_OVERLAY_BUTTON_SIZE;
        self.reset_overlay_style_form();
        self.reset_overlay_gate_form();
    }

    pub(crate) fn reset_item_form(&mut self) {
        self.form_name.clear();
        self.reset_item_param_fields();
    }

    /// Tags / grid / mask buffers used by Items and ScreenCap New Item (keeps Name).
    pub(crate) fn reset_item_param_fields(&mut self) {
        self.form_cols = "1".into();
        self.form_rows = "1".into();
        self.form_stack_max = "0".into();
        self.form_mask.clear();
        self.form_tags.clear();
        self.tag_draft.clear();
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

    pub(crate) fn reset_atlas_form(&mut self) {
        self.form_name.clear();
        self.form_atlas_members.clear();
        self.form_atlas_add.clear();
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
        match self.tab {
            EditorTab::Programs => self.draw_programs_form(ui, paint, ctx),
            EditorTab::Items => self.draw_items_form(ui, paint, settings),
            EditorTab::Points => {
                self.draw_points_form(ui, paint, ctx, settings);
                false
            }
            EditorTab::SearchAreas => {
                self.draw_search_areas_form(ui, paint, ctx, settings);
                false
            }
            EditorTab::Masks => {
                self.draw_masks_form(ui, paint, ctx, settings);
                false
            }
            EditorTab::Collections => {
                self.draw_collections_form(ui, paint, settings);
                false
            }
            EditorTab::Atlases => {
                self.draw_atlases_form(ui, paint, settings);
                false
            }
            EditorTab::ScreenCap => {
                self.draw_screen_cap_form(ui, paint, ctx, settings);
                false
            }
            EditorTab::PixelCheck => {
                self.paint_pixel_check_form(ui, paint, ctx, settings);
                false
            }
            EditorTab::Overlay => self.draw_overlay_form(ui, paint, ctx, settings),
        }
    }

    pub(super) fn paint_pixel_check_form(
        &mut self,
        ui: &mut egui::Ui,
        paint: &mut CatalogPaint<'_>,
        ctx: FormCtx<'_>,
        settings: &UserSettings,
    ) {
        let CatalogPaint {
            catalog,
            icons,
            previews,
            ..
        } = paint;
        let FormCtx {
            screen_click,
            active_macro,
            known,
            is_dark,
            ..
        } = ctx;
        #[cfg(feature = "native-runtime")]
        help::heading(ui, EditorTab::PixelCheck.label(), help::DE_PIXELCHECK_INTRO);
        #[cfg(not(feature = "native-runtime"))]
        ui.heading(EditorTab::PixelCheck.label());
        #[cfg(not(feature = "native-runtime"))]
        {
            let _ = (
                catalog,
                previews,
                screen_click,
                active_macro,
                known,
                is_dark,
                settings,
            );
            ui.colored_label(
                crate::theme::error_fg(),
                format!(
                    "{} requires the desktop app.",
                    EditorTab::PixelCheck.label()
                ),
            );
            return;
        }
        #[cfg(feature = "native-runtime")]
        {
            use crate::data_editor_preview::variant_display_label;
            use crate::pickers::{ActivePicker, CoordKind};
            use crate::widgets::match_settings;
            use sqyre_domain::CoordinateRef;

            ui.add_space(4.0);
            if self.selected_entity.is_none() {
                self.stop_pixel_check_compute();
                ui.weak("Select an item from the list.");
                return;
            }
            self.paint_name_record_row(
                ui,
                screen_click,
                "Two clicks: opposite corners of the search area",
                "Recording… click two corners.",
                ScreenClickBridge::arm_search_area,
            );
            let reference = CoordinateRef(self.form_search_area.clone());
            let display = if reference.is_empty() {
                "(optional — seed coords from a search area or cell)"
            } else {
                reference.as_str()
            };
            ui.horizontal(|ui| {
                help::label(ui, "Reference", help::DE_SCREENCAP_REF);
                if let Some(prog) = reference.program() {
                    crate::icon_cache::paint_program_icon(ui, catalog, icons, prog);
                }
                let resp = ui.monospace(display);
                if !reference.is_empty() {
                    let kind = if reference.is_collection() {
                        PreviewKind::Collection
                    } else {
                        PreviewKind::SearchArea
                    };
                    previews.show_for_coordinate_ref(ui, &resp, catalog, &reference, kind);
                }
                if crate::widgets::icon_button(ui, "☰", "Pick search area or collection cell…")
                    .clicked()
                {
                    self.window_picker = ActivePicker::Coord {
                        kind: CoordKind::SearchArea,
                        search: String::new(),
                        value: self.form_search_area.clone(),
                        cell_pick: None,
                        scroll_to_selection: true,
                    };
                }
            });
            ui.add_space(4.0);
            if let (Some(prog), Some(item)) = (
                self.selected_program.as_deref(),
                self.selected_entity.as_deref(),
            ) {
                let variants = super::pixel_check::variant_options(catalog, prog, item);
                if !variants.is_empty() {
                    let mut label = variant_display_label(&self.pixel_check.variant).to_string();
                    let labels: Vec<&str> = variants.iter().map(|(_, l)| l.as_str()).collect();
                    crate::widgets::combo_str(ui, "Variant", "", &mut label, &labels);
                    if let Some((name, _)) = variants.iter().find(|(_, l)| l == &label) {
                        if name != &self.pixel_check.variant {
                            self.pixel_check.variant = name.clone();
                            self.invalidate_pixel_check();
                        }
                    }
                }
            }
            ui.add_space(4.0);
            match_settings::paint_match_settings(
                ui,
                &mut self.pixel_check.tolerance,
                &mut self.pixel_check.blur,
                &mut self.pixel_check.match_method,
                false,
            );
            ui.collapsing("Advanced", |ui| {
                match_settings::paint_match_method(ui, &mut self.pixel_check.match_method);
            });
            help::label(ui, "Bounds", help::DE_PIXELCHECK_BOUNDS);
            self.paint_monitor_slot(ui);
            let (lx, ty, rx, by) = form_desktop_area(
                catalog,
                self.form_monitor,
                &self.form_left,
                &self.form_top,
                &self.form_right,
                &self.form_bottom,
            );
            if let Some((w, h)) = area_size_from_opts(lx, ty, rx, by) {
                ui.weak(pixel_size_text(w, h));
            }
            let (Some(prog), Some(item)) = (
                self.selected_program.as_deref(),
                self.selected_entity.as_deref(),
            ) else {
                self.stop_pixel_check_compute();
                return;
            };
            let can_compute = super::pixel_check::can_compute_pixel_check(
                catalog,
                prog,
                item,
                &self.pixel_check.variant,
                lx,
                ty,
                rx,
                by,
            );
            if !can_compute {
                self.stop_pixel_check_compute();
            }
            self.sync_coord_preview_view();
            let force = paint_preview_toolbar(ui, Some(&mut self.coord_preview));
            let computing = self.pixel_check_pending.is_some() && can_compute;
            if computing {
                ui.horizontal(|ui| {
                    ui.spinner();
                    ui.weak("Computing heatmap…");
                });
            }
            let preview_h = ui.available_height().max(120.0);
            let mut hover: Option<super::pixel_check::PixelCheckHover> = None;
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
                        force,
                        &mut self.coord_preview,
                    );
                    if let Some(cache) = &self.pixel_check_cache {
                        let inputs = super::pixel_check::inputs_key(
                            self.selected_program.as_deref().unwrap_or(""),
                            self.selected_entity.as_deref().unwrap_or(""),
                            &self.pixel_check.variant,
                            lx,
                            ty,
                            rx,
                            by,
                            self.pixel_check.blur,
                            self.pixel_check.match_method,
                            self.pixel_check.tolerance,
                            self.pixel_check.refresh_gen,
                            settings.image_search_close_matches_distance,
                        );
                        if cache.fingerprint == inputs {
                            let match_count = cache.tolerance_matches.len();
                            let show_boxes = super::pixel_check::should_paint_match_boxes(
                                match_count,
                                self.pixel_check.show_many_match_boxes,
                            );
                            let image_size = egui::vec2(cache.image_w as f32, cache.image_h as f32);
                            super::pixel_check::paint_heatmap_overlay(
                                ui,
                                rect,
                                image_size,
                                &self.coord_preview,
                                cache,
                                &mut hover,
                                self.pixel_check.match_method,
                                self.pixel_check.tolerance as f32,
                                show_boxes,
                            );
                        }
                    }
                    paint_search_area_preview_sizes(ui, rect, lx, ty, rx, by, preview_image_size);
                    paint_coord_chips(
                        ui,
                        rect,
                        known,
                        is_dark,
                        active_macro,
                        &mut [
                            (
                                &mut self.form_top,
                                CardinalEdge::Top,
                                "TopY",
                                help::DE_AREA_TOP,
                            ),
                            (
                                &mut self.form_bottom,
                                CardinalEdge::Bottom,
                                "BottomY",
                                help::DE_AREA_BOTTOM,
                            ),
                            (
                                &mut self.form_left,
                                CardinalEdge::Left,
                                "LeftX",
                                help::DE_AREA_LEFT,
                            ),
                            (
                                &mut self.form_right,
                                CardinalEdge::Right,
                                "RightX",
                                help::DE_AREA_RIGHT,
                            ),
                        ],
                    );
                },
            );
            if let Some(cache) = &self.pixel_check_cache {
                super::pixel_check::paint_legend(
                    ui,
                    &cache.summary,
                    cache.tolerance_matches.len(),
                    &mut self.pixel_check.show_many_match_boxes,
                    hover.as_ref(),
                    self.pixel_check.tolerance as f32,
                    cache.tmpl_w,
                    cache.tmpl_h,
                );
            }
            if can_compute {
                self.request_pixel_check_match(
                    catalog,
                    lx,
                    ty,
                    rx,
                    by,
                    force,
                    settings.image_search_close_matches_distance,
                );
            }
            if can_compute && (force || lx.is_some()) {
                ui.ctx().request_repaint();
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
