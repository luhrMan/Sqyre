//! Appearance settings section.

use super::common::*;
use super::*;
use sqyre_domain::format_hex_color;
use sqyre_domain::ACTION_COLOR_CATEGORIES;
use sqyre_ui_model::{
    action_pastel_color, clear_all_custom_action_colors, clear_custom_action_color,
    default_action_pastel_color, sample_action_type_for_color_key, set_custom_action_color,
};

impl SettingsUi {
    pub(super) fn draw_appearance(
        &mut self,
        ui: &mut egui::Ui,
        ctx: &egui::Context,
        q: &str,
        section_hit: bool,
    ) {
        if setting_visible(q, section_hit, SETTING_COMPACT_HEADERS)
            && dirty_checkbox(
                ui,
                &mut self.settings.ui.compact_program_headers,
                "Compact program headers with icons",
                ht::SET_ICON_HEADERS,
            )
        {
            self.mark_dirty();
        }

        if setting_visible(q, section_hit, SETTING_EMPTY_HEADERS)
            && dirty_checkbox(
                ui,
                &mut self.settings.ui.show_empty_program_headers,
                "Show empty program headers",
                ht::SET_KEEP_HEADERS,
            )
        {
            self.mark_dirty();
        }

        if setting_visible(q, section_hit, SETTING_WINDOW_BORDER)
            && dirty_checkbox(
                ui,
                &mut self.settings.ui.window_border,
                "Window border",
                ht::SET_WIN_OUTLINE,
            )
        {
            self.mark_dirty();
        }

        if setting_visible(q, section_hit, SETTING_WINDOW_DECORATIONS)
            && dirty_checkbox(
                ui,
                &mut self.settings.ui.window_decorations,
                "OS title bar",
                ht::SET_WIN_DECORATIONS,
            )
        {
            ctx.send_viewport_cmd(egui::ViewportCommand::Decorations(
                self.settings.ui.window_decorations,
            ));
            // Keep resize grips even after OS chrome is removed.
            ctx.send_viewport_cmd(egui::ViewportCommand::Resizable(true));
            self.mark_dirty();
        }

        if setting_visible(q, section_hit, SETTING_RUN_GLOW)
            && dirty_checkbox(
                ui,
                &mut self.settings.ui.run_button_glow,
                "Glowing Run button",
                ht::SET_RUN_GLOW,
            )
        {
            self.mark_dirty();
        }

        if setting_visible(q, section_hit, SETTING_PREVIEW_OUTLINE)
            && dirty_checkbox(
                ui,
                &mut self.settings.ui.preview_desktop_outline,
                "Outline preview locations on the desktop",
                ht::SET_PREVIEW_OUTLINE,
            )
        {
            self.mark_dirty();
        }

        if setting_visible(q, section_hit, SETTING_ACCENT) {
            ui.horizontal(|ui| {
                ui.label("Accent color:").on_hover_text(ht::SET_ACCENT);
                let mut color = sqyre_ui_theme::accent();
                if ui.color_edit_button_srgba(&mut color).changed() {
                    self.settings.ui.accent_color =
                        format_hex_color([color.r(), color.g(), color.b(), 255]);
                    self.mark_dirty();
                    Self::apply_appearance(ctx, &self.settings);
                }
                let (rect, _) =
                    ui.allocate_exact_size(egui::vec2(22.0, 22.0), egui::Sense::hover());
                ui.painter().rect_filled(rect, 3.0, color);
                ui.painter().rect_stroke(
                    rect,
                    3.0,
                    egui::Stroke::new(1.0, ui.visuals().widgets.noninteractive.bg_stroke.color),
                    egui::StrokeKind::Outside,
                );
                if !self.settings.ui.accent_color.trim().is_empty()
                    && settings_reset_btn(ui, "Reset accent color").clicked()
                {
                    self.settings.ui.accent_color.clear();
                    self.mark_dirty();
                    Self::apply_appearance(ctx, &self.settings);
                }
            });
        }

        if setting_visible(q, section_hit, SETTING_FONT_FAMILY) {
            ui.horizontal(|ui| {
                ui.label("Font family:").on_hover_text(ht::SET_FONT_FAMILY);
                let mut family = self.settings.ui.ui_font_family;
                egui::ComboBox::from_id_salt("settings_ui_font_family")
                    .selected_text(family.label())
                    .show_ui(ui, |ui| {
                        for f in UiFontFamily::ALL {
                            ui.selectable_value(&mut family, f, f.label());
                        }
                    });
                if family != self.settings.ui.ui_font_family {
                    self.settings.ui.ui_font_family = family;
                    self.mark_dirty();
                    Self::apply_appearance(ctx, &self.settings);
                }
            });
        }

        if setting_visible(q, section_hit, SETTING_MONO_SIZE) {
            ui.horizontal(|ui| {
                ui.label("Code font size:").on_hover_text(ht::SET_MONO_SIZE);
                let mut v = self.settings.ui.mono_font_size;
                if ui
                    .add(crate::widgets::num_entry(&mut v).range(0..=40).speed(1))
                    .on_hover_text(ht::SET_MONO_SIZE)
                    .changed()
                {
                    self.settings.ui.mono_font_size = v;
                    self.mark_dirty();
                    Self::apply_appearance(ctx, &self.settings);
                }
                if settings_reset_btn(ui, "Reset to default").clicked() {
                    self.settings.ui.mono_font_size = 0;
                    self.mark_dirty();
                    Self::apply_appearance(ctx, &self.settings);
                }
            });
            ui.label(
                egui::RichText::new("0 matches the UI font size.")
                    .small()
                    .weak(),
            );
        }

        if setting_visible(q, section_hit, SETTING_FONT_SIZE) {
            ui.horizontal(|ui| {
                ui.label("Font size:");
                let mut v = self.settings.ui.ui_font_size;
                if ui
                    .add(crate::widgets::num_entry(&mut v).range(10..=28).speed(1))
                    .on_hover_text(ht::SET_FONT_SIZE)
                    .changed()
                {
                    self.settings.ui.ui_font_size = v;
                    self.mark_dirty();
                    Self::apply_appearance(ctx, &self.settings);
                }
                if settings_reset_btn(ui, "Reset to default").clicked() {
                    self.settings.ui.ui_font_size = DEFAULT_UI_FONT_SIZE;
                    self.mark_dirty();
                    Self::apply_appearance(ctx, &self.settings);
                }
            });
        }

        if setting_visible(q, section_hit, SETTING_UI_SCALE) {
            ui.horizontal(|ui| {
                ui.label("UI scale:");
                let mut v = self.settings.ui.ui_scale;
                if ui
                    .add(
                        crate::widgets::num_entry(&mut v)
                            .range(MIN_UI_SCALE..=MAX_UI_SCALE)
                            .speed(0.1)
                            .fixed_decimals(1),
                    )
                    .on_hover_text(ht::SET_UI_SCALE)
                    .changed()
                {
                    self.settings.ui.ui_scale = v;
                    self.mark_dirty();
                    self.ui_scale_apply_pending = true;
                }
                if settings_reset_btn(ui, "Reset to default").clicked() {
                    self.settings.ui.ui_scale = DEFAULT_UI_SCALE;
                    self.mark_dirty();
                    self.ui_scale_apply_pending = false;
                    Self::apply_appearance(ctx, &self.settings);
                }
            });
        }

        if self.ui_scale_apply_pending && !ctx.input(|i| i.pointer.any_down()) {
            Self::apply_appearance(ctx, &self.settings);
            self.ui_scale_apply_pending = false;
        }

        if setting_visible(q, section_hit, SETTING_SPACING) {
            ui.horizontal(|ui| {
                ui.label("Spacing:");
                let mut density = self.settings.ui.ui_density;
                egui::ComboBox::from_id_salt("settings_ui_density")
                    .selected_text(density.label())
                    .show_ui(ui, |ui| {
                        for d in UiDensity::ALL {
                            ui.selectable_value(&mut density, d, d.label());
                        }
                    })
                    .response
                    .on_hover_text(ht::SET_DENSITY);
                if density != self.settings.ui.ui_density {
                    self.settings.ui.ui_density = density;
                    self.mark_dirty();
                    Self::apply_appearance(ctx, &self.settings);
                }
                if dirty_checkbox(
                    ui,
                    &mut self.settings.ui.auto_compact_density,
                    "Auto-compact",
                    ht::SET_AUTO_COMPACT,
                ) {
                    self.mark_dirty();
                    Self::apply_appearance(ctx, &self.settings);
                }
            });
        }

        let show_colors = setting_visible(q, section_hit, SETTING_ACTION_COLORS)
            || ACTION_COLOR_CATEGORIES
                .iter()
                .any(|&(_, label)| setting_visible(q, false, &[label]));
        if show_colors {
            ui.label(egui::RichText::new("Macro tree action colors").strong());

            let is_dark = ui.visuals().dark_mode;
            for &(key, label) in ACTION_COLOR_CATEGORIES {
                if !setting_visible(q, section_hit, SETTING_ACTION_COLORS)
                    && !setting_visible(q, false, &[label])
                {
                    continue;
                }
                ui.horizontal_wrapped(|ui| {
                    ui.label(label);
                    // Keep right-aligned controls clear of the floating V-bar overlay.
                    crate::widgets::clamp_row_right_to_clip(ui);
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if settings_reset_btn(ui, "Reset to default").clicked() {
                            self.settings.ui.action_colors.set(key, String::new());
                            clear_custom_action_color(key);
                            self.mark_dirty();
                        }

                        let sample = sample_action_type_for_color_key(key);
                        let current = if self.settings.ui.action_colors.get(key).is_empty() {
                            default_action_pastel_color(sample, is_dark)
                        } else {
                            action_pastel_color(sample, is_dark)
                        };
                        let mut color = Color32::from_rgba_unmultiplied(
                            current[0], current[1], current[2], current[3],
                        );
                        if ui.color_edit_button_srgba(&mut color).changed() {
                            let rgba = [color.r(), color.g(), color.b(), 255];
                            self.settings
                                .ui
                                .action_colors
                                .set(key, format_hex_color(rgba));
                            set_custom_action_color(key, rgba);
                            self.mark_dirty();
                        }

                        // Swatch preview
                        let (rect, _) =
                            ui.allocate_exact_size(egui::vec2(22.0, 22.0), egui::Sense::hover());
                        ui.painter().rect_filled(rect, 3.0, color);
                        ui.painter().rect_stroke(
                            rect,
                            3.0,
                            egui::Stroke::new(
                                1.0,
                                ui.visuals().widgets.noninteractive.bg_stroke.color,
                            ),
                            egui::StrokeKind::Outside,
                        );
                    });
                });
            }

            if setting_visible(q, section_hit, SETTING_ACTION_COLORS)
                && settings_reset_btn(ui, "Reset all action colors").clicked()
            {
                self.settings.ui.action_colors.clear_all();
                clear_all_custom_action_colors();
                self.mark_dirty();
            }
        }
    }
}
