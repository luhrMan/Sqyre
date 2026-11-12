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
    ) -> bool {
        let CatalogPaint {
            catalog,
            icons,
            previews,
            ..
        } = paint;
        let FormCtx { macros, .. } = ctx;
        if self.selected_program.is_none() || self.selected_entity.is_none() {
            return false;
        }
        let hub = self
            .selected_entity
            .as_deref()
            .is_some_and(sqyre_persist::is_quick_access_hub_id);
        let icon = overlay_icons::resolve(&self.form_overlay_icon);
        crate::widgets::pair_labeled(ui, egui_phosphor::regular::SMILEY, icon.label, |ui| {
            let mut preview_cfg = OverlayButtonConfig::new("preview", "");
            self.apply_overlay_style_to_config(&mut preview_cfg);
            if hub {
                preview_cfg.icon = sqyre_persist::QUICK_ACCESS_HUB_ICON.into();
                preview_cfg.border_width = 0.0;
                preview_cfg.border_alpha = 0;
                preview_cfg.bg_alpha = 0;
            }
            let style = overlay_icons::OverlayPaintStyle::from_config(&preview_cfg);
            let preview = overlay_icons::style_preview_button(ui, icon, 48.0, &style)
                .on_hover_text(if hub {
                    help::DE_OVERLAY_HUB
                } else {
                    help::DE_OVERLAY_ICON
                });
            if !hub && preview.clicked() {
                if let Some(id) = self.selected_entity.clone() {
                    self.overlay_icon_search.clear();
                    self.overlay_icon_picker_for = Some(id);
                }
            }
        });
        if hub {
            help::tip(
                ui.label(sqyre_persist::QUICK_ACCESS_HUB_LABEL),
                help::DE_OVERLAY_HUB,
            );
            ui.weak("Drag on screen to move; snaps to a screen edge.");
        } else {
            let macro_names: Vec<String> = macros.iter().map(|m| m.name.clone()).collect();
            let _ = crate::widgets::labeled_searchable_combo(
                ui,
                crate::model_icons::MACRO,
                "Macro",
                help::DE_OVERLAY_MACRO,
                "overlay_form_macro",
                &mut self.form_overlay_macro,
                crate::widgets::SearchableComboOpts {
                    options: &macro_names,
                    empty_text: "(pick macro)",
                    none_label: Some(sqyre_domain::EMPTY_NONE),
                    width: Some(220.0),
                    on_option_hover: None,
                    option_icon: None,
                    closed_icon: true,
                    option_label: None,
                },
                None,
            );
        }
        if !hub {
            use crate::pickers::CoordKind;
            use sqyre_domain::CoordinateRef;
            let point_val = self.form_overlay_point.clone();
            let point = CoordinateRef(point_val.clone());
            let display = if point.is_empty() {
                "(unset — use X/Y below)"
            } else {
                point.as_str()
            };
            let clear_tip = (!point_val.is_empty())
                .then_some("Clear — use manual X/Y instead of a catalog point");
            if pickers::coord_ref_field(
                ui,
                catalog,
                icons,
                &mut self.window_picker,
                CoordKind::Point,
                &point_val,
                pickers::CoordRefFieldOpts {
                    icon: crate::model_icons::POINT,
                    pick_tip: "Pick point…",
                    display,
                    program: point.program(),
                    color: None,
                    truncate: false,
                    clear_tip,
                },
                |ui, resp| {
                    if !point.is_empty() {
                        previews.show_for_coordinate_ref(
                            ui,
                            resp,
                            catalog,
                            &point,
                            PreviewKind::Point,
                            help::DE_OVERLAY_POINT,
                        );
                    } else {
                        help::tip(resp.clone(), help::DE_OVERLAY_POINT);
                    }
                },
            ) {
                self.form_overlay_point.clear();
            }
        }
        let point_set = !hub && !self.form_overlay_point.trim().is_empty();
        ui.add_enabled_ui(!point_set, |ui| {
            crate::widgets::wrapped_row(ui, |ui| {
                crate::widgets::drag_field(
                    ui,
                    egui_phosphor::regular::ARROWS_HORIZONTAL,
                    "X",
                    if hub {
                        help::DE_OVERLAY_HUB
                    } else {
                        help::DE_OVERLAY_X
                    },
                    &mut self.form_overlay_x,
                    |d| d.speed(1.0).suffix(" px"),
                );
                crate::widgets::drag_field(
                    ui,
                    egui_phosphor::regular::ARROWS_VERTICAL,
                    "Y",
                    if hub {
                        help::DE_OVERLAY_HUB
                    } else {
                        help::DE_OVERLAY_Y
                    },
                    &mut self.form_overlay_y,
                    |d| d.speed(1.0).suffix(" px"),
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
        if hub {
            crate::widgets::drag_field(
                ui,
                egui_phosphor::regular::RULER,
                "Size",
                help::DE_OVERLAY_HUB_SIZE,
                &mut self.form_overlay_size,
                |d| {
                    d.speed(1)
                        .range(MIN_OVERLAY_BUTTON_SIZE..=MAX_OVERLAY_BUTTON_SIZE)
                },
            );
            // Full-color logo: only alpha is applied on the face.
            {
                let mut opacity = i32::from(self.form_overlay_icon_color.a());
                let before = opacity;
                crate::widgets::drag_field(
                    ui,
                    egui_phosphor::regular::DROP,
                    "Opacity",
                    help::DE_OVERLAY_HUB_OPACITY,
                    &mut opacity,
                    |d| d.range(0..=255).speed(1),
                );
                if opacity != before {
                    let c = self.form_overlay_icon_color;
                    self.form_overlay_icon_color = egui::Color32::from_rgba_unmultiplied(
                        c.r(),
                        c.g(),
                        c.b(),
                        opacity.clamp(0, 255) as u8,
                    );
                }
            }
            return false;
        }
        crate::widgets::collapsible_section(
            ui,
            "Show only when image found",
            "de_overlay_gate",
            |ui| {
                ui.horizontal(|ui| {
                    help::tip(
                        ui.checkbox(&mut self.form_overlay_gate_enabled, "Enabled"),
                        help::DE_OVERLAY_GATE,
                    );
                });
                ui.add_enabled_ui(self.form_overlay_gate_enabled, |ui| {
                    {
                        use crate::pickers::CoordKind;
                        let area_val = self.form_overlay_gate_search_area.clone();
                        let area = CoordinateRef(area_val.clone());
                        let display = if area.is_empty() {
                            "(pick search area)"
                        } else {
                            area.as_str()
                        };
                        let clear_tip = (!area_val.is_empty()).then_some("Clear search area");
                        if pickers::coord_ref_field(
                            ui,
                            catalog,
                            icons,
                            &mut self.window_picker,
                            CoordKind::SearchArea,
                            &area_val,
                            pickers::CoordRefFieldOpts {
                                icon: crate::model_icons::SEARCH_AREA,
                                pick_tip: "Pick search area…",
                                display,
                                program: area.program(),
                                color: None,
                                truncate: false,
                                clear_tip,
                            },
                            |ui, resp| {
                                if !area.is_empty() {
                                    previews.show_for_coordinate_ref(
                                        ui,
                                        resp,
                                        catalog,
                                        &area,
                                        PreviewKind::SearchArea,
                                        help::DE_OVERLAY_GATE_AREA,
                                    );
                                } else {
                                    help::tip(resp.clone(), help::DE_OVERLAY_GATE_AREA);
                                }
                            },
                        ) {
                            self.form_overlay_gate_search_area.clear();
                        }
                    }
                    crate::widgets::pair_leading(
                        ui,
                        |ui| {
                            if crate::widgets::entity_pick_button(
                                ui,
                                crate::model_icons::ITEM,
                                "Add / edit gate items",
                            )
                            .clicked()
                            {
                                self.window_picker = pickers::ActivePicker::Items {
                                    search: String::new(),
                                    staged: self.form_overlay_gate_targets.clone(),
                                    staged_tags: None,
                                };
                            }
                        },
                        |ui| {
                            help::tip(
                                ui.label(
                                    egui::RichText::new(format!(
                                        "({})",
                                        self.form_overlay_gate_targets.len()
                                    ))
                                    .weak(),
                                ),
                                help::DE_OVERLAY_GATE_ITEMS,
                            );
                        },
                    );
                    if self.form_overlay_gate_targets.is_empty() {
                        ui.label(sqyre_domain::EMPTY_NONE);
                    } else {
                        let mut remove: Option<usize> = None;
                        let snapshot = self.form_overlay_gate_targets.clone();
                        let mut on_remove = |i| {
                            remove = Some(i);
                        };
                        pickers::paint_even_icon_grid(
                            ui,
                            catalog,
                            icons,
                            &snapshot,
                            |_| true,
                            pickers::IconGridKind::Targets { removable: true },
                            pickers::IconGridOps {
                                on_remove: Some(&mut on_remove),
                                ..Default::default()
                            },
                            None,
                        );
                        if let Some(i) = remove {
                            self.form_overlay_gate_targets.remove(i);
                        }
                    }
                    match_settings::paint_match_settings_advanced(
                        ui,
                        &mut self.form_overlay_gate_tolerance,
                        &mut self.form_overlay_gate_blur,
                        &mut self.form_overlay_gate_match_method,
                        |ui, method| {
                            crate::widgets::advanced_section(ui, "de_overlay_gate_adv", |ui| {
                                match_settings::paint_match_method(ui, method);
                                crate::widgets::drag_field(
                                    ui,
                                    egui_phosphor::regular::TIMER,
                                    "Interval",
                                    help::DE_OVERLAY_GATE_INTERVAL,
                                    &mut self.form_overlay_gate_interval_ms,
                                    |d| {
                                        d.speed(50)
                                            .range(
                                                MIN_OVERLAY_GATE_INTERVAL_MS
                                                    ..=MAX_OVERLAY_GATE_INTERVAL_MS,
                                            )
                                            .suffix(" ms")
                                    },
                                );
                            });
                        },
                    );
                });
            },
        );
        // Size is common-path — keep it outside collapsed Appearance.
        crate::widgets::drag_field(
            ui,
            egui_phosphor::regular::RULER,
            "Size",
            help::DE_OVERLAY_SIZE,
            &mut self.form_overlay_size,
            |d| {
                d.speed(1)
                    .range(MIN_OVERLAY_BUTTON_SIZE..=MAX_OVERLAY_BUTTON_SIZE)
            },
        );
        crate::widgets::collapsible_section(ui, "Appearance", "de_overlay_appearance", |ui| {
            crate::widgets::wrapped_row(ui, |ui| {
                crate::widgets::drag_field(
                    ui,
                    egui_phosphor::regular::CIRCLE,
                    "Corner radius",
                    help::DE_OVERLAY_RADIUS,
                    &mut self.form_overlay_corner_radius,
                    |d| {
                        d.speed(0.5)
                            .range(MIN_OVERLAY_CORNER_RADIUS..=MAX_OVERLAY_CORNER_RADIUS)
                            .suffix(" px")
                    },
                );
                crate::widgets::drag_field(
                    ui,
                    egui_phosphor::regular::FRAME_CORNERS,
                    "Border width",
                    help::DE_OVERLAY_BORDER,
                    &mut self.form_overlay_border_width,
                    |d| {
                        d.speed(0.1)
                            .range(MIN_OVERLAY_BORDER_WIDTH..=MAX_OVERLAY_BORDER_WIDTH)
                            .suffix(" px")
                    },
                );
            });
            ui.horizontal(|ui| {
                color_alpha_drag(ui, "Border", &mut self.form_overlay_border);
            });
            ui.horizontal(|ui| {
                color_alpha_drag(ui, "Background", &mut self.form_overlay_bg);
            });
            ui.horizontal(|ui| {
                color_alpha_drag(ui, "Icon", &mut self.form_overlay_icon_color);
            });
            crate::widgets::pair_labeled(
                ui,
                egui_phosphor::regular::HAND_POINTING,
                "Icon hover",
                |ui| {
                    help::tip(
                        ui.color_edit_button_srgba(&mut self.form_overlay_icon_hover),
                        help::DE_OVERLAY_ICON_HOVER,
                    );
                },
            );
            if ui.small_button("Reset appearance to defaults").clicked() {
                self.reset_overlay_style_form();
            }
        });
        // This tab never commits a tag; the early returns above say the same.
        false
    }
}
