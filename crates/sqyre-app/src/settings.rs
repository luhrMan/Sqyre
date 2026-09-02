//! User Settings window.

use crate::status_banner::StatusBanner;
use eframe::egui::{self, Color32};
use sqyre_domain::Macro;
use sqyre_domain::{format_hex_color, parse_hex_color, ACTION_COLOR_CATEGORIES};
#[cfg(not(target_arch = "wasm32"))]
use sqyre_persist::{
    backups_dir, create_backup, import_backup, list_backups, open_sqyre_dir, prune_backups,
};
use sqyre_persist::{
    move_dir, set_sqyre_dir_override, sqyre_dir, Database, ImportMode, ProgramCatalog,
    UserSettings, DEFAULT_UI_FONT_SIZE, DEFAULT_UI_SCALE,
};
#[cfg(not(target_arch = "wasm32"))]
use sqyre_persist::{
    DEFAULT_BACKUP_INTERVAL_HOURS, DEFAULT_BACKUP_MAX_KEEP, MAX_BACKUP_INTERVAL_HOURS,
    MAX_BACKUP_MAX_KEEP, MIN_BACKUP_INTERVAL_HOURS, MIN_BACKUP_MAX_KEEP,
};
use sqyre_ui_model::{
    action_pastel_color, clear_all_custom_action_colors, clear_custom_action_color,
    default_action_pastel_color, sample_action_type_for_color_key, set_custom_action_color,
};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone)]
enum PendingConfirm {
    /// Move current data to `new_dir` (Yes) or start fresh (No).
    MoveData { old_dir: PathBuf, new_dir: PathBuf },
    /// Replace live data with the contents of a backup archive.
    RestoreBackup { path: PathBuf },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum SettingsSection {
    #[default]
    General,
    Sound,
    #[cfg(all(not(target_arch = "wasm32"), feature = "native-runtime"))]
    Permissions,
    Data,
    #[cfg(not(target_arch = "wasm32"))]
    Updates,
    Appearance,
}

impl SettingsSection {
    fn label(self) -> &'static str {
        match self {
            Self::General => "General",
            Self::Sound => "Sound",
            #[cfg(all(not(target_arch = "wasm32"), feature = "native-runtime"))]
            Self::Permissions => "Permissions",
            Self::Data => "Data",
            #[cfg(not(target_arch = "wasm32"))]
            Self::Updates => "Updates",
            Self::Appearance => "Appearance",
        }
    }

    fn subtitle(self) -> &'static str {
        match self {
            Self::General => "Application and behavior options.",
            Self::Sound => "Cue sounds for macros and UI actions.",
            #[cfg(all(not(target_arch = "wasm32"), feature = "native-runtime"))]
            Self::Permissions => {
                "Desktop access for capture, recording, hotkeys, and macro playback."
            }
            Self::Data => {
                "Data folder location and zip backups of macros, settings, images, and variables."
            }
            #[cfg(not(target_arch = "wasm32"))]
            Self::Updates => "Check GitHub Releases for a newer Sqyre build.",
            Self::Appearance => "Theme and display options.",
        }
    }

    fn visible(self, q: &str) -> bool {
        match self {
            Self::General => section_visible(q, SECTION_GENERAL, GENERAL_SETTINGS),
            Self::Sound => section_visible(q, SECTION_SOUND, SOUND_SETTINGS),
            #[cfg(all(not(target_arch = "wasm32"), feature = "native-runtime"))]
            Self::Permissions => section_visible(q, SECTION_PERMISSIONS, PERMISSIONS_SETTINGS),
            Self::Data => section_visible(q, SECTION_DATA, DATA_SETTINGS),
            #[cfg(not(target_arch = "wasm32"))]
            Self::Updates => section_visible(q, SECTION_UPDATES, UPDATES_SETTINGS),
            Self::Appearance => appearance_section_visible(q),
        }
    }

    fn all() -> impl Iterator<Item = Self> {
        [
            Self::General,
            Self::Sound,
            #[cfg(all(not(target_arch = "wasm32"), feature = "native-runtime"))]
            Self::Permissions,
            Self::Data,
            #[cfg(not(target_arch = "wasm32"))]
            Self::Updates,
            Self::Appearance,
        ]
        .into_iter()
    }
}

#[derive(Default)]
pub struct SettingsUi {
    pub open: bool,
    settings: UserSettings,
    dirty: bool,
    active_section: SettingsSection,
    /// Filter query for the settings list (case-insensitive fuzzy match).
    search: String,
    status_banner: StatusBanner,
    confirm: Option<PendingConfirm>,
    /// Set when the data directory changed and the shell should reload from disk.
    pub reload_requested: bool,
    /// Set when the user asks to restart after applying an update.
    #[cfg(not(target_arch = "wasm32"))]
    pub restart_requested: bool,
    #[cfg(all(not(target_arch = "wasm32"), feature = "native-runtime"))]
    permissions: crate::permissions_panel::PermissionsPanel,
}

