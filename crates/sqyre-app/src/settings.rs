//! User Settings window.

use eframe::egui::{self, Color32};
use sqyre_domain::{
    action_pastel_color, clear_all_custom_action_colors, clear_custom_action_color,
    default_action_pastel_color, format_hex_color, parse_hex_color, sample_action_type_for_color_key,
    set_custom_action_color, ACTION_COLOR_CATEGORIES,
};
use sqyre_domain::Macro;
use sqyre_persist::{
    move_dir, open_sqyre_dir, set_sqyre_dir_override, sqyre_dir, Database, ProgramCatalog,
    UserSettings, DEFAULT_UI_FONT_SIZE, DEFAULT_UI_SCALE,
};
use std::path::PathBuf;

#[derive(Debug, Clone)]
enum PendingConfirm {
    /// Move current data to `new_dir` (Yes) or start fresh (No).
    MoveData {
        old_dir: PathBuf,
        new_dir: PathBuf,
    },
}

pub struct SettingsUi {
    pub open: bool,
    settings: UserSettings,
    dirty: bool,
    status: Option<String>,
    status_error: bool,
    confirm: Option<PendingConfirm>,
    /// Set when the data directory changed and the shell should reload from disk.
    pub reload_requested: bool,
}

impl Default for SettingsUi {
    fn default() -> Self {
        Self {
            open: false,
            settings: UserSettings::default(),
            dirty: false,
            status: None,
            status_error: false,
            confirm: None,
            reload_requested: false,
        }
    }
}

impl SettingsUi {
    pub fn from_settings(settings: UserSettings) -> Self {
        Self {
            settings,
            ..Self::default()
        }
    }

    pub fn settings(&self) -> &UserSettings {
        &self.settings
    }

    pub fn settings_mut(&mut self) -> &mut UserSettings {
        &mut self.settings
    }

    pub fn save_settings(&mut self) -> Result<(), String> {
        self.settings
            .save_default()
            .map_err(|e| e.to_string())
    }

    /// Ensure Hack is in the proportional fallback chain so geometric/arrow
    /// symbols (e.g. ➔ ◫) are available — egui's default omits Hack there.
    pub fn install_fonts(ctx: &egui::Context) {
        let mut fonts = egui::FontDefinitions::default();
        if let Some(prop) = fonts.families.get_mut(&egui::FontFamily::Proportional) {
            if !prop.iter().any(|n| n == "Hack") {
                // After Ubuntu (UI text), before emoji fallbacks.
                let insert_at = prop
                    .iter()
                    .position(|n| n == "Ubuntu-Light")
                    .map(|i| i + 1)
                    .unwrap_or(0);
                prop.insert(insert_at, "Hack".to_owned());
            }
        }
        ctx.set_fonts(fonts);
    }

    /// Apply appearance prefs to the egui context (Sqyre theme, fonts, scale).
    pub fn apply_appearance(ctx: &egui::Context, settings: &UserSettings) {
        let scale = if settings.ui_scale > 0.0 {
            settings.ui_scale
        } else {
            DEFAULT_UI_SCALE
        };
        ctx.set_pixels_per_point(scale);

        crate::theme::apply(ctx);

        let mut style = (*ctx.global_style()).clone();
        let base = settings.ui_font_size.max(10) as f32;
        use egui::TextStyle;
        style.text_styles.insert(
            TextStyle::Small,
            egui::FontId::proportional((base * 0.85).round()),
        );
        style
            .text_styles
            .insert(TextStyle::Body, egui::FontId::proportional(base));
        style.text_styles.insert(
            TextStyle::Button,
            egui::FontId::proportional(base),
        );
        style.text_styles.insert(
            TextStyle::Heading,
            egui::FontId::proportional((base * 1.35).round()),
        );
        style.text_styles.insert(
            TextStyle::Monospace,
            egui::FontId::monospace(base),
        );
        ctx.set_global_style(style);
    }

    /// Load action-color overrides from settings into the domain map.
    pub fn apply_action_colors(settings: &UserSettings) {
        clear_all_custom_action_colors();
        for &(key, _) in ACTION_COLOR_CATEGORIES {
            let hex = settings.action_colors.get(key);
            if hex.is_empty() {
                continue;
            }
            if let Some(rgba) = parse_hex_color(hex) {
                set_custom_action_color(key, rgba);
            }
        }
    }

    pub fn persist(&mut self) {
        self.settings.clamp();
        if let Err(e) = self.settings.save_default() {
            self.set_err(format!("Failed to save settings: {e}"));
            return;
        }
        self.dirty = false;
    }

    fn mark_dirty(&mut self) {
        self.dirty = true;
    }

    fn set_ok(&mut self, msg: impl Into<String>) {
        self.status = Some(msg.into());
        self.status_error = false;
    }

    fn set_err(&mut self, msg: impl Into<String>) {
        self.status = Some(msg.into());
        self.status_error = true;
    }

