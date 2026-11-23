//! Point, Search Area, Mask, ScreenCap, and Pixel Check form bodies.
//!
//! Split out of `draw_form`; see `forms.rs` for the tab dispatch.

use super::super::mask_preview::{
    ensure_mask_scrim, mask_percent, paint_mask_scrim_over, MaskItemsFilter, MaskShapeParams,
};
use super::super::{DataEditor, PendingConfirm};
use super::*;
use crate::help_text as ht;
use crate::image_view;
use sqyre_domain::PROGRAM_DELIMITER;

impl DataEditor {
    /// The four area-edge chips (`form_top/bottom/left/right`) shared by the
    /// point, search-area, and collection bounds forms.
    pub(super) fn area_edge_fields(
        &mut self,
    ) -> [(&mut String, CardinalEdge, &'static str, &'static str); 4] {
        [
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
        ]
    }

    pub(super) fn draw_points_form(
        &mut self,
        ui: &mut egui::Ui,
        paint: &mut CatalogPaint<'_>,
        ctx: FormCtx<'_>,
    ) {
        let CatalogPaint {
            catalog, previews, ..
        } = paint;
        let FormCtx {
            screen_click,
            active_macro,
            known,
            is_dark,
            ..
        } = ctx;
        paint_name_field(ui, &mut self.form_name);
        self.paint_monitor_slot(ui);
        let (x, y) = form_absolute_xy(
            catalog,
            self.form_monitor,
            form_coord_literal(&self.form_x),
            form_coord_literal(&self.form_y),
        );
        self.sync_coord_preview_view();
        let force = std::mem::take(&mut self.preview_force);
        let rect = previews.paint_point_panel(ui, x, y, force, &mut self.coord_preview);
        self.paint_point_tail(ui, rect, screen_click, known, is_dark, active_macro);
    }