impl SettingsUi {
    pub fn from_settings(settings: UserSettings) -> Self {
        Self {
            settings,
            ..Self::default()
        }
    }

    /// Open Settings on the Appearance section (docs / screenshot harnesses).
    pub fn open_appearance_for_docs(&mut self) {
        self.open = true;
        self.active_section = SettingsSection::Appearance;
        self.search.clear();
    }

    pub fn settings(&self) -> &UserSettings {
        &self.settings
    }

    pub fn settings_mut(&mut self) -> &mut UserSettings {
        &mut self.settings
    }

    pub fn save_settings(&mut self) -> Result<(), String> {
        self.settings.save_default().map_err(|e| e.to_string())
    }

    /// Ensure Hack is in the proportional fallback chain so geometric/arrow
    /// symbols (e.g. ➔ ◫) are available — egui's default omits Hack there.
    /// Also registers Phosphor for overlay button icons.
    pub fn install_fonts(ctx: &egui::Context) {
        let mut fonts = egui::FontDefinitions::default();
        egui_phosphor::add_to_fonts(&mut fonts, egui_phosphor::Variant::Regular);
        crate::overlay_icons::register_phosphor_family(&mut fonts);
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
        style
            .text_styles
            .insert(TextStyle::Button, egui::FontId::proportional(base));
        style.text_styles.insert(
            TextStyle::Heading,
            egui::FontId::proportional((base * 1.35).round()),
        );
        style
            .text_styles
            .insert(TextStyle::Monospace, egui::FontId::monospace(base));
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
        self.status_banner.set_ok(msg);
    }

    fn set_err(&mut self, msg: impl Into<String>) {
        self.status_banner.set_err(msg);
    }

