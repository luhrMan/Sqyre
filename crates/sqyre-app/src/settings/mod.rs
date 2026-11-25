//! User Settings window.

use crate::file_dialogs::PickPurpose;
use crate::help_text as ht;
use crate::status_banner::StatusBanner;
use eframe::egui::{self, Color32};
use sqyre_domain::Macro;
use sqyre_domain::{parse_hex_color, ACTION_COLOR_CATEGORIES};

use sqyre_persist::{
    backups_dir, create_backup, import_backup, list_backups, open_sqyre_dir, prune_backups,
};
use sqyre_persist::{
    move_dir, set_sqyre_dir_override, sqyre_dir, Database, HotkeyWhileRunning, ImportMode,
    ProgramCatalog, UiDensity, UiFontFamily, UserSettings, DEFAULT_UI_FONT_SIZE, DEFAULT_UI_SCALE,
    MAX_UI_SCALE, MIN_UI_SCALE,
};

use sqyre_persist::{
    DEFAULT_BACKUP_INTERVAL_HOURS, DEFAULT_BACKUP_MAX_KEEP, MAX_BACKUP_INTERVAL_HOURS,
    MAX_BACKUP_MAX_KEEP, MIN_BACKUP_INTERVAL_HOURS, MIN_BACKUP_MAX_KEEP,
};
use sqyre_ui_model::{clear_all_custom_action_colors, set_custom_action_color};
use std::path::PathBuf;
use std::sync::Arc;

mod appearance;
mod common;
mod data;
mod general;
mod macros;
#[cfg(feature = "native-runtime")]
mod permissions;
mod sound;

mod updates;

use common::*;

fn settings_window_id() -> egui::Id {
    egui::Id::new("User Settings")
}

/// Borrowed workspace data the Settings window edits (backups, restores,
/// data-dir moves, overlay buttons).
pub struct SettingsData<'a> {
    pub db: &'a mut Database,
    pub macros: &'a mut Vec<Macro>,
    pub catalog: &'a mut ProgramCatalog,
}

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
    Macros,
    Sound,
    #[cfg(feature = "native-runtime")]
    Permissions,
    Data,

    Updates,
    Appearance,
}

