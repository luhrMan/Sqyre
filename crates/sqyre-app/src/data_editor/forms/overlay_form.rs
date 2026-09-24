//! Overlay button form body.
//!
//! Split out of `draw_form`; see `forms.rs` for the tab dispatch.

use super::super::DataEditor;
use super::*;

impl DataEditor {
    pub(super) fn draw_overlay_form(
        &mut self,
        ui: &mut egui::Ui,
        paint: &mut CatalogPaint<'_>,
        ctx: FormCtx<'_>,
        settings: &mut UserSettings,
    ) -> bool {
        let CatalogPaint {
            catalog,
            icons,
            previews,
            ..
        } = paint;
        let FormCtx { macros, .. } = ctx;
        help::heading(ui, "Overlay Button", help::DE_OVERLAY_INTRO);
        ui.add_space(6.0);
        self.program_selector(ui, catalog, icons, settings);
        if self.selected_program.is_none() {
            ui.weak("Select a program, then New to add a button.");
            return false;
        }
        if self.selected_entity.is_none() {
            ui.weak("Select a button from the list, or click New.");
            return false;
        }
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            let icon = overlay_icons::resolve(&self.form_overlay_icon);
            let mut preview_cfg = OverlayButtonConfig::new("preview", "");
            self.apply_overlay_style_to_config(&mut preview_cfg);
            let style = overlay_icons::OverlayPaintStyle::from_config(&preview_cfg);
            let preview = overlay_icons::style_preview_button(ui, icon, 48.0, &style)
                .on_hover_text(help::DE_OVERLAY_ICON);
            if preview.clicked() {
                if let Some(id) = self.selected_entity.clone() {
                    self.overlay_icon_search.clear();
                    self.overlay_icon_picker_for = Some(id);
                }
            }
            ui.vertical(|ui| {
                help::label(ui, icon.label, help::DE_OVERLAY_ICON);
            });
        });
        ui.add_space(6.0);
        help::label(ui, "Macro", help::DE_OVERLAY_MACRO);
        let mut selected = self.form_overlay_macro.clone();
        let before = selected.clone();
        let macro_names: Vec<String> = macros.iter().map(|m| m.name.clone()).collect();
        searchable_combo_width(
            ui,
            "overlay_form_macro",
            &mut selected,
            &macro_names,
            "(pick macro)",
            Some("(none)"),
            Some(220.0),
        );
        if selected != before {
            self.form_overlay_macro = selected;
        }
        ui.add_space(4.0);
        {
            use crate::pickers::{ActivePicker, CoordKind};
            use sqyre_domain::CoordinateRef;
            let point = CoordinateRef(self.form_overlay_point.clone());
            let display = if point.is_empty() {
                "(unset — use X/Y below)"
            } else {
                point.as_str()
            };
            ui.horizontal(|ui| {
                help::label(ui, "Point", help::DE_OVERLAY_POINT);
                if let Some(prog) = point.program() {
                    crate::icon_cache::paint_program_icon(ui, catalog, icons, prog);
                }
                let resp = ui.monospace(display);
                if !point.is_empty() {
                    previews.show_for_coordinate_ref(
                        ui,
                        &resp,
                        catalog,
                        &point,
                        PreviewKind::Point,
                    );
                }
                if crate::theme::icon_button(ui, "☰")
                    .on_hover_text("Pick point…")
                    .clicked()
                {
                    self.window_picker = ActivePicker::Coord {
                        kind: CoordKind::Point,
                        search: String::new(),
                        value: self.form_overlay_point.clone(),
                        cell_pick: None,
                        scroll_to_selection: true,
                    };
                }
                if !self.form_overlay_point.is_empty()
                    && ui
                        .small_button("Clear")
                        .on_hover_text("Use manual X/Y instead of a catalog point")
                        .clicked()
                {
                    self.form_overlay_point.clear();
                }
            });
        }
        ui.add_space(4.0);
        let point_set = !self.form_overlay_point.trim().is_empty();
        ui.add_enabled_ui(!point_set, |ui| {
            ui.horizontal(|ui| {
                help::label(ui, "X", help::DE_OVERLAY_X);
                help::tip(
                    ui.add(
                        egui::DragValue::new(&mut self.form_overlay_x)
                            .speed(1.0)
                            .suffix(" px"),
                    ),
                    help::DE_OVERLAY_X,
                );
                help::label(ui, "Y", help::DE_OVERLAY_Y);
                help::tip(
                    ui.add(
                        egui::DragValue::new(&mut self.form_overlay_y)
                            .speed(1.0)
                            .suffix(" px"),
                    ),
                    help::DE_OVERLAY_Y,
                );
            });
        });
        if point_set {
            let mut loc = OverlayButtonConfig::new("preview", "");
            loc.point = self.form_overlay_point.clone();
            loc.x = self.form_overlay_x;
            loc.y = self.form_overlay_y;
            let (rx, ry) = loc.resolved_position(catalog);
            ui.weak(format!("Position from point → ({rx:.0}, {ry:.0})"));
        }
        ui.add_space(8.0);
        ui.collapsing("Show only when image found", |ui| {
            ui.horizontal(|ui| {
                ui.checkbox(&mut self.form_overlay_gate_enabled, "Enabled");
                help::icon(ui, help::DE_OVERLAY_GATE);
            });
            ui.add_enabled_ui(self.form_overlay_gate_enabled, |ui| {
                {
                    use crate::pickers::{ActivePicker, CoordKind};
                    let area = CoordinateRef(self.form_overlay_gate_search_area.clone());
                    let display = if area.is_empty() {
                        "(pick search area)"
                    } else {
                        area.as_str()
                    };
                    ui.horizontal(|ui| {
                        help::label(ui, "Search area", help::DE_OVERLAY_GATE_AREA);
                        if let Some(prog) = area.program() {
                            crate::icon_cache::paint_program_icon(ui, catalog, icons, prog);
                        }
                        let resp = ui.monospace(display);
                        if !area.is_empty() {
                            previews.show_for_coordinate_ref(
                                ui,
                                &resp,
                                catalog,
                                &area,
                                PreviewKind::SearchArea,
                            );
                        }
                        if crate::theme::icon_button(ui, "☰")
                            .on_hover_text("Pick search area…")
                            .clicked()
                        {
                            self.window_picker = ActivePicker::Coord {
                                kind: CoordKind::SearchArea,
                                search: String::new(),
                                value: self.form_overlay_gate_search_area.clone(),
                                cell_pick: None,
                                scroll_to_selection: true,
                            };
                        }
                        if !self.form_overlay_gate_search_area.is_empty()
                            && ui.small_button("Clear").clicked()
                        {
                            self.form_overlay_gate_search_area.clear();
                        }
                    });
                }
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    help::label(ui, "Items", help::DE_OVERLAY_GATE_ITEMS);
                    ui.label(
                        egui::RichText::new(format!("({})", self.form_overlay_gate_targets.len()))
                            .weak(),
                    );
                    if ui
                        .button(egui::RichText::new("Add / edit…").color(theme::MACRO_START))
                        .clicked()
                    {
                        self.window_picker = pickers::ActivePicker::Items {
                            search: String::new(),
                            staged: self.form_overlay_gate_targets.clone(),
                            staged_tags: None,
                        };
                    }
                });
                if self.form_overlay_gate_targets.is_empty() {
                    ui.label("(none)");
                } else {
                    let mut remove: Option<usize> = None;
                    let snapshot = self.form_overlay_gate_targets.clone();
                    pickers::paint_even_icon_grid(
                        ui,
                        catalog,
                        icons,
                        &snapshot,
                        |_| true,
                        pickers::IconGridKind::Targets { removable: true },
                        |_, _| {},
                        |i| {
                            remove = Some(i);
                        },
                        None,
                        |_| true,
                    );
                    if let Some(i) = remove {
                        self.form_overlay_gate_targets.remove(i);
                    }
                }
                ui.add_space(4.0);
                match_settings::paint_match_settings(
                    ui,
                    &mut self.form_overlay_gate_tolerance,
                    &mut self.form_overlay_gate_blur,
                    &mut self.form_overlay_gate_match_method,
                    false,
                );
                ui.collapsing("Advanced", |ui| {
                    match_settings::paint_match_method(
                        ui,
                        &mut self.form_overlay_gate_match_method,
                    );
                });
                ui.horizontal(|ui| {
                    help::label(ui, "Interval", help::DE_OVERLAY_GATE_INTERVAL);
                    help::tip(
                        ui.add(
                            egui::DragValue::new(&mut self.form_overlay_gate_interval_ms)
                                .speed(50)
                                .range(MIN_OVERLAY_GATE_INTERVAL_MS..=MAX_OVERLAY_GATE_INTERVAL_MS)
                                .suffix(" ms"),
                        ),
                        help::DE_OVERLAY_GATE_INTERVAL,
                    );
                });
            });
        });
        ui.add_space(8.0);
        ui.collapsing("Appearance", |ui| {
            ui.horizontal(|ui| {
                help::label(ui, "Size", help::DE_OVERLAY_SIZE);
                help::tip(
                    ui.add(
                        egui::DragValue::new(&mut self.form_overlay_size)
                            .speed(1)
                            .range(MIN_OVERLAY_BUTTON_SIZE..=MAX_OVERLAY_BUTTON_SIZE),
                    ),
                    help::DE_OVERLAY_SIZE,
                );
            });
            ui.horizontal(|ui| {
                help::label(ui, "Corner radius", help::DE_OVERLAY_RADIUS);
                help::tip(
                    ui.add(
                        egui::DragValue::new(&mut self.form_overlay_corner_radius)
                            .speed(0.5)
                            .range(MIN_OVERLAY_CORNER_RADIUS..=MAX_OVERLAY_CORNER_RADIUS)
                            .suffix(" px"),
                    ),
                    help::DE_OVERLAY_RADIUS,
                );
                help::label(ui, "Border width", help::DE_OVERLAY_BORDER);
                help::tip(
                    ui.add(
                        egui::DragValue::new(&mut self.form_overlay_border_width)
                            .speed(0.1)
                            .range(MIN_OVERLAY_BORDER_WIDTH..=MAX_OVERLAY_BORDER_WIDTH)
                            .suffix(" px"),
                    ),
                    help::DE_OVERLAY_BORDER,
                );
            });
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                color_alpha_drag(ui, "Border", &mut self.form_overlay_border);
            });
            ui.horizontal(|ui| {
                color_alpha_drag(ui, "Background", &mut self.form_overlay_bg);
                help::icon(ui, help::DE_OVERLAY_ALPHA_NONE);
            });
            ui.horizontal(|ui| {
                color_alpha_drag(ui, "Icon", &mut self.form_overlay_icon_color);
            });
            ui.horizontal(|ui| {
                help::label(ui, "Icon hover", help::DE_OVERLAY_ICON_HOVER);
                ui.color_edit_button_srgba(&mut self.form_overlay_icon_hover);
            });
            ui.add_space(4.0);
            if ui.button("Reset appearance to defaults").clicked() {
                self.reset_overlay_style_form();
            }
        });
        // This tab never commits a tag; the early returns above say the same.
        false
    }
}