    /// Surface an update-related success message in the settings status line.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn set_update_status_ok(&mut self, msg: impl Into<String>) {
        self.set_ok(msg);
    }

    pub fn show(
        &mut self,
        ctx: &egui::Context,
        db: &mut Database,
        macros: &mut Vec<Macro>,
        catalog: &mut ProgramCatalog,
        #[cfg(not(target_arch = "wasm32"))] update: &mut crate::update::UpdateManager,
    ) {
        if !self.open {
            return;
        }
        let mut open = self.open;
        // Bounds only — settings owns sidebar/content ScrollAreas; outer window
        // scroll produced phantom H bars when content claimed available_width.
        crate::widgets::fit_dialog_popup(
            egui::Window::new("User Settings")
                .open(&mut open)
                .default_size([680.0, 640.0])
                .min_size([520.0, 360.0])
                .resizable(true),
            ctx,
        )
        .show(ctx, |ui| {
            #[cfg(not(target_arch = "wasm32"))]
            self.ui(ui, ctx, db, macros, catalog, update);
            #[cfg(target_arch = "wasm32")]
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
        #[cfg(not(target_arch = "wasm32"))] update: &mut crate::update::UpdateManager,
    ) {
        if let Some(confirm) = self.confirm.clone() {
            self.draw_confirm(ui, confirm, db, macros, catalog);
            return;
        }

        ui.horizontal(|ui| {
            ui.label(egui::RichText::new(
                egui_phosphor::regular::MAGNIFYING_GLASS,
            ))
            .on_hover_text("Search");
            let search_w = (ui.available_width() - 60.0).max(120.0);
            if ui
                .add(
                    egui::TextEdit::singleline(&mut self.search)
                        .hint_text("Search settings…")
                        .desired_width(search_w),
                )
                .changed()
                && !self.search.trim().is_empty()
            {
                let q = self.search.trim().to_ascii_lowercase();
                if !self.active_section.visible(&q) {
                    if let Some(section) = SettingsSection::all().find(|s| s.visible(&q)) {
                        self.active_section = section;
                    }
                }
            }
            if !self.search.is_empty() && ui.small_button("Clear").clicked() {
                self.search.clear();
            }
        });
        ui.separator();

        let footer = if self.status_banner.status.is_some() {
            40.0
        } else {
            0.0
        };
        let body_h = (ui.available_height() - footer).max(40.0);
        let q = self.search.trim().to_ascii_lowercase();
        let visible_sections: Vec<SettingsSection> =
            SettingsSection::all().filter(|s| s.visible(&q)).collect();
        if !visible_sections.is_empty() && !visible_sections.contains(&self.active_section) {
            self.active_section = visible_sections[0];
        }

        ui.horizontal(|ui| {
            const SIDEBAR_W: f32 = 132.0;
            const MIN_CONTENT_W: f32 = 240.0;
            ui.allocate_ui_with_layout(
                egui::vec2(SIDEBAR_W, body_h),
                egui::Layout::top_down(egui::Align::Min),
                |ui| {
                    ui.set_max_size(egui::vec2(SIDEBAR_W, body_h));
                    for section in visible_sections.iter().copied() {
                        if ui
                            .selectable_label(self.active_section == section, section.label())
                            .clicked()
                        {
                            self.active_section = section;
                        }
                    }
                },
            );
            ui.separator();
            let content_w = ui.available_width().max(MIN_CONTENT_W);
            ui.allocate_ui_with_layout(
                egui::vec2(content_w, body_h),
                egui::Layout::top_down(egui::Align::Min),
                |ui| {
                    ui.set_max_size(egui::vec2(content_w, body_h));
                    crate::pickers::scroll_vertical()
                        .id_salt("user_settings_content")
                        .auto_shrink([false, false])
                        .max_height(body_h)
                        .show(ui, |ui| {
                            ui.set_max_width(ui.available_width());
                            if visible_sections.is_empty() {
                                ui.label(
                                    egui::RichText::new("No settings match your search.")
                                        .weak()
                                        .italics(),
                                );
                                return;
                            }
                            let section = self.active_section;
                            ui.label(egui::RichText::new(section.label()).strong().heading());
                            ui.label(egui::RichText::new(section.subtitle()).weak());
                            ui.separator();
                            let section_hit = match section {
                                SettingsSection::General => query_matches(&q, SECTION_GENERAL),
                                SettingsSection::Sound => query_matches(&q, SECTION_SOUND),
                                #[cfg(all(
                                    not(target_arch = "wasm32"),
                                    feature = "native-runtime"
                                ))]
                                SettingsSection::Permissions => {
                                    query_matches(&q, SECTION_PERMISSIONS)
                                }
                                SettingsSection::Data => query_matches(&q, SECTION_DATA),
                                #[cfg(not(target_arch = "wasm32"))]
                                SettingsSection::Updates => query_matches(&q, SECTION_UPDATES),
                                SettingsSection::Appearance => {
                                    query_matches(&q, SECTION_APPEARANCE)
                                }
                            };
                            match section {
                                SettingsSection::General => self.draw_general(ui, &q, section_hit),
                                SettingsSection::Sound => self.draw_sound(ui, &q, section_hit),
                                #[cfg(all(
                                    not(target_arch = "wasm32"),
                                    feature = "native-runtime"
                                ))]
                                SettingsSection::Permissions => {
                                    self.draw_permissions(ui, ctx, &q, section_hit)
                                }
                                SettingsSection::Data => {
                                    self.draw_data(ui, db, macros, catalog, &q, section_hit);
                                    #[cfg(not(target_arch = "wasm32"))]
                                    if setting_visible(&q, section_hit, DATA_LOCATION)
                                        && setting_visible(&q, section_hit, DATA_BACKUP)
                                    {
                                        ui.add_space(10.0);
                                    }
                                    self.draw_backup(ui, db, macros, catalog, &q, section_hit);
                                }
                                #[cfg(not(target_arch = "wasm32"))]
                                SettingsSection::Updates => {
                                    self.draw_updates(ui, update, &q, section_hit)
                                }
                                SettingsSection::Appearance => {
                                    self.draw_appearance(ui, ctx, &q, section_hit)
                                }
                            }
                        });
                },
            );
        });

        if self.status_banner.status.is_some() {
            ui.separator();
            self.status_banner.paint(ui);
        }
    }

    fn draw_general(&mut self, ui: &mut egui::Ui, q: &str, section_hit: bool) {
        if setting_visible(q, section_hit, SETTING_LOG_META) {
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
        }

        if setting_visible(q, section_hit, SETTING_HIGHLIGHT_ACTION)
            && ui
                .checkbox(
                    &mut self.settings.highlight_active_action,
                    "Highlight the currently executing action",
                )
                .on_hover_text("Scroll and tint the tree row of the action running now.")
                .changed()
        {
            self.mark_dirty();
        }

        if setting_visible(q, section_hit, SETTING_HIDE_WHILE_RECORDING)
            && ui
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

        if setting_visible(q, section_hit, SETTING_RELEASE_HELD)
            && ui
                .checkbox(
                    &mut self.settings.release_held_inputs_on_end,
                    "Release held keys and buttons when a macro ends",
                )
                .on_hover_text(
                    "When enabled, any key or mouse button still held from Down/hold actions is released when the macro finishes, stops, or errors.",
                )
                .changed()
        {
            self.mark_dirty();
        }

        if setting_visible(q, section_hit, SETTING_WHILE_BUDGET) {
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                ui.label("While safety budget (iterations):");
                let mut v = self.settings.while_max_iterations;
                if ui
                    .add(
                        egui::DragValue::new(&mut v)
                            .range(
                                sqyre_persist::MIN_WHILE_MAX_ITERATIONS
                                    ..=sqyre_persist::MAX_WHILE_MAX_ITERATIONS,
                            )
                            .speed(1000),
                    )
                    .on_hover_text(
                        "Used when a While action has max_iterations ≤ 0. Prevents runaway loops.",
                    )
                    .changed()
                {
                    self.settings.while_max_iterations = v;
                    self.mark_dirty();
                }
            });
        }

        if setting_visible(q, section_hit, SETTING_RUN_MACRO_DEPTH) {
            ui.horizontal(|ui| {
                ui.label("Run Macro max nesting depth:");
                let mut v = self.settings.run_macro_max_depth;
                if ui
                    .add(
                        egui::DragValue::new(&mut v)
                            .range(
                                sqyre_persist::MIN_RUN_MACRO_MAX_DEPTH
                                    ..=sqyre_persist::MAX_RUN_MACRO_MAX_DEPTH,
                            )
                            .speed(1),
                    )
                    .on_hover_text(
                        "Maximum nested Run Macro calls (including the top-level macro). Cycles are always rejected.",
                    )
                    .changed()
                {
                    self.settings.run_macro_max_depth = v;
                    self.mark_dirty();
                }
            });
        }

        if setting_visible(q, section_hit, SETTING_IMAGE_SEARCH_DISTANCE) {
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                ui.label("Image search close-match distance (px):");
                let mut v = self.settings.image_search_close_matches_distance;
                if ui
                    .add(egui::DragValue::new(&mut v).range(0..=100).speed(1))
                    .on_hover_text(
                        "Image search: ignore duplicate matches within this many pixels.",
                    )
                    .changed()
                {
                    self.settings.image_search_close_matches_distance = v;
                    self.mark_dirty();
                }
            });
        }
    }

    #[cfg(all(not(target_arch = "wasm32"), feature = "native-runtime"))]
    fn draw_permissions(
        &mut self,
        ui: &mut egui::Ui,
        ctx: &egui::Context,
        q: &str,
        section_hit: bool,
    ) {
        if !setting_visible(q, section_hit, PERMISSIONS_PANEL) {
            return;
        }
        self.permissions.paint(ui, ctx);
    }

    fn draw_sound(&mut self, ui: &mut egui::Ui, q: &str, section_hit: bool) {
        if setting_visible(q, section_hit, SETTING_FINISH_SOUND)
            && ui
                .checkbox(
                    &mut self.settings.play_finish_sound,
                    "Play a sound when a macro finishes",
                )
                .on_hover_text(
                    "Plays a short cue when a top-level macro run completes successfully.",
                )
                .changed()
        {
            self.mark_dirty();
        }

        if setting_visible(q, section_hit, SETTING_UI_SOUNDS)
            && ui
                .checkbox(
                    &mut self.settings.play_ui_sounds,
                    "Play sounds when adding or deleting",
                )
                .on_hover_text(
                    "Plays short cues when macros, actions, or data-editor entities are added or deleted.",
                )
                .changed()
        {
            self.mark_dirty();
        }

        if setting_visible(q, section_hit, SETTING_SOUND_VOLUME) {
            ui.horizontal(|ui| {
                ui.label("Sound volume:");
                let mut pct = (self.settings.sound_volume * 100.0).round() as i32;
                if ui
                    .add(egui::Slider::new(&mut pct, 0..=100).suffix("%"))
                    .on_hover_text("Volume for finish and add/delete cue sounds.")
                    .changed()
                {
                    self.settings.sound_volume = pct as f32 / 100.0;
                    self.mark_dirty();
                }
            });
        }
    }

    fn draw_data(
        &mut self,
        ui: &mut egui::Ui,
        _db: &mut Database,
        _macros: &mut Vec<Macro>,
        _catalog: &mut ProgramCatalog,
        q: &str,
        section_hit: bool,
    ) {
        if !setting_visible(q, section_hit, DATA_LOCATION) {
            return;
        }
        #[cfg(target_arch = "wasm32")]
        {
            ui.label(
                "Browser editor: macros live in memory. Use Import / Export on the toolbar for db.yaml. Full backups are not available.",
            );
            return;
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
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
    }

    fn draw_backup(
        &mut self,
        ui: &mut egui::Ui,
        db: &mut Database,
        macros: &mut Vec<Macro>,
        catalog: &mut ProgramCatalog,
        q: &str,
        section_hit: bool,
    ) {
        if !setting_visible(q, section_hit, DATA_BACKUP) {
            return;
        }
        #[cfg(target_arch = "wasm32")]
        {
            let _ = (ui, db, macros, catalog);
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            let _ = (db, macros, catalog);

            if ui
                .checkbox(
                    &mut self.settings.backup_enabled,
                    "Automatic backups",
                )
                .on_hover_text(
                    "Periodically zip the data directory into the backups folder when Sqyre is running.",
                )
                .changed()
            {
                self.mark_dirty();
            }

            ui.horizontal(|ui| {
                ui.label("Interval (hours):");
                let mut v = self.settings.backup_interval_hours;
                if ui
                    .add(
                        egui::DragValue::new(&mut v)
                            .range(MIN_BACKUP_INTERVAL_HOURS..=MAX_BACKUP_INTERVAL_HOURS)
                            .speed(1),
                    )
                    .on_hover_text("Hours between automatic backups while Sqyre is open.")
                    .changed()
                {
                    self.settings.backup_interval_hours = v;
                    self.mark_dirty();
                }
                if ui.small_button("Reset").clicked() {
                    self.settings.backup_interval_hours = DEFAULT_BACKUP_INTERVAL_HOURS;
                    self.mark_dirty();
                }
            });

            ui.horizontal(|ui| {
                ui.label("Keep at most:");
                let mut v = self.settings.backup_max_keep;
                if ui
                    .add(
                        egui::DragValue::new(&mut v)
                            .range(MIN_BACKUP_MAX_KEEP..=MAX_BACKUP_MAX_KEEP)
                            .speed(1),
                    )
                    .on_hover_text(
                        "Oldest managed sqyre-backup-*.zip files are deleted beyond this count.",
                    )
                    .changed()
                {
                    self.settings.backup_max_keep = v;
                    self.mark_dirty();
                }
                if ui.small_button("Reset").clicked() {
                    self.settings.backup_max_keep = DEFAULT_BACKUP_MAX_KEEP;
                    self.mark_dirty();
                }
            });

            ui.add_space(6.0);
            ui.label(
                egui::RichText::new(format!("Backups folder: {}", backups_dir().display()))
                    .weak()
                    .small(),
            );
            ui.label(
                egui::RichText::new(format_last_backup(self.settings.last_backup_unix))
                    .weak()
                    .small(),
            );
            if let Ok(list) = list_backups() {
                if let Some(latest) = list.first() {
                    if let Some(name) = latest.file_name().and_then(|n| n.to_str()) {
                        ui.label(
                            egui::RichText::new(format!("Latest archive: {name}"))
                                .weak()
                                .small(),
                        );
                    }
                }
            }

            ui.add_space(6.0);
            ui.horizontal(|ui| {
                if ui.button("Back up now").clicked() {
                    self.run_manual_backup();
                }
                if ui.button("Restore from backup…").clicked() {
                    self.choose_restore_backup();
                }
            });
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn run_manual_backup(&mut self) {
        match create_backup() {
            Ok(path) => {
                let keep = self.settings.backup_max_keep.max(1) as usize;
                if let Err(e) = prune_backups(keep) {
                    self.set_err(format!("Backup created but prune failed: {e}"));
                }
                self.note_backup_success(&path);
            }
            Err(e) => self.set_err(format!("Backup failed: {e}")),
        }
    }

    /// Record a successful backup timestamp and persist settings.
    pub fn note_backup_success(&mut self, path: &std::path::Path) {
        self.settings.last_backup_unix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);
        self.persist();
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("backup");
        self.set_ok(format!("Backup saved: {name}"));
    }

    /// Pick a backup zip and open Settings on the restore confirm dialog.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn request_restore_backup(&mut self) {
        self.choose_restore_backup();
        if self.confirm.is_some() {
            self.open = true;
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn choose_restore_backup(&mut self) {
        let start = backups_dir();
        let start = if start.exists() { start } else { sqyre_dir() };
        let Some(path) = crate::file_dialogs::pick_zip("Restore from backup", &start) else {
            return;
        };
        self.confirm = Some(PendingConfirm::RestoreBackup { path });
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn draw_updates(
        &mut self,
        ui: &mut egui::Ui,
        update: &mut crate::update::UpdateManager,
        q: &str,
        section_hit: bool,
    ) {
        use crate::update::{UpdateState, SQYRE_VERSION};

        if setting_visible(q, section_hit, SETTING_UPDATE_VERSION) {
            ui.label(format!("Current version: {SQYRE_VERSION}"));
        }

        if setting_visible(q, section_hit, SETTING_AUTO_UPDATE)
            && ui
                .checkbox(
                    &mut self.settings.auto_update_check,
                    "Check for updates on startup",
                )
                .on_hover_text(
                    "Queries GitHub Releases for a newer Sqyre build when the app starts.",
                )
                .changed()
        {
            self.mark_dirty();
        }

        if setting_visible(q, section_hit, SETTING_UPDATE_ACTIONS) {
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                let busy = update.is_busy();
                if ui
                    .add_enabled(!busy, egui::Button::new("Check now"))
                    .on_hover_text("Query GitHub Releases for a newer build.")
                    .clicked()
                {
                    update.start_check();
                    ui.ctx().request_repaint();
                }
                match &update.state {
                    UpdateState::Available { .. } => {
                        if ui
                            .add_enabled(!busy, egui::Button::new("Download & install"))
                            .clicked()
                        {
                            update.start_download();
                        }
                    }
                    UpdateState::Ready { .. } => {
                        if ui.button("Restart to finish").clicked() {
                            self.restart_requested = true;
                        }
                    }
                    _ => {}
                }
            });

            ui.add_space(4.0);
            let status = match &update.state {
                UpdateState::Idle => "Update status: idle".to_string(),
                UpdateState::Unavailable { reason } => reason.clone(),
                UpdateState::Checking => "Checking for updates…".to_string(),
                UpdateState::UpToDate => "You are up to date.".to_string(),
                UpdateState::Available { version, .. } => {
                    format!("Update available: v{version}")
                }
                UpdateState::Downloading { version } => {
                    format!("Downloading v{version}…")
                }
                UpdateState::Ready { version } => {
                    format!("v{version} installed. Restart to finish.")
                }
                UpdateState::Failed { message } => format!("Update check failed: {message}"),
            };
            ui.label(egui::RichText::new(status).weak().small());
        }
    }

    fn choose_sqyre_location(&mut self) {
        let start = sqyre_dir()
            .parent()
            .map(PathBuf::from)
            .unwrap_or_else(sqyre_dir);
        let Some(parent) = crate::file_dialogs::pick_folder("Choose .sqyre location", &start)
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
            PendingConfirm::RestoreBackup { path } => {
                ui.label("Import backup?");
                ui.label(format!(
                    "Archive:\n{}\n\nOverwrite replaces all macros, settings, images, and variables with the archive.\n\nMerge keeps live-only items, prefers the archive on name conflicts, replaces settings, and merges other assets. Automatic backups in the backups folder are not removed.",
                    path.display()
                ));
                ui.horizontal(|ui| {
                    if ui.button("Cancel").clicked() {
                        self.confirm = None;
                    }
                    if ui.button("Overwrite").clicked() {
                        let path = path.clone();
                        self.confirm = None;
                        self.apply_restore_backup(path, ImportMode::Overwrite, db, macros, catalog);
                    }
                    if ui.button("Merge").clicked() {
                        let path = path.clone();
                        self.confirm = None;
                        self.apply_restore_backup(path, ImportMode::Merge, db, macros, catalog);
                    }
                });
            }
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn apply_restore_backup(
        &mut self,
        path: PathBuf,
        mode: ImportMode,
        db: &mut Database,
        macros: &mut Vec<Macro>,
        catalog: &mut ProgramCatalog,
    ) {
        if let Err(e) = import_backup(&path, mode) {
            self.set_err(format!("Import failed: {e}"));
            return;
        }

        // Prefer restored settings.yaml; keep current if load fails.
        match UserSettings::load_default() {
            Ok(loaded) => {
                loaded.apply_sqyre_dir_override();
                Self::apply_action_colors(&loaded);
                self.settings = loaded;
            }
            Err(e) => {
                crate::log::warn(format_args!(
                    "import succeeded but settings reload failed: {e}"
                ));
            }
        }

        match Database::load_default() {
            Ok(mut loaded) => {
                let mut cat = Arc::unwrap_or_clone(loaded.program_catalog().unwrap_or_default());
                let _ = crate::catalog::prepare_catalog(&mut cat, &mut loaded);
                let mut list: Vec<_> = loaded.macros.values().cloned().collect();
                list.sort_by(|a, b| a.name.cmp(&b.name));
                *db = loaded;
                *macros = list;
                *catalog = cat;
                self.reload_requested = true;
                let name = path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("backup");
                let mode_label = match mode {
                    ImportMode::Overwrite => "Overwrote from",
                    ImportMode::Merge => "Merged from",
                };
                self.set_ok(format!("{mode_label} {name}."));
            }
            Err(e) => {
                *db = Database::default();
                macros.clear();
                *catalog = ProgramCatalog::default();
                crate::catalog::apply_main_monitor_resolution(catalog);
                let _ = crate::catalog::ensure_general_program_seeded(catalog);
                self.reload_requested = true;
                self.set_err(format!("Imported archive but failed to load db.yaml: {e}"));
            }
        }
    }

    #[cfg(target_arch = "wasm32")]
    fn apply_restore_backup(
        &mut self,
        _path: PathBuf,
        _mode: ImportMode,
        _db: &mut Database,
        _macros: &mut Vec<Macro>,
        _catalog: &mut ProgramCatalog,
    ) {
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
            Ok(mut loaded) => {
                let mut cat = Arc::unwrap_or_clone(loaded.program_catalog().unwrap_or_default());
                let _ = crate::catalog::prepare_catalog(&mut cat, &mut loaded);
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
                let _ = crate::catalog::ensure_general_program_seeded(catalog);
                self.reload_requested = true;
                self.set_err(format!(
                    "Switched to {} but failed to load db.yaml: {e}",
                    new_dir.display()
                ));
            }
        }
    }

    fn draw_appearance(
        &mut self,
        ui: &mut egui::Ui,
        ctx: &egui::Context,
        q: &str,
        section_hit: bool,
    ) {
        if setting_visible(q, section_hit, SETTING_COMPACT_HEADERS)
            && ui
                .checkbox(
                    &mut self.settings.compact_program_headers,
                    "Compact program headers with icons",
                )
                .on_hover_text(
                    "When a program has a process icon, list headers show only the icon with the child count on the right (name on hover).",
                )
                .changed()
        {
            self.mark_dirty();
        }

        if setting_visible(q, section_hit, SETTING_FONT_SIZE) {
            ui.add_space(6.0);
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
        }

        if setting_visible(q, section_hit, SETTING_UI_SCALE) {
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
                    .on_hover_text(
                        "Scale padding, icons, and other non-text UI elements (1.0 = default).",
                    )
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
        }

        let show_colors = setting_visible(q, section_hit, SETTING_ACTION_COLORS)
            || ACTION_COLOR_CATEGORIES
                .iter()
                .any(|&(_, label)| setting_visible(q, false, &[label]));
        if show_colors {
            ui.add_space(8.0);
            ui.label(egui::RichText::new("Macro tree action colors").strong());

            let is_dark = ui.visuals().dark_mode;
            for &(key, label) in ACTION_COLOR_CATEGORIES {
                if !setting_visible(q, section_hit, SETTING_ACTION_COLORS)
                    && !setting_visible(q, false, &[label])
                {
                    continue;
                }
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
                            self.settings.action_colors.set(key, format_hex_color(rgba));
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
                && ui.button("Reset all action colors").clicked()
            {
                self.settings.action_colors.clear_all();
                clear_all_custom_action_colors();
                self.mark_dirty();
            }
        }
    }
}

fn format_last_backup(unix: i64) -> String {
    if unix <= 0 {
        return "Last backup: never".into();
    }
    let Ok(now) = SystemTime::now().duration_since(UNIX_EPOCH) else {
        return format!("Last backup: unix {unix}");
    };
    let age_secs = now.as_secs().saturating_sub(unix as u64);
    let label = if age_secs < 60 {
        format!("{age_secs}s ago")
    } else if age_secs < 3600 {
        format!("{}m ago", age_secs / 60)
    } else if age_secs < 86_400 {
        format!("{}h ago", age_secs / 3600)
    } else {
        format!("{}d ago", age_secs / 86_400)
    };
    format!("Last backup: {label}")
}

// --- Settings search keywords ------------------------------------------------

const SECTION_GENERAL: &[&str] = &["general", "application", "behavior"];
const SECTION_SOUND: &[&str] = &["sound", "audio", "cue"];
#[cfg(all(not(target_arch = "wasm32"), feature = "native-runtime"))]
const SECTION_PERMISSIONS: &[&str] = &[
    "permissions",
    "permission",
    "screen recording",
    "input group",
    "bazzite",
    "atomic",
    "hotkeys",
    "portal",
    "wayland",
    "access",
];
const SECTION_DATA: &[&str] = &["data", "folder", "backup", "restore", "archive"];
#[cfg(not(target_arch = "wasm32"))]
const SECTION_UPDATES: &[&str] = &["updates", "github", "release", "version"];
const SECTION_APPEARANCE: &[&str] = &["appearance", "theme", "display"];

const SETTING_LOG_META: &[&str] = &[
    "log meta images",
    "meta images",
    "ocr",
    "debug frames",
    "memory",
];
const SETTING_HIGHLIGHT_ACTION: &[&str] =
    &["highlight", "executing action", "active action", "tint"];
const SETTING_HIDE_WHILE_RECORDING: &[&str] =
    &["hide", "recording", "points", "search areas", "snapshot"];
const SETTING_RELEASE_HELD: &[&str] = &[
    "release",
    "held keys",
    "held buttons",
    "inputs",
    "macro ends",
];
const SETTING_WHILE_BUDGET: &[&str] = &[
    "while",
    "safety budget",
    "iterations",
    "loop",
    "max_iterations",
];
const SETTING_RUN_MACRO_DEPTH: &[&str] = &["run macro", "nesting", "depth", "recursion"];
const SETTING_IMAGE_SEARCH_DISTANCE: &[&str] = &[
    "image search",
    "close-match",
    "close match",
    "distance",
    "duplicate",
];

const SETTING_FINISH_SOUND: &[&str] = &["finish sound", "macro finishes", "complete"];
const SETTING_UI_SOUNDS: &[&str] = &["ui sounds", "adding", "deleting", "add/delete"];
const SETTING_SOUND_VOLUME: &[&str] = &["volume", "sound volume"];

#[cfg(all(not(target_arch = "wasm32"), feature = "native-runtime"))]
const PERMISSIONS_PANEL: &[&str] = &[
    "permissions",
    "screen recording",
    "input device",
    "input group",
    "global shortcuts",
    "hotkeys",
    "automation",
    "usermod",
    "portal",
    "wayland",
    "evdev",
];

const DATA_LOCATION: &[&str] = &[
    "data folder",
    "data directory",
    "sqyre",
    ".sqyre",
    "location",
    "choose location",
    "open folder",
];
const DATA_BACKUP: &[&str] = &[
    "backup",
    "automatic backups",
    "interval",
    "keep at most",
    "restore",
    "archive",
    "zip",
];

#[cfg(not(target_arch = "wasm32"))]
const SETTING_UPDATE_VERSION: &[&str] = &["current version", "version"];
#[cfg(not(target_arch = "wasm32"))]
const SETTING_AUTO_UPDATE: &[&str] = &["check for updates", "startup", "auto update", "automatic"];
#[cfg(not(target_arch = "wasm32"))]
const SETTING_UPDATE_ACTIONS: &[&str] = &[
    "check now",
    "download",
    "install",
    "restart",
    "update status",
];

const SETTING_COMPACT_HEADERS: &[&str] = &["compact", "program headers", "icons", "headers"];
const SETTING_FONT_SIZE: &[&str] = &["font size", "font", "text size"];
const SETTING_UI_SCALE: &[&str] = &["ui scale", "scale", "padding"];
const SETTING_ACTION_COLORS: &[&str] =
    &["action colors", "macro tree", "colors", "colour", "pastel"];

const GENERAL_SETTINGS: &[&[&str]] = &[
    SETTING_LOG_META,
    SETTING_HIGHLIGHT_ACTION,
    SETTING_HIDE_WHILE_RECORDING,
    SETTING_RELEASE_HELD,
    SETTING_WHILE_BUDGET,
    SETTING_RUN_MACRO_DEPTH,
    SETTING_IMAGE_SEARCH_DISTANCE,
];
const SOUND_SETTINGS: &[&[&str]] = &[
    SETTING_FINISH_SOUND,
    SETTING_UI_SOUNDS,
    SETTING_SOUND_VOLUME,
];
#[cfg(all(not(target_arch = "wasm32"), feature = "native-runtime"))]
const PERMISSIONS_SETTINGS: &[&[&str]] = &[PERMISSIONS_PANEL];
const DATA_SETTINGS: &[&[&str]] = &[DATA_LOCATION, DATA_BACKUP];
#[cfg(not(target_arch = "wasm32"))]
const UPDATES_SETTINGS: &[&[&str]] = &[
    SETTING_UPDATE_VERSION,
    SETTING_AUTO_UPDATE,
    SETTING_UPDATE_ACTIONS,
];
const APPEARANCE_SETTINGS: &[&[&str]] = &[
    SETTING_COMPACT_HEADERS,
    SETTING_FONT_SIZE,
    SETTING_UI_SCALE,
    SETTING_ACTION_COLORS,
];

fn query_matches(q: &str, keywords: &[&str]) -> bool {
    keywords
        .iter()
        .any(|k| crate::pickers::fuzzy_match_fold(q, k))
}

fn setting_visible(q: &str, section_hit: bool, keywords: &[&str]) -> bool {
    q.is_empty() || section_hit || query_matches(q, keywords)
}

fn section_visible(q: &str, section_keywords: &[&str], settings: &[&[&str]]) -> bool {
    q.is_empty()
        || query_matches(q, section_keywords)
        || settings.iter().any(|kw| query_matches(q, kw))
}

fn appearance_section_visible(q: &str) -> bool {
    section_visible(q, SECTION_APPEARANCE, APPEARANCE_SETTINGS)
        || ACTION_COLOR_CATEGORIES
            .iter()
            .any(|&(_, label)| crate::pickers::fuzzy_match_fold(q, label))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_query_shows_every_section() {
        assert!(section_visible("", SECTION_GENERAL, GENERAL_SETTINGS));
        assert!(section_visible("", SECTION_SOUND, SOUND_SETTINGS));
        assert!(appearance_section_visible(""));
    }

    #[test]
    fn volume_matches_sound_not_general() {
        assert!(section_visible("volume", SECTION_SOUND, SOUND_SETTINGS));
        assert!(!section_visible(
            "volume",
            SECTION_GENERAL,
            GENERAL_SETTINGS
        ));
        assert!(setting_visible("volume", false, SETTING_SOUND_VOLUME));
        assert!(!setting_visible("volume", false, SETTING_FINISH_SOUND));
    }

    #[test]
    fn detection_matches_appearance_color_row() {
        assert!(appearance_section_visible("detection"));
        assert!(setting_visible("detection", false, &["Detection"]));
        assert!(!setting_visible("detection", false, SETTING_FONT_SIZE));
    }

    #[test]
    fn section_title_reveals_all_settings_in_section() {
        assert!(setting_visible("general", true, SETTING_LOG_META));
        assert!(setting_visible("general", true, SETTING_WHILE_BUDGET));
    }
}