    pub fn show(
        &mut self,
        ctx: &egui::Context,
        db: &mut Database,
        macros: &mut Vec<Macro>,
        catalog: &mut ProgramCatalog,
    ) {
        if !self.open {
            return;
        }
        let mut open = self.open;
        egui::Window::new("User Settings")
            .open(&mut open)
            .default_size([520.0, 640.0])
            .min_size([400.0, 360.0])
            .resizable(true)
            .constrain(true)
            .show(ctx, |ui| {
                self.ui(ui, ctx, db, macros, catalog);
            });
        self.open = open;
        if self.dirty {
            self.persist();
            Self::apply_appearance(ctx, &self.settings);
        }
    }

    fn ui(
        &mut self,
        ui: &mut egui::Ui,
        ctx: &egui::Context,
        db: &mut Database,
        macros: &mut Vec<Macro>,
        catalog: &mut ProgramCatalog,
    ) {
        if let Some(confirm) = self.confirm.clone() {
            self.draw_confirm(ui, confirm, db, macros, catalog);
            return;
        }

        // Status line may appear below; leave room so the window doesn't grow unboundedly.
        let list_h = crate::pickers::popup_scroll_max_height(ui, 40.0);
        egui::ScrollArea::vertical()
            .max_height(list_h)
            .show(ui, |ui| {
                crate::theme::section_frame(ui.style()).show(ui, |ui| {
                    self.draw_general(ui);
                });
                ui.add_space(12.0);
                crate::theme::section_frame(ui.style()).show(ui, |ui| {
                    self.draw_data(ui, db, macros, catalog);
                });
                ui.add_space(12.0);
                crate::theme::section_frame(ui.style()).show(ui, |ui| {
                    self.draw_appearance(ui, ctx);
                });
            });

        if let Some(status) = &self.status {
            ui.separator();
            if self.status_error {
                ui.colored_label(Color32::RED, status);
            } else {
                ui.label(status);
            }
        }
    }

    fn section_header(ui: &mut egui::Ui, title: &str, subtitle: &str) {
        ui.label(egui::RichText::new(title).strong().size(16.0));
        if !subtitle.is_empty() {
            ui.label(egui::RichText::new(subtitle).weak());
        }
        ui.separator();
    }

    fn draw_general(&mut self, ui: &mut egui::Ui) {
        Self::section_header(ui, "General", "Application and behavior options.");

        if ui
            .checkbox(
                &mut self.settings.save_meta_images,
                "Log Meta Images",
            )
            .on_hover_text(
                "When enabled, image search / OCR keep debug frames in action logs (in memory). Can be very memory intensive.",
            )
            .changed()
        {
            self.mark_dirty();
        }
        ui.label(
            egui::RichText::new("Warning: can be very memory intensive.")
                .weak()
                .small(),
        );

        if ui
            .checkbox(
                &mut self.settings.highlight_active_action,
                "Highlight the currently executing action",
            )
            .changed()
        {
            self.mark_dirty();
        }

        if ui
            .checkbox(
                &mut self.settings.hide_app_during_recording,
                "Hide Sqyre while recording points and search areas",
            )
            .on_hover_text(
                "When enabled, Sqyre windows are hidden before the desktop snapshot used by recording.",
            )
            .changed()
        {
            self.mark_dirty();
        }

        ui.add_space(6.0);

        ui.horizontal(|ui| {
            ui.label("Image search close-match distance (px):");
            let mut v = self.settings.image_search_close_matches_distance;
            if ui
                .add(egui::DragValue::new(&mut v).range(0..=100).speed(1))
                .on_hover_text("Image search: ignore duplicate matches within this many pixels.")
                .changed()
            {
                self.settings.image_search_close_matches_distance = v;
                self.mark_dirty();
            }
        });
    }

    fn draw_data(
        &mut self,
        ui: &mut egui::Ui,
        _db: &mut Database,
        _macros: &mut Vec<Macro>,
        _catalog: &mut ProgramCatalog,
    ) {
        Self::section_header(ui, "Data", "User data and configuration files.");

        let current = if self.settings.sqyre_dir.trim().is_empty() {
            sqyre_dir()
        } else {
            PathBuf::from(self.settings.sqyre_dir.trim())
        };
        ui.label(current.display().to_string());

        ui.horizontal(|ui| {
            if ui.button("Open .sqyre folder").clicked() {
                match open_sqyre_dir() {
                    Ok(()) => self.set_ok("Opened data folder."),
                    Err(e) => self.set_err(format!("Open folder failed: {e}")),
                }
            }
            if ui.button("Choose location…").clicked() {
                self.choose_sqyre_location();
            }
        });
    }

    fn choose_sqyre_location(&mut self) {
        let start = sqyre_dir()
            .parent()
            .map(PathBuf::from)
            .unwrap_or_else(sqyre_dir);
        let Some(parent) =
            crate::file_dialogs::pick_folder("Choose .sqyre location", &start)
        else {
            return;
        };
        let new_dir = parent.join(".sqyre");
        let old_dir = sqyre_dir();
        if new_dir == old_dir {
            return;
        }
        self.confirm = Some(PendingConfirm::MoveData { old_dir, new_dir });
    }