impl SettingsSection {
    fn label(self) -> &'static str {
        match self {
            Self::General => "General",
            Self::Macros => "Macros",
            Self::Sound => "Sound",
            #[cfg(feature = "native-runtime")]
            Self::Permissions => "Permissions",
            Self::Data => "Data",

            Self::Updates => "Updates",
            Self::Appearance => "Appearance",
        }
    }

    fn visible(self, q: &str) -> bool {
        match self {
            Self::General => section_visible(q, SECTION_GENERAL, GENERAL_SETTINGS),
            Self::Macros => section_visible(q, SECTION_MACROS, MACROS_SETTINGS),
            Self::Sound => section_visible(q, SECTION_SOUND, SOUND_SETTINGS),
            #[cfg(feature = "native-runtime")]
            Self::Permissions => section_visible(q, SECTION_PERMISSIONS, PERMISSIONS_SETTINGS),
            Self::Data => section_visible(q, SECTION_DATA, DATA_SETTINGS),

            Self::Updates => section_visible(q, SECTION_UPDATES, UPDATES_SETTINGS),
            Self::Appearance => appearance_section_visible(q),
        }
    }

    fn is_permissions(self) -> bool {
        #[cfg(feature = "native-runtime")]
        {
            self == Self::Permissions
        }
        #[cfg(not(feature = "native-runtime"))]
        {
            false
        }
    }

    fn all() -> impl Iterator<Item = Self> {
        [
            Self::General,
            Self::Macros,
            Self::Sound,
            #[cfg(feature = "native-runtime")]
            Self::Permissions,
            Self::Data,
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
    ui_scale_apply_pending: bool,
    active_section: SettingsSection,
    /// Filter query for the settings list (case-insensitive fuzzy match).
    search: String,
    status_banner: StatusBanner,
    confirm: Option<PendingConfirm>,
    /// Set when the data directory changed and the shell should reload from disk.
    pub reload_requested: bool,
    /// Set when the user asks to restart after applying an update.
    pub restart_requested: bool,
    /// Single-key capture for the panic/stop key (global hotkeys are native-only).
    panic_record: crate::key_record::KeyRecordUi,
    #[cfg(feature = "native-runtime")]
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

    pub fn section_labels_for_docs() -> Vec<&'static str> {
        SettingsSection::all().map(SettingsSection::label).collect()
    }

    pub fn open_section_for_docs(&mut self, label: &str) -> bool {
        let Some(section) = SettingsSection::all().find(|section| section.label() == label) else {
            return false;
        };
        self.open = true;
        self.active_section = section;
        self.search.clear();
        true
    }

    /// Seed the move-data confirm without a folder picker (docs / kittest).
    pub fn seed_move_data_confirm_for_docs(&mut self, old_dir: PathBuf, new_dir: PathBuf) {
        self.open = true;
        self.confirm = Some(PendingConfirm::MoveData { old_dir, new_dir });
    }

    /// Seed the restore-backup confirm without a file picker (docs / kittest).
    pub fn seed_restore_backup_confirm_for_docs(&mut self, path: PathBuf) {
        self.open = true;
        self.confirm = Some(PendingConfirm::RestoreBackup { path });
    }

    /// Whether a settings confirm is waiting for Yes/No or Overwrite/Merge.
    pub fn docs_settings_confirm_pending(&self) -> bool {
        self.confirm.is_some()
    }

    /// Settings status line, if one is showing (docs / kittest).
    pub fn docs_settings_status(&self) -> Option<&str> {
        self.status_banner.status.as_deref()
    }

    /// Open the settings window (re-arms search focus).
    pub fn request_open(&mut self, ctx: &egui::Context) {
        self.open = true;
        crate::widgets::reset_focus_search(ctx, settings_window_id());
    }

    pub fn settings(&self) -> &UserSettings {
        &self.settings
    }

    pub fn settings_mut(&mut self) -> &mut UserSettings {
        &mut self.settings
    }

    /// Background permission checks behind the missing-permission badge.
    #[cfg(feature = "native-runtime")]
    pub fn tick_permissions(&mut self, ctx: &egui::Context) {
        self.permissions.tick(ctx);
    }

    /// Docs / kittest: skip the background permission probe so harness frames settle.
    #[cfg(feature = "native-runtime")]
    pub fn suppress_permission_probe_for_docs(&mut self) {
        self.permissions.suppress_for_docs();
    }

    /// At least one permission in Settings → Permissions needs the user's action.
    pub fn permissions_missing(&self) -> bool {
        #[cfg(feature = "native-runtime")]
        {
            self.permissions.has_missing()
        }
        #[cfg(not(feature = "native-runtime"))]
        {
            false
        }
    }

    pub fn save_settings(&mut self) -> Result<(), String> {
        self.settings.save_default().map_err(|e| e.to_string())
    }

    /// Ensure Hack is in the proportional fallback chain so geometric/arrow
    /// symbols (e.g. ➔ ◫) are available — egui's default omits Hack there.
    /// Also registers Phosphor for overlay button icons.
    ///
    /// `UiFontFamily::Monospace` moves Hack to the front of the proportional
    /// family so all UI text renders mono; `Standard` keeps it as a fallback.
    /// Rebuilt from `FontDefinitions::default()` every call, so switching the
    /// family just re-runs this.
    pub fn install_fonts(ctx: &egui::Context, settings: &UserSettings) {
        let mut fonts = egui::FontDefinitions::default();
        egui_phosphor::add_to_fonts(&mut fonts, egui_phosphor::Variant::Regular);
        // Fill variant as a named family so outline vs filled icons can coexist
        // (same codepoints; Proportional keeps Regular).
        egui_phosphor::add_font_bytes_as_family(
            &mut fonts,
            crate::widgets::PHOSPHOR_FILL_FAMILY,
            egui_phosphor::Variant::Fill.font_bytes(),
        );
        crate::overlay_icons::register_phosphor_family(&mut fonts);
        if let Some(prop) = fonts.families.get_mut(&egui::FontFamily::Proportional) {
            if !prop.iter().any(|n| n == "Hack") {
                // After Ubuntu (UI text), before emoji fallbacks; first overall
                // when the whole UI renders monospace.
                let insert_at =
                    if settings.ui.ui_font_family == sqyre_persist::UiFontFamily::Monospace {
                        0
                    } else {
                        prop.iter()
                            .position(|n| n == "Ubuntu-Light")
                            .map(|i| i + 1)
                            .unwrap_or(0)
                    };
                prop.insert(insert_at, "Hack".to_owned());
            }
        }
        ctx.set_fonts(fonts);
    }

    /// Apply appearance prefs to the egui context (Sqyre theme, fonts, scale).
    pub fn apply_appearance(ctx: &egui::Context, settings: &UserSettings) {
        let scale = if settings.ui.ui_scale > 0.0 {
            settings.ui.ui_scale
        } else {
            DEFAULT_UI_SCALE
        };
        // Ctrl+`+`/`-`/`0` edit `ui_scale` via `handle_zoom_keys`; egui's
        // built-in keyboard zoom would bypass the persisted setting.
        ctx.options_mut(|o| o.zoom_with_keyboard = false);
        // Multiplies the OS scale (monitor DPI, Android density, browser pixel ratio).
        ctx.set_zoom_factor(scale);

        // Accent feeds `theme::apply` visuals and every `theme::accent()` draw.
        sqyre_ui_theme::set_accent(
            sqyre_domain::parse_hex_color(settings.ui.accent_color.trim())
                .map(|[r, g, b, _]| [r, g, b]),
        );
        #[cfg(feature = "native-runtime")]
        {
            // `Some(true)` when the toggle is on; `None` keeps `SQYRE_DIAG` honored.
            sqyre_ports::set_disk_logging(settings.runtime.diagnostic_logging.then_some(true));
            sqyre_ports::set_pipewire_suspend_on_idle(
                settings.runtime.suspend_screen_capture_when_idle,
            );
            crate::mem_diag::set_enabled(
                settings.runtime.memory_logging,
                settings.runtime.memory_logging_interval_ms,
            );
            #[cfg(target_os = "linux")]
            crate::desktop_entry::set_autostart(settings.data.launch_on_login);
        }
        Self::install_fonts(ctx, settings);
        crate::theme::apply(ctx);

        let mut style = (*ctx.global_style()).clone();
        let base = settings.ui.ui_font_size.max(10) as f32;
        let mono_base = if settings.ui.mono_font_size > 0 {
            settings.ui.mono_font_size.max(8) as f32
        } else {
            base
        };
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
        // Heading also sizes the window title bar (egui allocates its buttons
        // at the Heading row height), so a modest multiplier keeps dialogs slim.
        style.text_styles.insert(
            TextStyle::Heading,
            egui::FontId::proportional((base * 1.15).round()),
        );
        style
            .text_styles
            .insert(TextStyle::Monospace, egui::FontId::monospace(mono_base));
        // Thin opaque floating bars: always visible when content overflows
        // (including horizontal), instead of fully dormant until hover.
        style.spacing.scroll = crate::theme::scroll_style();
        // egui's 10pt corner grab radius puts a 20×20 hit rect on top of any
        // widget near a dialog's bottom-right corner — shrink it so widgets
        // there keep their clicks. The grip stays paint/interact only and
        // takes no layout space.
        style.interaction.resize_grab_radius_corner = crate::theme::WINDOW_RESIZE_GRAB_RADIUS;
        ctx.set_global_style(style);
        Self::apply_spacing(ctx, settings);
    }

    /// Browser-style Ctrl+`+`/`=`/`-`/`0`: edits and persists `ui_scale`
    /// (egui's own `zoom_with_keyboard` is off — it would bypass the setting).
    /// `persist` (not `mark_dirty`) so the change lands even with the
    /// Settings window closed.
    pub fn handle_zoom_keys(&mut self, ctx: &egui::Context) {
        use egui::gui_zoom::kb_shortcuts;
        let mut scale = self.settings.ui.ui_scale;
        if !ctx.input_mut(|i| {
            if i.consume_shortcut(&kb_shortcuts::ZOOM_RESET) {
                scale = DEFAULT_UI_SCALE;
            } else {
                let up = i.consume_shortcut(&kb_shortcuts::ZOOM_IN)
                    || i.consume_shortcut(&kb_shortcuts::ZOOM_IN_SECONDARY);
                let down = i.consume_shortcut(&kb_shortcuts::ZOOM_OUT);
                match (up, down) {
                    (true, false) => scale += 0.1,
                    (false, true) => scale -= 0.1,
                    _ => return false,
                }
            }
            true
        }) {
            return;
        }
        let scale = ((scale * 10.0).round() / 10.0).clamp(MIN_UI_SCALE, MAX_UI_SCALE);
        if scale == self.settings.ui.ui_scale {
            return;
        }
        self.settings.ui.ui_scale = scale;
        self.persist();
        Self::apply_appearance(ctx, &self.settings);
    }

    /// Viewport width (points) at or below which auto-compact kicks in.
    const AUTO_COMPACT_WIDTH: f32 = 480.0;

    /// Density for the current viewport: the chosen one, or Compact when the
    /// window is narrow and `auto_compact_density` is on. Gap tokens stay in
    /// points — they do not scale with resolution.
    fn effective_density(ctx: &egui::Context, settings: &UserSettings) -> UiDensity {
        Self::density_for(
            settings.ui.ui_density,
            settings.ui.auto_compact_density,
            ctx.content_rect().width(),
        )
    }

    /// Density after the narrow-viewport auto-compact rule.
    fn density_for(density: UiDensity, auto_compact: bool, viewport_width: f32) -> UiDensity {
        if auto_compact && viewport_width <= Self::AUTO_COMPACT_WIDTH {
            UiDensity::Compact
        } else {
            density
        }
    }

    fn effective_gaps(ctx: &egui::Context, settings: &UserSettings) -> crate::theme::DensityGaps {
        let d = Self::effective_density(ctx, settings);
        crate::theme::DensityGaps::new(d.section_gap(), d.element_gap())
    }

    /// Push the effective gaps into the egui `Spacing` (one place, instead of
    /// per-row `spacing_mut` overrides) and into [`crate::theme::set_density`].
    ///
    /// Only *gaps* follow density — point sizes (hit areas, icon buttons, card
    /// dimensions) stay fixed, with `interact_size` kept at egui's defaults so
    /// tree/list rows don't inflate. `text_edit_width` narrows under Compact
    /// so single-line fields fit phone-width panes instead of clipping.
    fn apply_spacing(ctx: &egui::Context, settings: &UserSettings) {
        let d = Self::effective_density(ctx, settings);
        let gaps = crate::theme::DensityGaps::new(d.section_gap(), d.element_gap());
        let mut style = (*ctx.global_style()).clone();
        // `item_spacing` IS the element gap: widgets sit `element` points
        // apart with no per-row overrides. `add_section_gap` tops sections up.
        style.spacing.item_spacing = egui::vec2(gaps.element, gaps.element);
        style.spacing.text_edit_width = if d == UiDensity::Compact {
            200.0
        } else {
            280.0
        };
        ctx.set_global_style(style);
        crate::theme::set_density(ctx, gaps);
    }

    /// Keep the applied gaps in sync with the viewport size — cheap no-op until
    /// the window crosses [`Self::AUTO_COMPACT_WIDTH`]. Runs each frame.
    pub fn sync_density(ctx: &egui::Context, settings: &UserSettings) {
        if crate::theme::density(ctx) != Self::effective_gaps(ctx, settings) {
            Self::apply_spacing(ctx, settings);
        }
    }

    /// Load action-color overrides from settings into the domain map.
    pub fn apply_action_colors(settings: &UserSettings) {
        clear_all_custom_action_colors();
        for &(key, _) in ACTION_COLOR_CATEGORIES {
            let hex = settings.ui.action_colors.get(key);
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
            self.set_err(format!(
                "Failed to save settings: {}",
                crate::status_banner::user_error_detail(&e)
            ));
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
    pub fn set_update_status_ok(&mut self, msg: impl Into<String>) {
        self.set_ok(msg);
    }

    pub fn show(
        &mut self,
        ctx: &egui::Context,
        data: &mut SettingsData<'_>,
        update: &mut crate::update::UpdateManager,
        macro_hotkeys: &sqyre_hotkeys::MacroHotkeyBridge,
        pending_scale: Option<&crate::widgets::ViewportScaleEvent>,
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
                .min_size([crate::widgets::FLOATER_MIN_EDITOR[0], 360.0])
                .resizable(true),
            ctx,
            settings_window_id(),
            pending_scale,
        )
        .show(ctx, |ui| {
            crate::widgets::fill_resize_body(ui, |ui| {
                self.ui(ui, ctx, data, update, macro_hotkeys);
            });
        });
        // `ui` may clear `self.open` (Esc); Window `.open` may clear `open` (titlebar).
        self.open = open && self.open;
        if !self.open {
            crate::widgets::reset_focus_search(ctx, settings_window_id());
        }
        if let Some(confirm) = self.confirm.clone() {
            self.draw_confirm(ctx, confirm, pending_scale, data);
        }
        if let Some(key) = self.panic_record.show(ctx, macro_hotkeys, pending_scale) {
            self.settings.runtime.panic_key = key;
            self.mark_dirty();
        }
        if self.dirty {
            self.persist();
            if !self.ui_scale_apply_pending {
                Self::apply_appearance(ctx, &self.settings);
            }
        }
    }

    fn ui(
        &mut self,
        ui: &mut egui::Ui,
        ctx: &egui::Context,
        data: &mut SettingsData<'_>,
        update: &mut crate::update::UpdateManager,
        macro_hotkeys: &sqyre_hotkeys::MacroHotkeyBridge,
    ) {
        // Esc: clear search first, then close (window already has titlebar close).
        if crate::widgets::consume_escape(ui) {
            if !self.search.is_empty() {
                self.search.clear();
            } else {
                self.open = false;
            }
        }

        let search_focused = crate::widgets::show_overflow(ui, "settings_search", 1.0, |ui| {
            // Pinned id so focus survives the clear eraser appearing and disappearing.
            let out = crate::widgets::search_field(
                ui,
                &mut self.search,
                crate::widgets::SearchFieldOpts {
                    hint: "Search settings…",
                    focus_id: Some(settings_window_id()),
                    clearable: true,
                    stable_id: Some(settings_window_id().with("search")),
                    trailing: None,
                },
            );
            if out.changed && !self.search.trim().is_empty() {
                let q = self.search.trim().to_ascii_lowercase();
                if !self.active_section.visible(&q) {
                    if let Some(section) = SettingsSection::all().find(|s| s.visible(&q)) {
                        self.active_section = section;
                    }
                }
            }
            out.response.has_focus()
        });

        let footer = if self.status_banner.status.is_some() {
            40.0
        } else {
            0.0
        };
        let rem = ui.available_size();
        let (outer, _) = ui.allocate_exact_size(rem, egui::Sense::hover());
        let body_h = (outer.height() - footer).max(40.0);
        let body_rect = egui::Rect::from_min_size(outer.min, egui::vec2(outer.width(), body_h));
        let footer_rect =
            egui::Rect::from_min_max(egui::pos2(outer.min.x, outer.min.y + body_h), outer.max);

        let q = self.search.trim().to_ascii_lowercase();
        let visible_sections: Vec<SettingsSection> =
            SettingsSection::all().filter(|s| s.visible(&q)).collect();
        if !visible_sections.is_empty() && !visible_sections.contains(&self.active_section) {
            self.active_section = visible_sections[0];
        }

        // ↑↓ move the sidebar selection (like the command palette). Prefer when
        // search is focused; otherwise only while this is the top surface and
        // no widget inside it owns arrow keys.
        let allow_section_keys = search_focused
            || (crate::focus_nav::is_top_layer(ui.ctx(), ui.layer_id())
                && !crate::focus_nav::focused_in_layer(ui.ctx(), ui.layer_id()));
        if allow_section_keys && !visible_sections.is_empty() {
            let mut idx = visible_sections
                .iter()
                .position(|s| *s == self.active_section)
                .unwrap_or(0);
            let down = ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowDown));
            let up = ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowUp));
            if down {
                idx = (idx + 1).min(visible_sections.len() - 1);
                self.active_section = visible_sections[idx];
            } else if up {
                idx = idx.saturating_sub(1);
                self.active_section = visible_sections[idx];
            }
        }

        let natural_side = sidebar_width(ui);
        let side_cap = body_rect.width() * 0.45;
        let side_w = if side_cap < natural_side {
            side_cap.max(1.0)
        } else {
            natural_side
        };
        let crate::widgets::SplitView {
            left: mut left_ui,
            right: mut right_ui,
            splitter,
        } = crate::widgets::split_view(ui, body_rect, side_w);

        let permissions_missing = self.permissions_missing();
        crate::widgets::show_overflow(&mut left_ui, "settings_sections", 1.0, |ui| {
            for section in visible_sections.iter().copied() {
                let warn = permissions_missing && section.is_permissions();
                let response = ui.selectable_label(self.active_section == section, section.label());
                if warn {
                    crate::widgets::warn_badge(ui, response.rect);
                }
                let tip = if warn {
                    "A permission is missing. ↑↓ to switch sections"
                } else {
                    "↑↓ to switch sections"
                };
                if response.on_hover_text(tip).clicked() {
                    self.active_section = section;
                }
            }
        });
        ui.painter().vline(
            splitter.center().x,
            splitter.y_range(),
            crate::theme::panel_split_stroke(),
        );
        {
            let content_w = right_ui.max_rect().width();
            crate::widgets::show_dialog_scroll_mod(
                &mut right_ui,
                content_w,
                body_h,
                |s| s.id_salt("user_settings_content"),
                |ui| {
                    // Soft-wrap to the painted clip (the reserved scrollbar
                    // lane insets it past `max_rect`); both-axis scroll covers
                    // hard min-width overflows.
                    crate::widgets::clamp_row_right_to_clip(ui);
                    if visible_sections.is_empty() {
                        crate::widgets::list_vacancy(ui, &q, 0, "settings");
                        return;
                    }
                    let section = self.active_section;
                    // Strong title only — sidebar already names the section; no
                    // Body subtitle band under the heading.
                    ui.label(egui::RichText::new(section.label()).strong());
                    crate::widgets::section_separator(ui);
                    let section_hit = match section {
                        SettingsSection::General => query_matches(&q, SECTION_GENERAL),
                        SettingsSection::Macros => query_matches(&q, SECTION_MACROS),
                        SettingsSection::Sound => query_matches(&q, SECTION_SOUND),
                        #[cfg(feature = "native-runtime")]
                        SettingsSection::Permissions => query_matches(&q, SECTION_PERMISSIONS),
                        SettingsSection::Data => query_matches(&q, SECTION_DATA),

                        SettingsSection::Updates => query_matches(&q, SECTION_UPDATES),
                        SettingsSection::Appearance => query_matches(&q, SECTION_APPEARANCE),
                    };
                    match section {
                        SettingsSection::General => self.draw_general(ui, &q, section_hit),
                        SettingsSection::Macros => {
                            self.draw_macros(ui, &q, section_hit, macro_hotkeys)
                        }
                        SettingsSection::Sound => self.draw_sound(ui, &q, section_hit),
                        #[cfg(feature = "native-runtime")]
                        SettingsSection::Permissions => {
                            self.draw_permissions(ui, ctx, &q, section_hit)
                        }
                        SettingsSection::Data => {
                            self.draw_data(ui, data.db, data.macros, data.catalog, &q, section_hit);

                            if setting_visible(&q, section_hit, DATA_LOCATION)
                                && setting_visible(&q, section_hit, DATA_BACKUP)
                            {
                                crate::theme::add_section_gap(ui);
                            }
                            self.draw_backup(
                                ui,
                                data.db,
                                data.macros,
                                data.catalog,
                                &q,
                                section_hit,
                            );
                        }

                        SettingsSection::Updates => self.draw_updates(ui, update, &q, section_hit),
                        SettingsSection::Appearance => {
                            self.draw_appearance(ui, ctx, &q, section_hit)
                        }
                    }
                },
            );
        }

        if footer > 0.0 {
            let mut footer_ui = ui.new_child(
                egui::UiBuilder::new()
                    .max_rect(footer_rect)
                    .layout(egui::Layout::top_down(egui::Align::Min)),
            );
            footer_ui.set_clip_rect(footer_rect.intersect(ui.clip_rect()));
            self.status_banner.paint(&mut footer_ui);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui_kittest::kittest::Queryable;
    use sqyre_persist::{Database, ProgramCatalog};

    #[test]
    fn empty_query_shows_every_section() {
        assert!(section_visible("", SECTION_GENERAL, GENERAL_SETTINGS));
        assert!(section_visible("", SECTION_MACROS, MACROS_SETTINGS));
        assert!(section_visible("", SECTION_SOUND, SOUND_SETTINGS));
        assert!(appearance_section_visible(""));
    }

    #[test]
    fn narrow_viewport_compacts_only_when_enabled() {
        use UiDensity::*;
        assert_eq!(SettingsUi::density_for(Spaced, true, 360.0), Compact);
        assert_eq!(SettingsUi::density_for(Spaced, false, 360.0), Spaced);
        assert_eq!(SettingsUi::density_for(Spaced, true, 960.0), Spaced);
        // Compact is idempotent — already-compact users see no change.
        assert_eq!(SettingsUi::density_for(Compact, true, 360.0), Compact);
    }

    #[test]
    fn resolve_sqyre_dir_accepts_dot_sqyre_or_parent() {
        let direct = PathBuf::from("/home/me/.sqyre");
        assert_eq!(resolve_sqyre_dir_from_pick(direct.clone()), direct);
        assert_eq!(
            resolve_sqyre_dir_from_pick(PathBuf::from("/home/me")),
            PathBuf::from("/home/me/.sqyre")
        );
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
    fn borderless_matches_os_title_bar_setting() {
        assert!(appearance_section_visible("borderless"));
        assert!(setting_visible(
            "borderless",
            false,
            SETTING_WINDOW_DECORATIONS
        ));
        assert!(setting_visible(
            "title bar",
            false,
            SETTING_WINDOW_DECORATIONS
        ));
    }

    #[test]
    fn section_title_reveals_all_settings_in_section() {
        assert!(setting_visible("general", true, SETTING_LOG_META));
        assert!(setting_visible("macros", true, SETTING_WHILE_BUDGET));
    }

    #[test]
    fn image_search_matches_macros_not_general() {
        assert!(section_visible(
            "image search",
            SECTION_MACROS,
            MACROS_SETTINGS
        ));
        assert!(!section_visible(
            "image search",
            SECTION_GENERAL,
            GENERAL_SETTINGS
        ));
    }

    fn press_key(harness: &mut egui_kittest::Harness<'_, SettingsUi>, key: egui::Key) {
        for pressed in [true, false] {
            harness.event(egui::Event::Key {
                key,
                physical_key: None,
                pressed,
                repeat: false,
                modifiers: egui::Modifiers::NONE,
            });
        }
    }

    fn search_field<'a>(
        harness: &'a egui_kittest::Harness<'_, SettingsUi>,
    ) -> egui_kittest::Node<'a> {
        harness.get_by_role(egui::accesskit::Role::TextInput)
    }

    /// The Clear button is inserted ahead of the field when the query crosses
    /// empty. That must not move the field's id, or the first typed or deleted
    /// character drops focus.
    #[test]
    fn search_keeps_focus_when_query_crosses_empty() {
        let mut harness = egui_kittest::Harness::builder()
            .with_size(egui::vec2(900.0, 700.0))
            .build_ui_state(
                |ui, settings: &mut SettingsUi| {
                    let mut db = Database::default();
                    let mut macros = Vec::new();
                    let mut catalog = ProgramCatalog::default();

                    let mut update = crate::update::UpdateManager::default();
                    let macro_hotkeys = sqyre_hotkeys::MacroHotkeyBridge::new();
                    settings.show(
                        ui.ctx(),
                        &mut SettingsData {
                            db: &mut db,
                            macros: &mut macros,
                            catalog: &mut catalog,
                        },
                        &mut update,
                        &macro_hotkeys,
                        None,
                    );
                },
                SettingsUi {
                    open: true,
                    ..SettingsUi::default()
                },
            );
        harness.run();
        assert!(
            search_field(&harness).is_focused(),
            "search focuses when Settings opens"
        );

        search_field(&harness).type_text("q");
        harness.run();
        harness.step();
        let field = search_field(&harness);
        assert_eq!(field.value().as_deref(), Some("q"));
        assert!(
            field.is_focused(),
            "typing the first character should keep the search field focused"
        );
        assert!(harness.query_by_label("Clear search").is_some());

        search_field(&harness).type_text("z");
        harness.run();
        harness.step();
        let field = search_field(&harness);
        assert_eq!(field.value().as_deref(), Some("qz"));
        assert!(field.is_focused());

        press_key(&mut harness, egui::Key::Backspace);
        harness.run();
        harness.step();
        let field = search_field(&harness);
        assert_eq!(field.value().as_deref(), Some("q"));
        assert!(field.is_focused());

        press_key(&mut harness, egui::Key::Backspace);
        harness.run();
        harness.step();
        let field = search_field(&harness);
        assert_eq!(field.value().as_deref(), Some(""));
        assert!(
            field.is_focused(),
            "deleting back to an empty query should keep the search field focused"
        );
        assert!(harness.query_by_label("Clear search").is_none());

        search_field(&harness).type_text("a");
        harness.run();
        harness.step();
        let field = search_field(&harness);
        assert_eq!(field.value().as_deref(), Some("a"));
        assert!(field.is_focused());
    }

    #[test]
    fn subsection_title_reveals_its_settings() {
        assert!(query_matches("playback", SUBSECTION_PLAYBACK));
        assert!(setting_visible("playback", true, SETTING_RUN_MACRO_DEPTH));
        assert!(!setting_visible(
            "playback",
            false,
            SETTING_VARIANT_EXIT_EARLY
        ));
    }
}
