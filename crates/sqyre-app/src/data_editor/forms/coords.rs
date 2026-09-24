//! Point, Search Area, Mask, ScreenCap, and Pixel Check form bodies.
//!
//! Split out of `draw_form`; see `forms.rs` for the tab dispatch.

use super::super::DataEditor;
use super::*;

impl DataEditor {
    pub(super) fn draw_points_form(
        &mut self,
        ui: &mut egui::Ui,
        paint: &mut CatalogPaint<'_>,
        ctx: FormCtx<'_>,
        settings: &mut UserSettings,
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
        help::heading(ui, "Point", help::DE_POINT_COORDS);
        self.program_selector(ui, catalog, icons, settings);
        ui.add_space(4.0);
        self.paint_name_record_row(
            ui,
            screen_click,
            "Click on screen to capture X/Y",
            "Recording… left-click to capture.",
            ScreenClickBridge::arm_point,
        );
        self.paint_monitor_slot(ui);
        let (x, y) = form_absolute_xy(
            catalog,
            self.form_monitor,
            form_coord_literal(&self.form_x),
            form_coord_literal(&self.form_y),
        );
        self.sync_coord_preview_view();
        let force = paint_preview_toolbar(ui, Some(&mut self.coord_preview));
        let rect = previews.paint_point_panel(ui, x, y, force, &mut self.coord_preview);
        paint_coord_chips(
            ui,
            rect,
            known,
            is_dark,
            active_macro,
            &mut [
                (&mut self.form_x, CardinalEdge::Left, "X", help::DE_POINT_X),
                (
                    &mut self.form_y,
                    CardinalEdge::Bottom,
                    "Y",
                    help::DE_POINT_Y,
                ),
            ],
        );
    }

    pub(super) fn draw_search_areas_form(
        &mut self,
        ui: &mut egui::Ui,
        paint: &mut CatalogPaint<'_>,
        ctx: FormCtx<'_>,
        settings: &mut UserSettings,
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
        let (lx, ty, rx, by) = form_desktop_area(
            catalog,
            self.form_monitor,
            &self.form_left,
            &self.form_top,
            &self.form_right,
            &self.form_bottom,
        );
        ui.horizontal(|ui| {
            ui.heading("Search Area");
            help::icon(ui, help::DE_AREA_BOUNDS);
            if let Some((w, h)) = area_size_from_opts(lx, ty, rx, by) {
                ui.weak(pixel_size_text(w, h));
            }
        });
        self.program_selector(ui, catalog, icons, settings);
        ui.add_space(4.0);
        self.paint_name_record_row(
            ui,
            screen_click,
            "Two clicks: opposite corners of the area",
            "Recording… click two corners.",
            ScreenClickBridge::arm_search_area,
        );
        self.paint_monitor_slot(ui);
        self.sync_coord_preview_view();
        let force = paint_preview_toolbar(ui, Some(&mut self.coord_preview));
        let (rect, preview_image_size) =
            previews.paint_search_area_panel(ui, lx, ty, rx, by, force, &mut self.coord_preview);
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
    }