    fn draw_confirm(
        &mut self,
        ui: &mut egui::Ui,
        confirm: PendingConfirm,
        db: &mut Database,
        macros: &mut Vec<Macro>,
        catalog: &mut ProgramCatalog,
    ) {
        match &confirm {
            PendingConfirm::MoveData { old_dir, new_dir } => {
                ui.label("Move existing data?");
                ui.label(format!(
                    "Move your current data from\n{}\nto\n{}?\n\nChoose No to start fresh at the new location (existing data is left in place).",
                    old_dir.display(),
                    new_dir.display()
                ));
                ui.horizontal(|ui| {
                    if ui.button("Cancel").clicked() {
                        self.confirm = None;
                    }
                    if ui.button("No").clicked() {
                        let old = old_dir.clone();
                        let new = new_dir.clone();
                        self.confirm = None;
                        self.apply_sqyre_location(old, new, false, db, macros, catalog);
                    }
                    if ui.button("Yes").clicked() {
                        let old = old_dir.clone();
                        let new = new_dir.clone();
                        self.confirm = None;
                        self.apply_sqyre_location(old, new, true, db, macros, catalog);
                    }
                });
            }
        }
    }

    fn apply_sqyre_location(
        &mut self,
        old_dir: PathBuf,
        new_dir: PathBuf,
        move_data: bool,
        db: &mut Database,
        macros: &mut Vec<Macro>,
        catalog: &mut ProgramCatalog,
    ) {
        if move_data && old_dir.exists() {
            if let Err(e) = move_dir(&old_dir, &new_dir) {
                self.set_err(format!("Move failed: {e}"));
                return;
            }
        }

        self.settings.sqyre_dir = new_dir.display().to_string();
        set_sqyre_dir_override(Some(new_dir.clone()));
        self.persist();

        match Database::load_default() {
            Ok(loaded) => {
                let mut cat = loaded.program_catalog().unwrap_or_default();
                crate::catalog::apply_main_monitor_resolution(&mut cat);
                let mut list: Vec<_> = loaded.macros.values().cloned().collect();
                list.sort_by(|a, b| a.name.cmp(&b.name));
                *db = loaded;
                *macros = list;
                *catalog = cat;
                self.reload_requested = true;
                self.set_ok(format!("Data location changed to {}.", new_dir.display()));
            }
            Err(e) => {
                // Still switched dirs; surface load error.
                *db = Database::default();
                macros.clear();
                *catalog = ProgramCatalog::default();
                crate::catalog::apply_main_monitor_resolution(catalog);
                self.reload_requested = true;
                self.set_err(format!(
                    "Switched to {} but failed to load db.yaml: {e}",
                    new_dir.display()
                ));
            }
        }
    }

    fn draw_appearance(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        Self::section_header(ui, "Appearance", "Theme and display options.");

        ui.horizontal(|ui| {
            ui.label("Font size:");
            let mut v = self.settings.ui_font_size;
            if ui
                .add(egui::DragValue::new(&mut v).range(10..=28).speed(1))
                .on_hover_text("Base text size for labels, buttons, and form fields.")
                .changed()
            {
                self.settings.ui_font_size = v;
                self.mark_dirty();
                Self::apply_appearance(ctx, &self.settings);
            }
            if ui.small_button("Reset").clicked() {
                self.settings.ui_font_size = DEFAULT_UI_FONT_SIZE;
                self.mark_dirty();
                Self::apply_appearance(ctx, &self.settings);
            }
        });

        ui.horizontal(|ui| {
            ui.label("UI scale:");
            let mut v = self.settings.ui_scale;
            if ui
                .add(
                    egui::DragValue::new(&mut v)
                        .range(0.5..=2.5)
                        .speed(0.05)
                        .fixed_decimals(1),
                )
                .on_hover_text("Scale padding, icons, and other non-text UI elements (1.0 = default).")
                .changed()
            {
                self.settings.ui_scale = v;
                self.mark_dirty();
                Self::apply_appearance(ctx, &self.settings);
            }
            if ui.small_button("Reset").clicked() {
                self.settings.ui_scale = DEFAULT_UI_SCALE;
                self.mark_dirty();
                Self::apply_appearance(ctx, &self.settings);
            }
        });

        ui.add_space(8.0);
        ui.label(egui::RichText::new("Macro tree action colors").strong());

        let is_dark = ui.visuals().dark_mode;
        for &(key, label) in ACTION_COLOR_CATEGORIES {
            ui.horizontal(|ui| {
                ui.label(label);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("Reset").clicked() {
                        self.settings.action_colors.set(key, String::new());
                        clear_custom_action_color(key);
                        self.mark_dirty();
                    }

                    let sample = sample_action_type_for_color_key(key);
                    let current = if self.settings.action_colors.get(key).is_empty() {
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
                        egui::Stroke::new(1.0, ui.visuals().widgets.noninteractive.bg_stroke.color),
                        egui::StrokeKind::Outside,
                    );
                });
            });
        }

        if ui.button("Reset all action colors").clicked() {
            self.settings.action_colors.clear_all();
            clear_all_custom_action_colors();
            self.mark_dirty();
        }
    }
}