    pub(super) fn draw_search_areas_form(
        &mut self,
        ui: &mut egui::Ui,
        paint: &mut CatalogPaint<'_>,
        ctx: FormCtx<'_>,
    ) {
        let CatalogPaint {
            catalog, previews, ..
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
        paint_name_field(ui, &mut self.form_name);
        self.paint_monitor_slot(ui);
        self.sync_coord_preview_view();
        let force = std::mem::take(&mut self.preview_force);
        let (rect, preview_image_size) =
            previews.paint_search_area_panel(ui, lx, ty, rx, by, force, &mut self.coord_preview);
        self.paint_search_area_tail(
            ui,
            rect,
            preview_image_size,
            (lx, ty, rx, by),
            screen_click,
            ht::ED_REC_AREA,
            "Recording… click two corners.",
            known,
            is_dark,
            active_macro,
        );
    }

    pub(super) fn draw_masks_form(
        &mut self,
        ui: &mut egui::Ui,
        paint: &mut CatalogPaint<'_>,
        ctx: FormCtx<'_>,
    ) {
        let CatalogPaint { catalog, icons, .. } = paint;
        let FormCtx {
            active_macro,
            known,
            is_dark,
            ..
        } = ctx;
        let has_image = self
            .selected_program
            .as_deref()
            .zip(self.selected_entity.as_deref())
            .map(|(p, m)| catalog.mask_image_path(p, m).is_file())
            .unwrap_or(false);
        let can_upload = self.selected_program.is_some() && self.selected_entity.is_some();
        let mask_path = self
            .selected_program
            .as_deref()
            .zip(self.selected_entity.as_deref())
            .map(|(p, m)| catalog.mask_image_path(p, m));

        // fill_row trailing is right-to-left: first widget is rightmost.
        let mut upload_clicked = false;
        let mut remove_clicked = false;
        crate::widgets::fill_row(
            ui,
            |ui| {
                remove_clicked = ui
                    .add_enabled(
                        has_image,
                        crate::widgets::IconButton::new(
                            egui_phosphor::regular::ERASER,
                            ht::ED_REMOVE_MASK_IMAGE,
                            Some(crate::theme::MACRO_STOP),
                        ),
                    )
                    .clicked();
                upload_clicked = ui
                    .add_enabled(
                        can_upload,
                        crate::widgets::IconButton::new(
                            egui_phosphor::regular::UPLOAD_SIMPLE,
                            ht::ED_UPLOAD_IMAGE,
                            None,
                        ),
                    )
                    .clicked();
                let thumb = crate::widgets::controls::icon_btn_side(ui);
                let (thumb_rect, _) =
                    ui.allocate_exact_size(egui::vec2(thumb, thumb), egui::Sense::hover());
                image_view::paint_checkerboard(
                    ui.painter(),
                    thumb_rect,
                    2.0,
                    image_view::CheckerboardEdge::Flush,
                );
                if let Some(path) = mask_path.as_ref().filter(|p| p.is_file()) {
                    if let Some(tex) = icons.for_path(ui.ctx(), path) {
                        let [tw, th] = tex.size();
                        let size = image_view::fit_icon_thumb(
                            tw as f32,
                            th as f32,
                            thumb_rect.width(),
                            thumb_rect.height(),
                        );
                        let img_rect = egui::Rect::from_center_size(thumb_rect.center(), size);
                        ui.painter().image(
                            tex.id(),
                            img_rect,
                            egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                            egui::Color32::WHITE,
                        );
                    }
                }
                help::tip(
                    ui.interact(
                        thumb_rect,
                        ui.id().with("mask_image_thumb"),
                        egui::Sense::hover(),
                    ),
                    help::DE_MASK_IMAGE_MODE,
                );
            },
            |ui| {
                name_field_body(ui, &mut self.form_name);
            },
        );
        paint_fs_name_hint(ui, &self.form_name);
        if upload_clicked {
            self.request_mask_image_pick();
        }
        if remove_clicked {
            self.confirm = Some(PendingConfirm::DeleteMaskImage);
        }

        if !has_image {
            crate::widgets::pair_labeled(ui, egui_phosphor::regular::SHAPES, "Shape", |ui| {
                help::tip(
                    ui.selectable_value(&mut self.form_shape, "rectangle".into(), "Rectangle"),
                    help::DE_MASK_SHAPE,
                );
                help::tip(
                    ui.selectable_value(&mut self.form_shape, "circle".into(), "Circle"),
                    help::DE_MASK_SHAPE,
                );
            });
            ui.horizontal(|ui| {
                help::tip(
                    ui.checkbox(
                        &mut self.form_inverse,
                        "Inverse (shape included, rest excluded)",
                    ),
                    help::DE_MASK_INVERSE,
                );
            });
            crate::widgets::advanced_section(ui, "de_mask_geometry", |ui| {
                let numeric =
                    |ui: &mut egui::Ui, icon: &str, label: &str, value: &mut String, help: &str| {
                        let validation = validate_numeric_expression(value, active_macro);
                        crate::widgets::num_or_var_field(
                            ui,
                            icon,
                            label,
                            value,
                            crate::widgets::NumOrVarOpts {
                                width: f32::INFINITY,
                                range: None,
                                theme: VarTheme {
                                    known_vars: known,
                                    is_dark,
                                },
                                validation: &validation,
                                help,
                            },
                        );
                    };
                numeric(
                    ui,
                    egui_phosphor::regular::ARROWS_HORIZONTAL,
                    "Center X %",
                    &mut self.form_center_x,
                    "Horizontal center of the shape (0–100%).",
                );
                numeric(
                    ui,
                    egui_phosphor::regular::ARROWS_VERTICAL,
                    "Center Y %",
                    &mut self.form_center_y,
                    "Vertical center of the shape (0–100%).",
                );
                if self.form_shape == "circle" {
                    numeric(
                        ui,
                        egui_phosphor::regular::CIRCLE,
                        "Radius",
                        &mut self.form_radius,
                        "Circle radius as a percent of the search area.",
                    );
                } else {
                    numeric(
                        ui,
                        egui_phosphor::regular::ARROWS_LEFT_RIGHT,
                        "Base",
                        &mut self.form_base,
                        "Rectangle width as a percent of the search area.",
                    );
                    numeric(
                        ui,
                        egui_phosphor::regular::ARROWS_DOWN_UP,
                        "Height",
                        &mut self.form_height,
                        "Rectangle height as a percent of the search area.",
                    );
                }
            });
        }

        crate::widgets::section_separator(ui);
        crate::widgets::combo_enum(
            ui,
            crate::model_icons::ITEM,
            "Items",
            help::DE_MASK_ITEMS_FILTER,
            &mut self.mask_items_filter,
            &MaskItemsFilter::ALL,
            MaskItemsFilter::label,
        );
        help::tip(
            ui.label(egui::RichText::new("Live mask preview").weak()),
            help::DE_MASK_ITEMS,
        );

        let Some(prog) = self.selected_program.as_deref() else {
            crate::widgets::list_vacancy(ui, "", 0, "items");
            return;
        };
        let mask_name = self.selected_entity.as_deref().unwrap_or("");
        let filter = self.mask_items_filter;
        let targets: Vec<String> = catalog
            .get(prog)
            .map(|p| {
                p.items
                    .iter()
                    .filter(|(_, item)| match filter {
                        MaskItemsFilter::AllItems => true,
                        MaskItemsFilter::UsingThisMask => {
                            !mask_name.is_empty() && item.mask == mask_name
                        }
                    })
                    .map(|(name, _)| format!("{prog}{PROGRAM_DELIMITER}{name}"))
                    .collect()
            })
            .unwrap_or_default();

        if targets.is_empty() {
            let entity = match filter {
                MaskItemsFilter::AllItems => "items",
                MaskItemsFilter::UsingThisMask => "items using this mask",
            };
            crate::widgets::list_vacancy(ui, "", 0, entity);
            return;
        }

        let shape = MaskShapeParams {
            circle: self.form_shape == "circle",
            inverse: self.form_inverse,
            center_x: mask_percent(&self.form_center_x, 50.0),
            center_y: mask_percent(&self.form_center_y, 50.0),
            base: mask_percent(&self.form_base, 50.0),
            height: mask_percent(&self.form_height, 50.0),
            radius: mask_percent(&self.form_radius, 50.0),
        };
        let png = mask_path
            .as_ref()
            .filter(|p| p.is_file())
            .map(|p| p.as_path());
        let overlay = ensure_mask_scrim(ui.ctx(), &mut self.mask_scrim_cache, 64, png, shape);
        let mut after_icon = |painter: &egui::Painter, icon_rect: egui::Rect, _target: &str| {
            if let Some(tex) = overlay.as_ref() {
                paint_mask_scrim_over(painter, icon_rect, tex);
            }
        };
        pickers::paint_even_icon_grid(
            ui,
            catalog,
            icons,
            &targets,
            |_| false,
            pickers::IconGridKind::Picker,
            pickers::IconGridOps {
                after_icon: Some(&mut after_icon),
                ..Default::default()
            },
            None,
        );
    }

    pub(super) fn draw_screen_cap_form(
        &mut self,
        ui: &mut egui::Ui,
        paint: &mut CatalogPaint<'_>,
        ctx: FormCtx<'_>,
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
        paint_name_field(ui, &mut self.form_name);
        self.paint_item_param_fields(ui, catalog, icons);
        help::tip(
            crate::widgets::wrap_unit(ui, |ui| {
                self.paint_monitor_slot(ui);
            })
            .response,
            help::DE_BOUNDS_PREVIEW,
        );
        let (lx, ty, rx, by) = form_desktop_area(
            catalog,
            self.form_monitor,
            &self.form_left,
            &self.form_top,
            &self.form_right,
            &self.form_bottom,
        );
        self.sync_coord_preview_view();
        let force = std::mem::take(&mut self.preview_force);
        // Keep the button row below the preview (panel fills remaining height);
        // the `Saves to …` path lives on the Save button's tooltip.
        let path_hint = format!("Saves to {}", screen_cap_path().display());
        let spacing = ui.spacing().item_spacing.y;
        let wrap_w = crate::widgets::visible_width(ui);
        // spacing + 8px gap + spacing + button (+ 1px slack).
        let footer_h = spacing * 2.0 + 8.0 + ui.spacing().interact_size.y + 1.0;
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
                self.paint_search_area_tail(
                    ui,
                    rect,
                    preview_image_size,
                    (lx, ty, rx, by),
                    screen_click,
                    ht::ED_REC_CAPTURE,
                    "Recording… click two corners.",
                    known,
                    is_dark,
                    active_macro,
                );
            },
        );
        let saving = self.screen_cap_pending.is_some();
        ui.horizontal(|ui| {
            if saving {
                crate::widgets::brand_spinner(ui);
            }
            let save_tip = if saving {
                "Saving…".to_owned()
            } else {
                path_hint.clone()
            };
            if ui
                .add_enabled(
                    !saving,
                    crate::widgets::IconButton::new(
                        egui_phosphor::regular::FLOPPY_DISK,
                        &save_tip,
                        None,
                    ),
                )
                .clicked()
            {
                self.save_screen_cap(catalog, previews);
                ui.ctx().request_repaint();
            }
            if help::tip(
                crate::widgets::add_button(ui, "New item", !saving),
                help::DE_SCREENCAP_NEW_ITEM,
            )
            .clicked()
            {
                self.screen_cap_new_item = true;
            }
        });
    }
}