    pub(super) fn draw_masks_form(
        &mut self,
        ui: &mut egui::Ui,
        paint: &mut CatalogPaint<'_>,
        ctx: FormCtx<'_>,
        settings: &mut UserSettings,
    ) {
        let CatalogPaint { catalog, icons, .. } = paint;
        let FormCtx {
            active_macro,
            known,
            is_dark,
            ..
        } = ctx;
        ui.heading("Mask");
        self.program_selector(ui, catalog, icons, settings);
        ui.add_space(4.0);
        help::label(ui, "Name", help::DE_NAME);
        ui.add(egui::TextEdit::singleline(&mut self.form_name).desired_width(f32::INFINITY));
        paint_fs_name_hint(ui, &self.form_name);
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
                .add_enabled(
                    has_image,
                    egui::Button::new(
                        egui::RichText::new("Remove Image").color(crate::theme::MACRO_STOP),
                    ),
                )
                .on_hover_text("Delete the PNG and use shape geometry instead.")
                .clicked()
            {
                self.remove_mask_image(catalog, icons);
            }
            if has_image {
                help::icon(ui, help::DE_MASK_IMAGE_MODE);
            }
        });
        if !has_image {
            ui.add_space(4.0);
            help::label(ui, "Shape", help::DE_MASK_SHAPE);
            ui.horizontal(|ui| {
                ui.selectable_value(&mut self.form_shape, "rectangle".into(), "Rectangle");
                ui.selectable_value(&mut self.form_shape, "circle".into(), "Circle");
            });
            ui.horizontal(|ui| {
                ui.checkbox(
                    &mut self.form_inverse,
                    "Inverse (shape included, rest excluded)",
                );
                help::icon(ui, help::DE_MASK_INVERSE);
            });
            ui.add_space(4.0);
            let cx = validate_numeric_expression(&self.form_center_x, active_macro);
            var_pills::validated_var_ref_edit(
                ui,
                "Center X %",
                &mut self.form_center_x,
                VarTheme {
                    known_vars: known,
                    is_dark,
                },
                VarFieldOpts {
                    desired_width: f32::INFINITY,
                    validation: &cx,
                    help: "Horizontal center of the shape (0–100%).",
                },
            );
            let cy = validate_numeric_expression(&self.form_center_y, active_macro);
            var_pills::validated_var_ref_edit(
                ui,
                "Center Y %",
                &mut self.form_center_y,
                VarTheme {
                    known_vars: known,
                    is_dark,
                },
                VarFieldOpts {
                    desired_width: f32::INFINITY,
                    validation: &cy,
                    help: "Vertical center of the shape (0–100%).",
                },
            );
            if self.form_shape == "circle" {
                let radius = validate_numeric_expression(&self.form_radius, active_macro);
                var_pills::validated_var_ref_edit(
                    ui,
                    "Radius",
                    &mut self.form_radius,
                    VarTheme {
                        known_vars: known,
                        is_dark,
                    },
                    VarFieldOpts {
                        desired_width: f32::INFINITY,
                        validation: &radius,
                        help: "Circle radius as a percent of the search area.",
                    },
                );
            } else {
                let base = validate_numeric_expression(&self.form_base, active_macro);
                var_pills::validated_var_ref_edit(
                    ui,
                    "Base",
                    &mut self.form_base,
                    VarTheme {
                        known_vars: known,
                        is_dark,
                    },
                    VarFieldOpts {
                        desired_width: f32::INFINITY,
                        validation: &base,
                        help: "Rectangle width as a percent of the search area.",
                    },
                );
                let height = validate_numeric_expression(&self.form_height, active_macro);
                var_pills::validated_var_ref_edit(
                    ui,
                    "Height",
                    &mut self.form_height,
                    VarTheme {
                        known_vars: known,
                        is_dark,
                    },
                    VarFieldOpts {
                        desired_width: f32::INFINITY,
                        validation: &height,
                        help: "Rectangle height as a percent of the search area.",
                    },
                );
            }
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

    pub(super) fn draw_screen_cap_form(
        &mut self,
        ui: &mut egui::Ui,
        paint: &mut CatalogPaint<'_>,
        ctx: FormCtx<'_>,
        settings: &mut UserSettings,
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
        help::heading(ui, "Screen capture", help::DE_SCREENCAP_INTRO);
        ui.add_space(4.0);
        self.program_selector(ui, catalog, icons, settings);
        self.paint_name_record_row(
            ui,
            screen_click,
            "Two clicks: opposite corners of the capture region",
            "Recording… click two corners.",
            ScreenClickBridge::arm_search_area,
        );
        ui.add_space(4.0);
        self.paint_item_param_fields(ui, catalog, icons);
        ui.add_space(4.0);
        help::label(ui, "Bounds", help::DE_BOUNDS_PREVIEW);
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
        self.sync_coord_preview_view();
        let force = paint_preview_toolbar(ui, Some(&mut self.coord_preview));
        // Keep Save + path hint below the preview (panel fills remaining height).
        let path_hint = format!("Saves to {}", screen_cap_path().display());
        let spacing = ui.spacing().item_spacing.y;
        let path_font = egui::TextStyle::Body.resolve(ui.style());
        let wrap_w = crate::widgets::visible_width(ui);
        let path_h = ui.fonts_mut(|f| {
            f.layout(path_hint.clone(), path_font, egui::Color32::WHITE, wrap_w)
                .size()
                .y
        });
        // spacing + 8px gap + spacing + button + spacing + path (+ 1px slack).
        let footer_h = spacing * 3.0 + 8.0 + ui.spacing().interact_size.y + path_h + 1.0;
        let preview_h = (ui.available_height() - footer_h).max(120.0);
        ui.allocate_ui_with_layout(
            egui::vec2(wrap_w, preview_h),
            egui::Layout::top_down(egui::Align::Min),
            |ui| {
                ui.set_max_width(wrap_w);
                let (rect, preview_image_size) = previews.paint_search_area_panel(
                    ui,
                    lx,
                    ty,
                    rx,
                    by,
                    force,
                    &mut self.coord_preview,
                );
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
        ui.add_space(8.0);
        let saving = self.screen_cap_pending.is_some();
        ui.horizontal(|ui| {
            if ui
                .add_enabled(
                    !saving,
                    egui::Button::new(if saving { "Saving…" } else { "Save" }),
                )
                .clicked()
            {
                self.save_screen_cap(catalog, previews);
                ui.ctx().request_repaint();
            }
            if ui
                .add_enabled(
                    !saving,
                    egui::Button::new(
                        egui::RichText::new("New Item").color(crate::theme::MACRO_START),
                    ),
                )
                .on_hover_text(help::DE_SCREENCAP_NEW_ITEM)
                .clicked()
            {
                self.screen_cap_new_item = true;
            }
        });
        ui.weak(path_hint);
    }
}
