//! User Settings window.

use crate::file_dialogs::PickPurpose;
use crate::help_text as ht;
use crate::status_banner::StatusBanner;
use eframe::egui::{self, Color32};
use sqyre_domain::Macro;
use sqyre_domain::{format_hex_color, parse_hex_color, ACTION_COLOR_CATEGORIES};
#[cfg(not(target_arch = "wasm32"))]
use sqyre_persist::{
    backups_dir, create_backup, import_backup, list_backups, open_sqyre_dir, prune_backups,
};
use sqyre_persist::{
    move_dir, set_sqyre_dir_override, sqyre_dir, Database, HotkeyWhileRunning, ImportMode,
    ProgramCatalog, UiDensity, UiFontFamily, UserSettings, DEFAULT_UI_FONT_SIZE, DEFAULT_UI_SCALE,
    MAX_UI_SCALE, MIN_UI_SCALE,
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
            Self::Macros => "Macros",
            Self::Sound => "Sound",
            #[cfg(all(not(target_arch = "wasm32"), feature = "native-runtime"))]
            Self::Permissions => "Permissions",
            Self::Data => "Data",
            #[cfg(not(target_arch = "wasm32"))]
            Self::Updates => "Updates",
            Self::Appearance => "Appearance",
        }
    }

    fn visible(self, q: &str) -> bool {
        match self {
            Self::General => section_visible(q, SECTION_GENERAL, GENERAL_SETTINGS),
            Self::Macros => section_visible(q, SECTION_MACROS, MACROS_SETTINGS),
            Self::Sound => section_visible(q, SECTION_SOUND, SOUND_SETTINGS),
            #[cfg(all(not(target_arch = "wasm32"), feature = "native-runtime"))]
            Self::Permissions => section_visible(q, SECTION_PERMISSIONS, PERMISSIONS_SETTINGS),
            Self::Data => section_visible(q, SECTION_DATA, DATA_SETTINGS),
            #[cfg(not(target_arch = "wasm32"))]
            Self::Updates => section_visible(q, SECTION_UPDATES, UPDATES_SETTINGS),
            Self::Appearance => appearance_section_visible(q),
        }
    }

    fn is_permissions(self) -> bool {
        #[cfg(all(not(target_arch = "wasm32"), feature = "native-runtime"))]
        {
            self == Self::Permissions
        }
        #[cfg(not(all(not(target_arch = "wasm32"), feature = "native-runtime")))]
        {
            false
        }
    }

    fn all() -> impl Iterator<Item = Self> {
        [
            Self::General,
            Self::Macros,
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
    ui_scale_apply_pending: bool,
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
    /// Single-key capture for the panic/stop key (global hotkeys are native-only).
    panic_record: crate::key_record::KeyRecordUi,
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
    #[cfg(all(not(target_arch = "wasm32"), feature = "native-runtime"))]
    pub fn tick_permissions(&mut self, ctx: &egui::Context) {
        self.permissions.tick(ctx);
    }

    /// Docs / kittest: skip the background permission probe so harness frames settle.
    #[cfg(all(not(target_arch = "wasm32"), feature = "native-runtime"))]
    pub fn suppress_permission_probe_for_docs(&mut self) {
        self.permissions.suppress_for_docs();
    }

    /// At least one permission in Settings → Permissions needs the user's action.
    pub fn permissions_missing(&self) -> bool {
        #[cfg(all(not(target_arch = "wasm32"), feature = "native-runtime"))]
        {
            self.permissions.has_missing()
        }
        #[cfg(not(all(not(target_arch = "wasm32"), feature = "native-runtime")))]
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
                let insert_at = if settings.ui_font_family == sqyre_persist::UiFontFamily::Monospace
                {
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
        let scale = if settings.ui_scale > 0.0 {
            settings.ui_scale
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
            sqyre_domain::parse_hex_color(settings.accent_color.trim())
                .map(|[r, g, b, _]| [r, g, b]),
        );
        #[cfg(all(feature = "native-runtime", not(target_arch = "wasm32")))]
        {
            // `Some(true)` when the toggle is on; `None` keeps `SQYRE_DIAG` honored.
            sqyre_capture::set_disk_logging(settings.diagnostic_logging.then_some(true));
            sqyre_capture::set_pipewire_suspend_on_idle(settings.suspend_screen_capture_when_idle);
            crate::mem_diag::set_enabled(
                settings.memory_logging,
                settings.memory_logging_interval_ms,
            );
            #[cfg(target_os = "linux")]
            crate::desktop_entry::set_autostart(settings.launch_on_login);
        }
        Self::install_fonts(ctx, settings);
        crate::theme::apply(ctx);

        let mut style = (*ctx.global_style()).clone();
        let base = settings.ui_font_size.max(10) as f32;
        let mono_base = if settings.mono_font_size > 0 {
            settings.mono_font_size.max(8) as f32
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
        let mut scale = self.settings.ui_scale;
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
        if scale == self.settings.ui_scale {
            return;
        }
        self.settings.ui_scale = scale;
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
            settings.ui_density,
            settings.auto_compact_density,
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
    #[cfg(not(target_arch = "wasm32"))]
    pub fn set_update_status_ok(&mut self, msg: impl Into<String>) {
        self.set_ok(msg);
    }

    pub fn show(
        &mut self,
        ctx: &egui::Context,
        data: &mut SettingsData<'_>,
        #[cfg(not(target_arch = "wasm32"))] update: &mut crate::update::UpdateManager,
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
                #[cfg(not(target_arch = "wasm32"))]
                self.ui(ui, ctx, data, update, macro_hotkeys);
                #[cfg(target_arch = "wasm32")]
                self.ui(ui, ctx, data, macro_hotkeys);
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
            self.settings.panic_key = key;
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
        #[cfg(not(target_arch = "wasm32"))] update: &mut crate::update::UpdateManager,
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
                        #[cfg(all(not(target_arch = "wasm32"), feature = "native-runtime"))]
                        SettingsSection::Permissions => query_matches(&q, SECTION_PERMISSIONS),
                        SettingsSection::Data => query_matches(&q, SECTION_DATA),
                        #[cfg(not(target_arch = "wasm32"))]
                        SettingsSection::Updates => query_matches(&q, SECTION_UPDATES),
                        SettingsSection::Appearance => query_matches(&q, SECTION_APPEARANCE),
                    };
                    match section {
                        SettingsSection::General => self.draw_general(ui, &q, section_hit),
                        SettingsSection::Macros => {
                            self.draw_macros(ui, &q, section_hit, macro_hotkeys)
                        }
                        SettingsSection::Sound => self.draw_sound(ui, &q, section_hit),
                        #[cfg(all(not(target_arch = "wasm32"), feature = "native-runtime"))]
                        SettingsSection::Permissions => {
                            self.draw_permissions(ui, ctx, &q, section_hit)
                        }
                        SettingsSection::Data => {
                            self.draw_data(ui, data.db, data.macros, data.catalog, &q, section_hit);
                            #[cfg(not(target_arch = "wasm32"))]
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
                        #[cfg(not(target_arch = "wasm32"))]
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

    fn draw_general(&mut self, ui: &mut egui::Ui, q: &str, section_hit: bool) {
        if setting_visible(q, section_hit, SETTING_LOG_META) {
            ui.horizontal(|ui| {
                if crate::action_tooltip::help::tip(
                    ui.checkbox(&mut self.settings.save_meta_images, "Log Meta Images"),
                    crate::action_tooltip::help::SETTING_LOG_META,
                )
                .changed()
                {
                    self.mark_dirty();
                }
                ui.label("Keep at most:");
                let mut v = self.settings.action_log_max_entries;
                if ui
                    .add(
                        crate::widgets::num_entry(&mut v)
                            .range(
                                sqyre_persist::MIN_ACTION_LOG_MAX_ENTRIES
                                    ..=sqyre_persist::MAX_ACTION_LOG_MAX_ENTRIES,
                            )
                            .speed(100)
                            .suffix(" entries"),
                    )
                    .on_hover_text(ht::SET_LOG_CAP)
                    .changed()
                {
                    self.settings.action_log_max_entries = v;
                    self.mark_dirty();
                }
            });
        }

        #[cfg(all(not(target_arch = "wasm32"), feature = "native-runtime"))]
        if setting_visible(q, section_hit, SETTING_WORKER_THREADS) {
            ui.horizontal(|ui| {
                ui.label("CPU threads for image work:");
                let max = sqyre_persist::available_worker_threads();
                let mut v = self.settings.worker_threads;
                if ui
                    .add(
                        crate::widgets::num_entry(&mut v)
                            .range(sqyre_persist::MIN_WORKER_THREADS..=max)
                            .speed(1),
                    )
                    .on_hover_text(format!(
                        "How many CPU threads Sqyre may use for image search, find pixel, and capture conversion (1–{max} on this machine). Default is {} (16 or fewer). Restart Sqyre after changing.",
                        sqyre_persist::default_worker_threads()
                    ))
                    .changed()
                {
                    self.settings.worker_threads = v;
                    self.mark_dirty();
                }
                ui.label(format!("of {max}"));
            });
            ui.label(
                egui::RichText::new("Restart Sqyre for a new thread count to take effect.")
                    .small()
                    .weak(),
            );
        }

        if setting_visible(q, section_hit, SETTING_LAST_MACRO)
            && dirty_checkbox(
                ui,
                &mut self.settings.restore_last_macro,
                "Re-select the last used macro on launch",
                ht::SET_LAST_MACRO,
            )
        {
            self.mark_dirty();
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            if setting_visible(q, section_hit, SETTING_CLOSE_TO_TRAY)
                && dirty_checkbox(
                    ui,
                    &mut self.settings.close_to_tray,
                    "Closing the window minimizes to the tray",
                    ht::SET_CLOSE_TO_TRAY,
                )
            {
                self.mark_dirty();
            }

            if setting_visible(q, section_hit, SETTING_START_MINIMIZED)
                && dirty_checkbox(
                    ui,
                    &mut self.settings.start_minimized,
                    "Start in the tray",
                    ht::SET_START_MINIMIZED,
                )
            {
                self.mark_dirty();
            }

            #[cfg(target_os = "linux")]
            if setting_visible(q, section_hit, SETTING_AUTOSTART)
                && dirty_checkbox(
                    ui,
                    &mut self.settings.launch_on_login,
                    "Launch on login",
                    ht::SET_AUTOSTART,
                )
            {
                self.mark_dirty();
            }

            #[cfg(all(target_os = "linux", feature = "native-runtime"))]
            if setting_visible(q, section_hit, SETTING_SUSPEND_CAPTURE)
                && dirty_checkbox(
                    ui,
                    &mut self.settings.suspend_screen_capture_when_idle,
                    "Sleep screen capture when idle",
                    ht::SET_SUSPEND_CAPTURE,
                )
            {
                self.mark_dirty();
            }
        }

        #[cfg(all(not(target_arch = "wasm32"), feature = "native-runtime"))]
        {
            let show_diag = setting_visible(q, section_hit, SETTING_DIAGNOSTICS);
            let show_mem = setting_visible(q, section_hit, SETTING_MEMORY_LOG);
            if show_diag || show_mem {
                ui.label(egui::RichText::new("Diagnostics").strong());
            }
            if show_diag
                && dirty_checkbox(
                    ui,
                    &mut self.settings.diagnostic_logging,
                    "Write diagnostic log (diag.log)",
                    ht::SET_DIAG_LOG,
                )
            {
                self.mark_dirty();
            }
            if show_mem {
                ui.horizontal(|ui| {
                    if dirty_checkbox(
                        ui,
                        &mut self.settings.memory_logging,
                        "Sample memory (mem.log)",
                        ht::SET_MEM_LOG,
                    ) {
                        self.mark_dirty();
                    }
                    ui.label("every");
                    let mut v = self.settings.memory_logging_interval_ms;
                    if ui
                        .add(
                            crate::widgets::num_entry(&mut v)
                                .range(
                                    sqyre_persist::MIN_MEMORY_SAMPLE_INTERVAL_MS
                                        ..=sqyre_persist::MAX_MEMORY_SAMPLE_INTERVAL_MS,
                                )
                                .speed(500)
                                .suffix(" ms"),
                        )
                        .on_hover_text(ht::SET_MEM_INTERVAL)
                        .changed()
                    {
                        self.settings.memory_logging_interval_ms = v;
                        self.mark_dirty();
                    }
                });
            }
        }
    }

    fn draw_macros(
        &mut self,
        ui: &mut egui::Ui,
        q: &str,
        section_hit: bool,
        macro_hotkeys: &sqyre_hotkeys::MacroHotkeyBridge,
    ) {
        let mut first = true;
        self.draw_macro_playback(ui, q, section_hit, &mut first);
        self.draw_macro_recording(ui, q, section_hit, &mut first);
        self.draw_macro_hotkeys(ui, q, section_hit, &mut first, macro_hotkeys);
        self.draw_macro_image_search(ui, q, section_hit, &mut first);
        self.draw_macro_action_defaults(ui, q, section_hit, &mut first);
    }

    fn draw_macro_playback(
        &mut self,
        ui: &mut egui::Ui,
        q: &str,
        section_hit: bool,
        first: &mut bool,
    ) {
        let hit = section_hit || query_matches(q, SUBSECTION_PLAYBACK);
        let show_highlight = setting_visible(q, hit, SETTING_HIGHLIGHT_ACTION);
        let show_click_fx = setting_visible(q, hit, SETTING_CLICK_FX);
        let show_release = setting_visible(q, hit, SETTING_RELEASE_HELD);
        let show_overlay_stop = setting_visible(q, hit, SETTING_OVERLAY_PRESS_STOPS);
        let show_overlay_detect = setting_visible(q, hit, SETTING_OVERLAY_IN_DETECTION);
        let show_overlay_over = setting_visible(q, hit, SETTING_OVERLAY_OVER_SQYRE);
        let show_cursor = setting_visible(q, hit, SETTING_RESTORE_CURSOR);
        let show_while = setting_visible(q, hit, SETTING_WHILE_BUDGET);
        let show_depth = setting_visible(q, hit, SETTING_RUN_MACRO_DEPTH);
        let show_flash_ms = setting_visible(q, hit, SETTING_CLICK_FLASH_MS);
        if !(show_highlight
            || show_click_fx
            || show_release
            || show_cursor
            || show_overlay_stop
            || show_overlay_detect
            || show_overlay_over
            || show_while
            || show_depth
            || show_flash_ms)
        {
            return;
        }
        subsection_heading(ui, first, "Playback");

        if show_highlight
            && dirty_checkbox(
                ui,
                &mut self.settings.highlight_active_action,
                "Highlight the currently executing action",
                ht::SET_RUN_ROW_HL,
            )
        {
            self.mark_dirty();
        }

        if show_click_fx
            && dirty_checkbox(
                ui,
                &mut self.settings.click_effect,
                "Flash a marker where clicks land",
                ht::SET_CLICK_MARKER,
            )
        {
            self.mark_dirty();
        }

        if show_release
            && dirty_checkbox(
                ui,
                &mut self.settings.release_held_inputs_on_end,
                "Release held keys and buttons when a macro ends",
                ht::SET_RELEASE_KEYS,
            )
        {
            self.mark_dirty();
        }

        #[cfg(any(feature = "native-runtime", target_os = "windows"))]
        if show_cursor
            && dirty_checkbox(
                ui,
                &mut self.settings.restore_cursor_on_end,
                "Restore the mouse position when a macro ends",
                ht::SET_RESTORE_CURSOR,
            )
        {
            self.mark_dirty();
        }

        if show_overlay_stop
            && dirty_checkbox(
                ui,
                &mut self.settings.overlay_press_stops_run,
                "Overlay buttons stop the running macro",
                ht::SET_OVL_STOP,
            )
        {
            self.mark_dirty();
        }

        if show_overlay_detect
            && dirty_checkbox(
                ui,
                &mut self.settings.overlay_buttons_in_detection,
                "Overlay buttons appear in detection",
                ht::SET_CAPTURE_BUTTONS,
            )
        {
            self.mark_dirty();
        }

        if show_overlay_over
            && dirty_checkbox(
                ui,
                &mut self.settings.overlay_buttons_over_sqyre,
                "Show overlay buttons over Sqyre",
                ht::SET_BUTTONS_FRONT,
            )
        {
            self.mark_dirty();
        }

        if show_while || show_depth || show_flash_ms {
            advanced_header(
                ui,
                "settings_macros_playback_advanced",
                q,
                &[
                    SETTING_WHILE_BUDGET,
                    SETTING_RUN_MACRO_DEPTH,
                    SETTING_CLICK_FLASH_MS,
                ],
                |ui| {
                    if show_flash_ms {
                        ui.horizontal(|ui| {
                            ui.label("Click marker stays visible for:");
                            let mut v = self.settings.click_flash_ms;
                            if ui
                                .add(
                                    crate::widgets::num_entry(&mut v)
                                        .range(
                                            sqyre_persist::MIN_CLICK_FLASH_MS
                                                ..=sqyre_persist::MAX_CLICK_FLASH_MS,
                                        )
                                        .speed(50)
                                        .suffix(" ms"),
                                )
                                .on_hover_text(ht::SET_CLICK_FLASH_MS)
                                .changed()
                            {
                                self.settings.click_flash_ms = v;
                                self.mark_dirty();
                            }
                        });
                    }

                    if show_while {
                        ui.horizontal(|ui| {
                            ui.label("Stop endless loops after:");
                            let mut v = self.settings.while_max_iterations;
                            if ui
                                .add(
                                    crate::widgets::num_entry(&mut v)
                                        .range(
                                            sqyre_persist::MIN_WHILE_MAX_ITERATIONS
                                                ..=sqyre_persist::MAX_WHILE_MAX_ITERATIONS,
                                        )
                                        .speed(1000)
                                        .suffix(" iterations"),
                                )
                                .on_hover_text(ht::SET_WHILE_BUDGET)
                                .changed()
                            {
                                self.settings.while_max_iterations = v;
                                self.mark_dirty();
                            }
                        });
                    }

                    if show_depth {
                        ui.horizontal(|ui| {
                            ui.label("Limit nested Run Macro calls:");
                            let mut v = self.settings.run_macro_max_depth;
                            if ui
                                .add(
                                    crate::widgets::num_entry(&mut v)
                                        .range(
                                            sqyre_persist::MIN_RUN_MACRO_MAX_DEPTH
                                                ..=sqyre_persist::MAX_RUN_MACRO_MAX_DEPTH,
                                        )
                                        .speed(1),
                                )
                                .on_hover_text(ht::SET_RUN_DEPTH)
                                .changed()
                            {
                                self.settings.run_macro_max_depth = v;
                                self.mark_dirty();
                            }
                        });
                    }
                },
            );
        }
    }

    fn draw_macro_recording(
        &mut self,
        ui: &mut egui::Ui,
        q: &str,
        section_hit: bool,
        first: &mut bool,
    ) {
        let hit = section_hit || query_matches(q, SUBSECTION_RECORDING);
        let show_hide = setting_visible(q, hit, SETTING_HIDE_WHILE_RECORDING);
        let show_thresholds = setting_visible(q, hit, SETTING_RECORD_THRESHOLDS);
        if !(show_hide || show_thresholds) {
            return;
        }
        subsection_heading(ui, first, "Recording");
        if show_hide
            && dirty_checkbox(
                ui,
                &mut self.settings.hide_app_during_recording,
                "Hide Sqyre while recording points and search areas",
                ht::SET_HIDE_ON_RECORD,
            )
        {
            self.mark_dirty();
        }

        if show_thresholds {
            advanced_header(
                ui,
                "settings_macros_recording_advanced",
                q,
                &[SETTING_RECORD_THRESHOLDS],
                |ui| {
                    ui.horizontal(|ui| {
                        ui.label("Write a Wait after a pause of:");
                        let mut v = self.settings.record_wait_threshold_ms;
                        if ui
                            .add(
                                crate::widgets::num_entry(&mut v)
                                    .range(
                                        sqyre_persist::MIN_RECORD_WAIT_THRESHOLD_MS
                                            ..=sqyre_persist::MAX_RECORD_WAIT_THRESHOLD_MS,
                                    )
                                    .speed(5)
                                    .suffix(" ms"),
                            )
                            .on_hover_text(ht::SET_REC_WAIT)
                            .changed()
                        {
                            self.settings.record_wait_threshold_ms = v;
                            self.mark_dirty();
                        }
                    });
                    ui.horizontal(|ui| {
                        ui.label("Drop mouse moves shorter than:");
                        let mut v = self.settings.record_move_min_distance;
                        if ui
                            .add(
                                crate::widgets::num_entry(&mut v)
                                    .range(
                                        sqyre_persist::MIN_RECORD_MOVE_MIN_DISTANCE
                                            ..=sqyre_persist::MAX_RECORD_MOVE_MIN_DISTANCE,
                                    )
                                    .speed(1)
                                    .suffix(" px"),
                            )
                            .on_hover_text(ht::SET_REC_MOVE)
                            .changed()
                        {
                            self.settings.record_move_min_distance = v;
                            self.mark_dirty();
                        }
                    });
                    ui.horizontal(|ui| {
                        ui.label("Clicks held longer than this become down/up:");
                        let mut v = self.settings.record_tap_max_ms;
                        if ui
                            .add(
                                crate::widgets::num_entry(&mut v)
                                    .range(
                                        sqyre_persist::MIN_RECORD_TAP_MAX_MS
                                            ..=sqyre_persist::MAX_RECORD_TAP_MAX_MS,
                                    )
                                    .speed(10)
                                    .suffix(" ms"),
                            )
                            .on_hover_text(ht::SET_REC_TAP)
                            .changed()
                        {
                            self.settings.record_tap_max_ms = v;
                            self.mark_dirty();
                        }
                    });
                    ui.horizontal(|ui| {
                        ui.label("Snap recorded points to saved points within:");
                        let mut v = self.settings.record_snap_point_px;
                        if ui
                            .add(
                                crate::widgets::num_entry(&mut v)
                                    .range(
                                        sqyre_persist::MIN_RECORD_SNAP_POINT_PX
                                            ..=sqyre_persist::MAX_RECORD_SNAP_POINT_PX,
                                    )
                                    .speed(1)
                                    .suffix(" px"),
                            )
                            .on_hover_text(ht::SET_REC_SNAP)
                            .changed()
                        {
                            self.settings.record_snap_point_px = v;
                            self.mark_dirty();
                        }
                    });
                },
            );
        }
    }

    fn draw_macro_hotkeys(
        &mut self,
        ui: &mut egui::Ui,
        q: &str,
        section_hit: bool,
        first: &mut bool,
        macro_hotkeys: &sqyre_hotkeys::MacroHotkeyBridge,
    ) {
        let hit = section_hit || query_matches(q, SUBSECTION_HOTKEYS);
        let show_tags = setting_visible(q, hit, SETTING_HOTKEY_TAGS_FOCUSED);
        #[cfg(not(target_arch = "wasm32"))]
        let show_panic = setting_visible(q, hit, SETTING_PANIC_KEY);
        let show_retrigger = setting_visible(q, hit, SETTING_RETRIGGER);
        let show_trigger = setting_visible(q, hit, SETTING_DEFAULT_TRIGGER);
        let _ = macro_hotkeys;
        #[cfg(not(target_arch = "wasm32"))]
        if !(show_tags || show_retrigger || show_trigger || show_panic) {
            return;
        }
        #[cfg(target_arch = "wasm32")]
        if !(show_tags || show_retrigger || show_trigger) {
            return;
        }
        subsection_heading(ui, first, "Hotkeys");
        if show_tags
            && dirty_checkbox(
                ui,
                &mut self.settings.hotkey_tags_while_focused,
                "Select hotkey tags while a Program is focused",
                ht::SET_HOTKEY_TAGS,
            )
        {
            self.mark_dirty();
        }

        #[cfg(not(target_arch = "wasm32"))]
        if show_panic {
            ui.horizontal(|ui| {
                ui.label("Panic/stop key:").on_hover_text(ht::SET_PANIC_KEY);
                let label = if self.settings.panic_key.trim().is_empty() {
                    "(disabled)".to_string()
                } else {
                    self.settings.panic_key.trim().to_string()
                };
                ui.label(egui::RichText::new(label).monospace());
                if crate::widgets::record_icon_button(ui, ht::SET_PANIC_KEY, true).clicked() {
                    self.panic_record.open(macro_hotkeys);
                }
                if crate::widgets::icon_button(
                    ui,
                    egui_phosphor::regular::ARROW_COUNTER_CLOCKWISE,
                    "Reset panic key to Esc",
                )
                .clicked()
                {
                    self.settings.panic_key = "esc".to_string();
                    self.mark_dirty();
                }
                if crate::widgets::clear_icon_button(ui, "Disable panic/stop key").clicked() {
                    self.settings.panic_key.clear();
                    self.mark_dirty();
                }
            });
        }

        if show_retrigger {
            ui.horizontal(|ui| {
                ui.label("Hotkey pressed while its macro is running:")
                    .on_hover_text(ht::SET_RETRIGGER);
                let mut policy = self.settings.hotkey_while_running;
                egui::ComboBox::from_id_salt("settings_hotkey_while_running")
                    .selected_text(policy.label())
                    .show_ui(ui, |ui| {
                        for p in HotkeyWhileRunning::ALL {
                            ui.selectable_value(&mut policy, p, p.label());
                        }
                    });
                if policy != self.settings.hotkey_while_running {
                    self.settings.hotkey_while_running = policy;
                    self.mark_dirty();
                }
            });
        }

        if show_trigger {
            ui.horizontal(|ui| {
                ui.label("New macro hotkeys fire on:")
                    .on_hover_text(ht::SET_DEFAULT_TRIGGER);
                let mut trigger =
                    sqyre_hotkeys::HotkeyTrigger::parse(&self.settings.default_hotkey_trigger);
                egui::ComboBox::from_id_salt("settings_default_hotkey_trigger")
                    .selected_text(trigger.as_str())
                    .show_ui(ui, |ui| {
                        ui.selectable_value(
                            &mut trigger,
                            sqyre_hotkeys::HotkeyTrigger::Press,
                            "press",
                        );
                        ui.selectable_value(
                            &mut trigger,
                            sqyre_hotkeys::HotkeyTrigger::Release,
                            "release",
                        );
                    });
                if trigger.as_str() != self.settings.default_hotkey_trigger {
                    self.settings.default_hotkey_trigger = trigger.as_str().to_string();
                    self.mark_dirty();
                }
            });
        }
    }

    /// Defaults stamped onto new macros; action defaults are edited per type
    /// in the Add Action picker.
    fn draw_macro_action_defaults(
        &mut self,
        ui: &mut egui::Ui,
        q: &str,
        section_hit: bool,
        first: &mut bool,
    ) {
        let hit = section_hit || query_matches(q, SUBSECTION_ACTION_DEFAULTS);
        let show_delays = setting_visible(q, hit, SETTING_DEFAULT_DELAYS);
        let show_log_edge = setting_visible(q, hit, SETTING_LOG_IMAGE_EDGE);
        let show_saved = setting_visible(q, hit, SETTING_ACTION_DEFAULTS_RESET);
        if !(show_delays || show_log_edge || show_saved) {
            return;
        }
        subsection_heading(ui, first, "New macros & actions");

        if show_delays {
            ui.horizontal(|ui| {
                ui.label("New macro delays — key:")
                    .on_hover_text(ht::SET_DEFAULT_DELAYS);
                if dirty_i32(
                    ui,
                    &mut self.settings.default_keyboard_delay_ms,
                    ht::SET_DEFAULT_DELAYS,
                    |d| d.range(0..=60_000).speed(5).suffix(" ms"),
                ) {
                    self.mark_dirty();
                }
                ui.label("mouse:");
                if dirty_i32(
                    ui,
                    &mut self.settings.default_mouse_delay_ms,
                    ht::SET_DEFAULT_DELAYS,
                    |d| d.range(0..=60_000).speed(5).suffix(" ms"),
                ) {
                    self.mark_dirty();
                }
            });
        }

        if show_log_edge {
            ui.horizontal(|ui| {
                ui.label("Logged search images capped at:");
                let mut v = self.settings.log_image_max_edge;
                if ui
                    .add(
                        crate::widgets::num_entry(&mut v)
                            .range(
                                sqyre_persist::MIN_LOG_IMAGE_MAX_EDGE
                                    ..=sqyre_persist::MAX_LOG_IMAGE_MAX_EDGE,
                            )
                            .speed(32)
                            .suffix(" px"),
                    )
                    .on_hover_text(ht::SET_LOG_IMAGE_EDGE)
                    .changed()
                {
                    self.settings.log_image_max_edge = v;
                    self.mark_dirty();
                }
            });
        }

        if show_saved {
            let saved = self.settings.action_defaults.len();
            ui.horizontal(|ui| {
                if ui
                    .add_enabled(
                        saved > 0,
                        crate::widgets::IconButton::new(
                            egui_phosphor::regular::ARROW_COUNTER_CLOCKWISE,
                            ht::SET_RESET_ACTION_DEFAULTS,
                            None,
                        ),
                    )
                    .clicked()
                {
                    self.settings.action_defaults.clear();
                    self.mark_dirty();
                }
                ui.label(egui::RichText::new(format!("{saved} saved")).weak().small());
            });
        }
    }

    fn draw_macro_image_search(
        &mut self,
        ui: &mut egui::Ui,
        q: &str,
        section_hit: bool,
        first: &mut bool,
    ) {
        let hit = section_hit || query_matches(q, SUBSECTION_IMAGE_SEARCH);
        let show_variant = setting_visible(q, hit, SETTING_VARIANT_EXIT_EARLY);
        let show_distance = setting_visible(q, hit, SETTING_IMAGE_SEARCH_DISTANCE);
        if !(show_variant || show_distance) {
            return;
        }
        subsection_heading(ui, first, "Image Search");

        if show_variant
            && dirty_checkbox(
                ui,
                &mut self.settings.image_search_variant_exit_early,
                "Stop at the first matching icon variant",
                ht::SET_VARIANT_STOP,
            )
        {
            self.mark_dirty();
        }

        if show_distance {
            advanced_header(
                ui,
                "settings_macros_image_search_advanced",
                q,
                &[SETTING_IMAGE_SEARCH_DISTANCE],
                |ui| {
                    ui.horizontal(|ui| {
                        ui.label("Ignore nearby duplicate image matches:");
                        if dirty_i32(
                            ui,
                            &mut self.settings.image_search_close_matches_distance,
                            ht::SET_DEDUP_PX,
                            |d| d.range(0..=100).speed(1).suffix(" px"),
                        ) {
                            self.mark_dirty();
                        }
                    });
                },
            );
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
            && dirty_checkbox(
                ui,
                &mut self.settings.play_finish_sound,
                "Play a sound when a macro finishes",
                ht::SET_FINISH_CUE,
            )
        {
            self.mark_dirty();
        }

        if setting_visible(q, section_hit, SETTING_UI_SOUNDS) {
            let mut changed = dirty_checkbox(
                ui,
                &mut self.settings.play_add_sound,
                "Play a sound when adding",
                ht::SET_ADD_DEL_CUES,
            );
            changed |= dirty_checkbox(
                ui,
                &mut self.settings.play_delete_sound,
                "Play a sound when deleting",
                ht::SET_ADD_DEL_CUES,
            );
            if changed {
                self.mark_dirty();
            }
        }

        if setting_visible(q, section_hit, SETTING_SOUND_VOLUME) {
            ui.horizontal(|ui| {
                ui.label("Sound volume:");
                let mut pct = (self.settings.sound_volume * 100.0).round() as i32;
                if ui
                    .add(egui::Slider::new(&mut pct, 0..=100).suffix("%"))
                    .on_hover_text(ht::SET_CUE_VOLUME)
                    .changed()
                {
                    self.settings.sound_volume = pct as f32 / 100.0;
                    self.mark_dirty();
                }
            });
        }

        #[cfg(not(target_arch = "wasm32"))]
        if setting_visible(q, section_hit, SETTING_SOUND_FILES) {
            self.sound_file_row(
                ui,
                "Finish sound:",
                PickPurpose::FinishSound,
                ht::SET_FINISH_FILE,
                |s| &mut s.finish_sound_file,
            );
            self.sound_file_row(
                ui,
                "Add sound:",
                PickPurpose::AddSound,
                ht::SET_ADD_FILE,
                |s| &mut s.add_sound_file,
            );
            self.sound_file_row(
                ui,
                "Delete sound:",
                PickPurpose::DeleteSound,
                ht::SET_DELETE_FILE,
                |s| &mut s.delete_sound_file,
            );
        }
    }

    /// Cue-sound file row: current file name, a picker, and a built-in reset.
    #[cfg(not(target_arch = "wasm32"))]
    fn sound_file_row(
        &mut self,
        ui: &mut egui::Ui,
        label: &str,
        purpose: PickPurpose,
        help: &str,
        field: impl Fn(&mut UserSettings) -> &mut String,
    ) {
        ui.horizontal(|ui| {
            ui.label(label).on_hover_text(help);
            let path = field(&mut self.settings);
            let name = if path.trim().is_empty() {
                "built-in".to_string()
            } else {
                std::path::Path::new(path.trim())
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_else(|| path.trim().to_string())
            };
            ui.label(egui::RichText::new(name).weak().small());
            if crate::file_dialogs::supported(purpose)
                && settings_choose_btn(ui, "Choose…").clicked()
            {
                crate::file_dialogs::request(purpose, "Choose cue sound", None);
            }
            if !field(&mut self.settings).trim().is_empty()
                && crate::widgets::icon_button(
                    ui,
                    egui_phosphor::regular::HOUSE,
                    "Use built-in sound",
                )
                .clicked()
            {
                field(&mut self.settings).clear();
                self.mark_dirty();
            }
        });
    }

    fn draw_data(
        &mut self,
        ui: &mut egui::Ui,
        db: &mut Database,
        macros: &mut Vec<Macro>,
        catalog: &mut ProgramCatalog,
        q: &str,
        section_hit: bool,
    ) {
        if !setting_visible(q, section_hit, DATA_LOCATION) {
            return;
        }
        #[cfg(target_arch = "wasm32")]
        {
            let _ = (db, macros, catalog);
            ui.label(
                "Browser editor: macros live in memory. Use Import / Export on the toolbar for db.yaml. Full backups are not available.",
            );
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            let current = if self.settings.sqyre_dir.trim().is_empty() {
                sqyre_dir()
            } else {
                PathBuf::from(self.settings.sqyre_dir.trim())
            };
            ui.label(current.display().to_string());

            let flatpak = sqyre_update::is_flatpak_install();
            if flatpak {
                ui.label(
                    egui::RichText::new(
                        "Flatpak keeps a private data copy by default. To share macros \
                         with AppImage/native, grant access and point at host ~/.sqyre.",
                    )
                    .weak()
                    .small(),
                );
            }

            ui.horizontal_wrapped(|ui| {
                if settings_folder_btn(ui, "Open .sqyre folder").clicked() {
                    match open_sqyre_dir() {
                        Ok(()) => self.set_ok("Opened data folder."),
                        Err(e) => self.set_err(format!(
                            "Open folder failed: {}",
                            crate::status_banner::user_error_detail(&e)
                        )),
                    }
                }
                if crate::file_dialogs::supported(PickPurpose::SqyreLocation)
                    && settings_choose_btn(ui, "Choose location…").clicked()
                {
                    self.choose_sqyre_location();
                }
                if flatpak {
                    if settings_folder_btn(ui, ht::SET_HOST_FOLDER).clicked() {
                        self.choose_host_sqyre_location();
                    }
                    if !self.settings.sqyre_dir.trim().is_empty()
                        && settings_reset_btn(ui, ht::SET_FLATPAK_RESET).clicked()
                    {
                        self.reset_sqyre_to_sandbox(db, macros, catalog);
                    }
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

            if dirty_checkbox(
                ui,
                &mut self.settings.backup_enabled,
                "Automatic backups",
                ht::SET_AUTO_BACKUP,
            ) {
                self.mark_dirty();
            }

            ui.horizontal(|ui| {
                ui.label("Interval (hours):");
                if dirty_i32(
                    ui,
                    &mut self.settings.backup_interval_hours,
                    ht::SET_BACKUP_HOURS,
                    |d| {
                        d.range(MIN_BACKUP_INTERVAL_HOURS..=MAX_BACKUP_INTERVAL_HOURS)
                            .speed(1)
                    },
                ) {
                    self.mark_dirty();
                }
                if settings_reset_btn(ui, "Reset to default").clicked() {
                    self.settings.backup_interval_hours = DEFAULT_BACKUP_INTERVAL_HOURS;
                    self.mark_dirty();
                }
            });

            ui.horizontal(|ui| {
                ui.label("Keep at most:");
                if dirty_i32(
                    ui,
                    &mut self.settings.backup_max_keep,
                    ht::SET_BACKUP_KEEP,
                    |d| d.range(MIN_BACKUP_MAX_KEEP..=MAX_BACKUP_MAX_KEEP).speed(1),
                ) {
                    self.mark_dirty();
                }
                if settings_reset_btn(ui, "Reset to default").clicked() {
                    self.settings.backup_max_keep = DEFAULT_BACKUP_MAX_KEEP;
                    self.mark_dirty();
                }
            });

            // The path is its own wrapping line. A horizontal row with the
            // buttons lets a long Windows path push Choose past the pane.
            ui.label(
                egui::RichText::new(format!("Backups folder: {}", backups_dir().display()))
                    .weak()
                    .small(),
            );
            ui.horizontal_wrapped(|ui| {
                if crate::file_dialogs::supported(PickPurpose::BackupDir)
                    && settings_choose_btn(ui, ht::SET_BACKUP_DIR).clicked()
                {
                    crate::file_dialogs::request(
                        PickPurpose::BackupDir,
                        "Choose backups folder",
                        Some(&backups_dir()),
                    );
                }
                if !self.settings.backup_dir.trim().is_empty()
                    && settings_reset_btn(ui, ht::SET_BACKUP_DIR).clicked()
                {
                    self.settings.backup_dir.clear();
                    self.settings.apply_backup_dir_override();
                    self.mark_dirty();
                }
            });
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

            ui.horizontal(|ui| {
                if crate::widgets::icon_button(ui, egui_phosphor::regular::ARCHIVE, "Back up now")
                    .clicked()
                {
                    self.run_manual_backup();
                }
                if crate::widgets::icon_button(
                    ui,
                    egui_phosphor::regular::UPLOAD_SIMPLE,
                    "Restore from backup…",
                )
                .clicked()
                {
                    self.request_restore_backup();
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
                    self.set_err(format!(
                        "Backup created but prune failed: {}",
                        crate::status_banner::user_error_detail(&e)
                    ));
                }
                self.note_backup_success(&path);
            }
            Err(e) => self.set_err(format!(
                "Backup failed: {}",
                crate::status_banner::user_error_detail(&e)
            )),
        }
    }

    /// Record a failed automatic backup in the Settings status line (the log
    /// keeps the same warning so diagnostics don't regress).
    #[cfg(not(target_arch = "wasm32"))]
    pub fn note_backup_failure(&mut self, err: &str) {
        crate::log::warn(format_args!("automatic backup failed: {err}"));
        self.set_err(format!(
            "Automatic backup failed: {}",
            crate::status_banner::user_error_detail(err)
        ));
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

    /// Pick a backup zip; [`Self::apply_pick`] opens Settings on the restore confirm.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn request_restore_backup(&mut self) {
        let start = backups_dir();
        let start = if start.exists() { start } else { sqyre_dir() };
        crate::file_dialogs::request(
            PickPurpose::RestoreBackup,
            "Restore from backup",
            Some(&start),
        );
    }

    /// Finish a Settings pick and open Settings on its confirm dialog.
    pub(crate) fn apply_pick(&mut self, purpose: PickPurpose, path: PathBuf) {
        match purpose {
            PickPurpose::RestoreBackup => {
                self.confirm = Some(PendingConfirm::RestoreBackup { path });
            }
            PickPurpose::SqyreLocation | PickPurpose::HostSqyreLocation => {
                self.confirm_sqyre_location(resolve_sqyre_dir_from_pick(path));
            }
            PickPurpose::FinishSound => {
                self.settings.finish_sound_file = path.to_string_lossy().into_owned();
                self.mark_dirty();
            }
            PickPurpose::AddSound => {
                self.settings.add_sound_file = path.to_string_lossy().into_owned();
                self.mark_dirty();
            }
            PickPurpose::DeleteSound => {
                self.settings.delete_sound_file = path.to_string_lossy().into_owned();
                self.mark_dirty();
            }
            PickPurpose::BackupDir => {
                self.settings.backup_dir = path.to_string_lossy().into_owned();
                self.settings.apply_backup_dir_override();
                self.mark_dirty();
            }
            PickPurpose::IconVariant | PickPurpose::MaskImage => return,
        }
        if self.confirm.is_some() {
            self.open = true;
        }
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

        let flatpak = sqyre_update::is_flatpak_install();

        if setting_visible(q, section_hit, SETTING_UPDATE_VERSION) {
            ui.label(format!("Current version: {SQYRE_VERSION}"));
        }

        if flatpak {
            if setting_visible(q, section_hit, SETTING_UPDATE_ACTIONS) {
                ui.label(
                    "Flatpak install: updates come from flatpak update or your software center \
                     (in-app self-replace is not supported).",
                );
            }
            return;
        }

        if setting_visible(q, section_hit, SETTING_AUTO_UPDATE)
            && dirty_checkbox(
                ui,
                &mut self.settings.auto_update_check,
                "Check for updates on startup",
                ht::SET_UPDATE_CHECK,
            )
        {
            self.mark_dirty();
        }

        if setting_visible(q, section_hit, SETTING_UPDATE_INTERVAL) {
            ui.horizontal(|ui| {
                ui.label("Check for updates every:");
                let mut v = self.settings.update_check_interval_hours;
                if ui
                    .add(
                        crate::widgets::num_entry(&mut v)
                            .range(
                                sqyre_persist::MIN_UPDATE_CHECK_INTERVAL_HOURS
                                    ..=sqyre_persist::MAX_UPDATE_CHECK_INTERVAL_HOURS,
                            )
                            .speed(1)
                            .suffix(" hours"),
                    )
                    .on_hover_text(ht::SET_UPDATE_INTERVAL)
                    .changed()
                {
                    self.settings.update_check_interval_hours = v;
                    self.mark_dirty();
                }
            });
            ui.label(
                egui::RichText::new("0 hours checks once per launch.")
                    .small()
                    .weak(),
            );
        }

        if setting_visible(q, section_hit, SETTING_UPDATE_PRERELEASE)
            && dirty_checkbox(
                ui,
                &mut self.settings.update_include_prerelease,
                "Include pre-release versions",
                ht::SET_UPDATE_PRERELEASE,
            )
        {
            self.mark_dirty();
        }

        if setting_visible(q, section_hit, SETTING_UPDATE_ACTIONS) {
            ui.horizontal(|ui| {
                let busy = update.is_busy();
                if ui
                    .add_enabled(
                        !busy,
                        crate::widgets::IconButton::new(
                            egui_phosphor::regular::ARROW_CLOCKWISE,
                            ht::SET_UPDATE_NOW,
                            None,
                        ),
                    )
                    .clicked()
                {
                    update.start_check(self.settings.update_include_prerelease);
                    ui.ctx().request_repaint();
                }
                match &update.state {
                    UpdateState::Available { .. }
                        if ui
                            .add_enabled(
                                !busy,
                                crate::widgets::IconButton::new(
                                    egui_phosphor::regular::DOWNLOAD_SIMPLE,
                                    "Download & install",
                                    None,
                                ),
                            )
                            .clicked() =>
                    {
                        update.start_download();
                    }
                    UpdateState::Ready { .. }
                        if crate::widgets::icon_button(
                            ui,
                            egui_phosphor::regular::ARROW_CLOCKWISE,
                            "Restart to finish",
                        )
                        .clicked() =>
                    {
                        self.restart_requested = true;
                    }
                    _ => {}
                }
            });

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
            let busy = matches!(
                update.state,
                UpdateState::Checking | UpdateState::Downloading { .. }
            );
            if busy {
                crate::widgets::loading_line(ui, egui::RichText::new(status).weak().small());
            } else {
                ui.label(egui::RichText::new(status).weak().small());
            }
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn choose_sqyre_location(&mut self) {
        let start = sqyre_dir()
            .parent()
            .map(PathBuf::from)
            .unwrap_or_else(sqyre_dir);
        crate::file_dialogs::request(
            PickPurpose::SqyreLocation,
            "Choose .sqyre location",
            Some(&start),
        );
    }

    /// Flatpak: portal-pick host `~/.sqyre` (or its parent) so the sandbox can use it.
    #[cfg(not(target_arch = "wasm32"))]
    fn choose_host_sqyre_location(&mut self) {
        let start = std::env::var_os("HOME")
            .map(PathBuf::from)
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or_else(sqyre_dir);
        crate::file_dialogs::request(
            PickPurpose::HostSqyreLocation,
            "Select host ~/.sqyre (or your Home folder)",
            Some(&start),
        );
    }

    fn confirm_sqyre_location(&mut self, new_dir: PathBuf) {
        let old_dir = sqyre_dir();
        if new_dir == old_dir {
            return;
        }
        self.confirm = Some(PendingConfirm::MoveData { old_dir, new_dir });
    }

    /// Clear a custom data path and reload from the default Flatpak/native location.
    #[cfg(not(target_arch = "wasm32"))]
    fn reset_sqyre_to_sandbox(
        &mut self,
        db: &mut Database,
        macros: &mut Vec<Macro>,
        catalog: &mut ProgramCatalog,
    ) {
        if self.settings.sqyre_dir.trim().is_empty() {
            return;
        }
        self.settings.sqyre_dir.clear();
        set_sqyre_dir_override(None);
        self.persist();
        let path = sqyre_dir();
        self.reload_data_from_disk(db, macros, catalog, &path, "Using Flatpak sandbox data");
    }

    fn draw_confirm(
        &mut self,
        ctx: &egui::Context,
        confirm: PendingConfirm,
        pending_scale: Option<&crate::widgets::ViewportScaleEvent>,
        data: &mut SettingsData<'_>,
    ) {
        match &confirm {
            PendingConfirm::MoveData { old_dir, new_dir } => {
                let open = crate::widgets::confirm_window(
                    ctx,
                    "Move existing data?",
                    pending_scale,
                    |ui| {
                        ui.label(format!(
                        "Move your current data from\n{}\nto\n{}?\n\nChoose No to start fresh at the new location (existing data is left in place).",
                        old_dir.display(),
                        new_dir.display()
                    ));
                        // Enter → Yes (index 1).
                        match crate::widgets::confirm_choice_row(
                            ui,
                            &[
                                ("No", crate::widgets::ConfirmKind::Primary),
                                ("Yes", crate::widgets::ConfirmKind::Primary),
                            ],
                            Some(1),
                        ) {
                            crate::widgets::ConfirmChoice::Cancel => {
                                self.confirm = None;
                            }
                            crate::widgets::ConfirmChoice::Choice(0) => {
                                let old = old_dir.clone();
                                let new = new_dir.clone();
                                self.confirm = None;
                                self.apply_sqyre_location(
                                    old,
                                    new,
                                    false,
                                    data.db,
                                    data.macros,
                                    data.catalog,
                                );
                            }
                            crate::widgets::ConfirmChoice::Choice(1) => {
                                let old = old_dir.clone();
                                let new = new_dir.clone();
                                self.confirm = None;
                                self.apply_sqyre_location(
                                    old,
                                    new,
                                    true,
                                    data.db,
                                    data.macros,
                                    data.catalog,
                                );
                            }
                            crate::widgets::ConfirmChoice::Choice(_)
                            | crate::widgets::ConfirmChoice::None => {}
                        }
                    },
                );
                if !open {
                    self.confirm = None;
                }
            }
            PendingConfirm::RestoreBackup { path } => {
                let open = crate::widgets::confirm_window(
                    ctx,
                    "Import backup?",
                    pending_scale,
                    |ui| {
                        ui.label(format!(
                        "Archive:\n{}\n\nOverwrite replaces all macros, settings, images, and variables with the archive.\n\nMerge keeps live-only items, prefers the archive on name conflicts, replaces settings, and merges other assets. Automatic backups in the backups folder are not removed.",
                        path.display()
                    ));
                        // Enter → Overwrite (index 0); both are intentional verbs.
                        match crate::widgets::confirm_choice_row(
                            ui,
                            &[
                                ("Overwrite", crate::widgets::ConfirmKind::Destructive),
                                ("Merge", crate::widgets::ConfirmKind::Primary),
                            ],
                            Some(0),
                        ) {
                            crate::widgets::ConfirmChoice::Cancel => {
                                self.confirm = None;
                            }
                            crate::widgets::ConfirmChoice::Choice(0) => {
                                let path = path.clone();
                                self.confirm = None;
                                self.apply_restore_backup(
                                    path,
                                    ImportMode::Overwrite,
                                    data.db,
                                    data.macros,
                                    data.catalog,
                                );
                            }
                            crate::widgets::ConfirmChoice::Choice(1) => {
                                let path = path.clone();
                                self.confirm = None;
                                self.apply_restore_backup(
                                    path,
                                    ImportMode::Merge,
                                    data.db,
                                    data.macros,
                                    data.catalog,
                                );
                            }
                            crate::widgets::ConfirmChoice::Choice(_)
                            | crate::widgets::ConfirmChoice::None => {}
                        }
                    },
                );
                if !open {
                    self.confirm = None;
                }
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
            self.set_err(format!(
                "Import failed: {}",
                crate::status_banner::user_error_detail(&e)
            ));
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
                list.sort_by(|a, b| crate::macro_meta::cmp_display_name(&a.name, &b.name));
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
                self.set_err(format!(
                    "Imported archive but failed to load db.yaml: {}",
                    crate::status_banner::user_error_detail(&e)
                ));
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
                self.set_err(format!(
                    "Move failed: {}",
                    crate::status_banner::user_error_detail(&e)
                ));
                return;
            }
        }

        self.settings.sqyre_dir = new_dir.display().to_string();
        set_sqyre_dir_override(Some(new_dir.clone()));
        self.persist();
        self.reload_data_from_disk(
            db,
            macros,
            catalog,
            &new_dir,
            &format!("Data location changed to {}.", new_dir.display()),
        );
    }

    fn reload_data_from_disk(
        &mut self,
        db: &mut Database,
        macros: &mut Vec<Macro>,
        catalog: &mut ProgramCatalog,
        path: &std::path::Path,
        ok_msg: &str,
    ) {
        match Database::load_default() {
            Ok(mut loaded) => {
                let mut cat = Arc::unwrap_or_clone(loaded.program_catalog().unwrap_or_default());
                let _ = crate::catalog::prepare_catalog(&mut cat, &mut loaded);
                let mut list: Vec<_> = loaded.macros.values().cloned().collect();
                list.sort_by(|a, b| crate::macro_meta::cmp_display_name(&a.name, &b.name));
                *db = loaded;
                *macros = list;
                *catalog = cat;
                self.reload_requested = true;
                self.set_ok(ok_msg.to_string());
            }
            Err(e) => {
                *db = Database::default();
                macros.clear();
                *catalog = ProgramCatalog::default();
                crate::catalog::apply_main_monitor_resolution(catalog);
                let _ = crate::catalog::ensure_general_program_seeded(catalog);
                self.reload_requested = true;
                self.set_err(format!(
                    "Switched to {} but failed to load db.yaml: {}",
                    path.display(),
                    crate::status_banner::user_error_detail(&e)
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
            && dirty_checkbox(
                ui,
                &mut self.settings.compact_program_headers,
                "Compact program headers with icons",
                ht::SET_ICON_HEADERS,
            )
        {
            self.mark_dirty();
        }

        if setting_visible(q, section_hit, SETTING_EMPTY_HEADERS)
            && dirty_checkbox(
                ui,
                &mut self.settings.show_empty_program_headers,
                "Show empty program headers",
                ht::SET_KEEP_HEADERS,
            )
        {
            self.mark_dirty();
        }

        if setting_visible(q, section_hit, SETTING_WINDOW_BORDER)
            && dirty_checkbox(
                ui,
                &mut self.settings.window_border,
                "Window border",
                ht::SET_WIN_OUTLINE,
            )
        {
            self.mark_dirty();
        }

        #[cfg(not(target_arch = "wasm32"))]
        if setting_visible(q, section_hit, SETTING_WINDOW_DECORATIONS)
            && dirty_checkbox(
                ui,
                &mut self.settings.window_decorations,
                "OS title bar",
                ht::SET_WIN_DECORATIONS,
            )
        {
            ctx.send_viewport_cmd(egui::ViewportCommand::Decorations(
                self.settings.window_decorations,
            ));
            // Keep resize grips even after OS chrome is removed.
            ctx.send_viewport_cmd(egui::ViewportCommand::Resizable(true));
            self.mark_dirty();
        }

        if setting_visible(q, section_hit, SETTING_RUN_GLOW)
            && dirty_checkbox(
                ui,
                &mut self.settings.run_button_glow,
                "Glowing Run button",
                ht::SET_RUN_GLOW,
            )
        {
            self.mark_dirty();
        }

        if setting_visible(q, section_hit, SETTING_PREVIEW_OUTLINE)
            && dirty_checkbox(
                ui,
                &mut self.settings.preview_desktop_outline,
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
                    self.settings.accent_color =
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
                if !self.settings.accent_color.trim().is_empty()
                    && settings_reset_btn(ui, "Reset accent color").clicked()
                {
                    self.settings.accent_color.clear();
                    self.mark_dirty();
                    Self::apply_appearance(ctx, &self.settings);
                }
            });
        }

        if setting_visible(q, section_hit, SETTING_FONT_FAMILY) {
            ui.horizontal(|ui| {
                ui.label("Font family:").on_hover_text(ht::SET_FONT_FAMILY);
                let mut family = self.settings.ui_font_family;
                egui::ComboBox::from_id_salt("settings_ui_font_family")
                    .selected_text(family.label())
                    .show_ui(ui, |ui| {
                        for f in UiFontFamily::ALL {
                            ui.selectable_value(&mut family, f, f.label());
                        }
                    });
                if family != self.settings.ui_font_family {
                    self.settings.ui_font_family = family;
                    self.mark_dirty();
                    Self::apply_appearance(ctx, &self.settings);
                }
            });
        }

        if setting_visible(q, section_hit, SETTING_MONO_SIZE) {
            ui.horizontal(|ui| {
                ui.label("Code font size:").on_hover_text(ht::SET_MONO_SIZE);
                let mut v = self.settings.mono_font_size;
                if ui
                    .add(crate::widgets::num_entry(&mut v).range(0..=40).speed(1))
                    .on_hover_text(ht::SET_MONO_SIZE)
                    .changed()
                {
                    self.settings.mono_font_size = v;
                    self.mark_dirty();
                    Self::apply_appearance(ctx, &self.settings);
                }
                if settings_reset_btn(ui, "Reset to default").clicked() {
                    self.settings.mono_font_size = 0;
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
                let mut v = self.settings.ui_font_size;
                if ui
                    .add(crate::widgets::num_entry(&mut v).range(10..=28).speed(1))
                    .on_hover_text(ht::SET_FONT_SIZE)
                    .changed()
                {
                    self.settings.ui_font_size = v;
                    self.mark_dirty();
                    Self::apply_appearance(ctx, &self.settings);
                }
                if settings_reset_btn(ui, "Reset to default").clicked() {
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
                        crate::widgets::num_entry(&mut v)
                            .range(MIN_UI_SCALE..=MAX_UI_SCALE)
                            .speed(0.1)
                            .fixed_decimals(1),
                    )
                    .on_hover_text(ht::SET_UI_SCALE)
                    .changed()
                {
                    self.settings.ui_scale = v;
                    self.mark_dirty();
                    self.ui_scale_apply_pending = true;
                }
                if settings_reset_btn(ui, "Reset to default").clicked() {
                    self.settings.ui_scale = DEFAULT_UI_SCALE;
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
                let mut density = self.settings.ui_density;
                egui::ComboBox::from_id_salt("settings_ui_density")
                    .selected_text(density.label())
                    .show_ui(ui, |ui| {
                        for d in UiDensity::ALL {
                            ui.selectable_value(&mut density, d, d.label());
                        }
                    })
                    .response
                    .on_hover_text(ht::SET_DENSITY);
                if density != self.settings.ui_density {
                    self.settings.ui_density = density;
                    self.mark_dirty();
                    Self::apply_appearance(ctx, &self.settings);
                }
                if dirty_checkbox(
                    ui,
                    &mut self.settings.auto_compact_density,
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
                && settings_reset_btn(ui, "Reset all action colors").clicked()
            {
                self.settings.action_colors.clear_all();
                clear_all_custom_action_colors();
                self.mark_dirty();
            }
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
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
const SECTION_MACROS: &[&str] = &["macros", "macro"];
const SUBSECTION_PLAYBACK: &[&str] = &["playback", "running", "execution"];
const SUBSECTION_RECORDING: &[&str] = &["recording", "record"];
const SUBSECTION_HOTKEYS: &[&str] = &["hotkeys", "hotkey", "shortcuts"];
const SUBSECTION_IMAGE_SEARCH: &[&str] = &["image search", "detection"];
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
    "revoke",
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
    "action log",
    "log entries",
    "log cap",
];
const SETTING_HIGHLIGHT_ACTION: &[&str] =
    &["highlight", "executing action", "active action", "tint"];
const SETTING_CLICK_FX: &[&str] = &[
    "click effect",
    "click",
    "flash",
    "marker",
    "cursor",
    "clicks land",
];
const SETTING_HIDE_WHILE_RECORDING: &[&str] =
    &["hide", "recording", "points", "search areas", "snapshot"];
const SETTING_RELEASE_HELD: &[&str] = &[
    "release",
    "held keys",
    "held buttons",
    "inputs",
    "macro ends",
];
const SETTING_OVERLAY_PRESS_STOPS: &[&str] = &["overlay", "overlay button", "stop macro", "press"];
const SETTING_OVERLAY_IN_DETECTION: &[&str] = &[
    "overlay",
    "overlay button",
    "detection",
    "image search",
    "find pixel",
    "ocr",
];
const SETTING_OVERLAY_OVER_SQYRE: &[&str] = &[
    "overlay",
    "overlay button",
    "over sqyre",
    "on top",
    "show overlay",
    "sqyre window",
];
const SETTING_HOTKEY_TAGS_FOCUSED: &[&str] = &[
    "hotkey",
    "hotkeys",
    "tags",
    "focused",
    "focus",
    "program",
    "while focused",
];
#[cfg(all(not(target_arch = "wasm32"), feature = "native-runtime"))]
const SETTING_WORKER_THREADS: &[&str] = &[
    "cpu",
    "cpus",
    "threads",
    "worker",
    "workers",
    "rayon",
    "parallel",
    "cores",
    "image work",
];
const SETTING_WHILE_BUDGET: &[&str] = &[
    "while",
    "safety budget",
    "iterations",
    "loop",
    "max_iterations",
    "endless",
    "stop endless",
    "advanced",
];
const SETTING_RUN_MACRO_DEPTH: &[&str] = &[
    "run macro",
    "nesting",
    "depth",
    "recursion",
    "nested",
    "advanced",
];
const SETTING_IMAGE_SEARCH_DISTANCE: &[&str] = &[
    "image search",
    "close-match",
    "close match",
    "distance",
    "duplicate",
    "nearby",
    "advanced",
];
const SETTING_VARIANT_EXIT_EARLY: &[&str] = &[
    "variant",
    "variants",
    "exit early",
    "early exit",
    "first match",
    "image search",
    "icon variants",
];

const SETTING_FINISH_SOUND: &[&str] = &["finish sound", "macro finishes", "complete"];
const SETTING_UI_SOUNDS: &[&str] = &["ui sounds", "adding", "deleting", "add/delete"];
const SETTING_SOUND_VOLUME: &[&str] = &["volume", "sound volume"];
#[cfg(not(target_arch = "wasm32"))]
const SETTING_SOUND_FILES: &[&str] = &["sound file", "custom sound", "mp3", "cue file", "built-in"];

#[cfg(all(not(target_arch = "wasm32"), feature = "native-runtime"))]
const SETTING_DIAGNOSTICS: &[&str] = &[
    "diagnostic",
    "diagnostics",
    "diag",
    "diag.log",
    "debug",
    "troubleshoot",
    "crash",
    "log file",
];
#[cfg(all(not(target_arch = "wasm32"), feature = "native-runtime"))]
const SETTING_MEMORY_LOG: &[&str] = &[
    "memory",
    "memory log",
    "mem.log",
    "rss",
    "leak",
    "sample",
    "interval",
];
#[cfg(not(target_arch = "wasm32"))]
const SETTING_CLOSE_TO_TRAY: &[&str] =
    &["close", "close to tray", "tray", "minimize", "quit", "exit"];
#[cfg(not(target_arch = "wasm32"))]
const SETTING_START_MINIMIZED: &[&str] =
    &["start minimized", "tray", "startup", "launch", "hidden"];
#[cfg(target_os = "linux")]
const SETTING_AUTOSTART: &[&str] = &["autostart", "launch on login", "login", "boot", "startup"];
#[cfg(all(target_os = "linux", feature = "native-runtime"))]
const SETTING_SUSPEND_CAPTURE: &[&str] = &[
    "sleep",
    "idle",
    "screen capture",
    "screen sharing",
    "wake",
    "suspend",
    "pipewire",
];
const SETTING_LAST_MACRO: &[&str] = &["restore", "last macro", "remember", "reopen", "selected"];

const SETTING_RESTORE_CURSOR: &[&str] = &["cursor", "restore cursor", "mouse position", "restore"];
const SETTING_CLICK_FLASH_MS: &[&str] = &[
    "click",
    "flash",
    "marker",
    "duration",
    "milliseconds",
    "advanced",
];
const SETTING_RECORD_THRESHOLDS: &[&str] = &[
    "recording",
    "record",
    "threshold",
    "wait",
    "pause",
    "move",
    "tap",
    "snap",
    "distance",
    "advanced",
];
#[cfg(not(target_arch = "wasm32"))]
const SETTING_PANIC_KEY: &[&str] = &[
    "panic",
    "panic key",
    "stop key",
    "esc",
    "escape",
    "abort",
    "emergency",
    "record",
];
const SETTING_RETRIGGER: &[&str] = &[
    "hotkey",
    "while running",
    "restart",
    "retrigger",
    "already running",
    "running macro",
];
const SETTING_DEFAULT_TRIGGER: &[&str] =
    &["hotkey", "trigger", "press", "release", "new macro", "fire"];

const SUBSECTION_ACTION_DEFAULTS: &[&str] =
    &["defaults", "new actions", "new macros", "action defaults"];
const SETTING_DEFAULT_DELAYS: &[&str] = &[
    "delay",
    "delays",
    "keyboard",
    "mouse",
    "new macro",
    "default",
];
const SETTING_LOG_IMAGE_EDGE: &[&str] = &[
    "log image",
    "max edge",
    "image size",
    "meta image",
    "annotated",
    "advanced",
];
const SETTING_ACTION_DEFAULTS_RESET: &[&str] = &[
    "reset",
    "saved defaults",
    "action defaults",
    "save as default",
];

#[cfg(not(target_arch = "wasm32"))]
const SETTING_BACKUP_DIR: &[&str] = &[
    "backup folder",
    "backup directory",
    "backups folder",
    "destination",
    "backup location",
];
#[cfg(not(target_arch = "wasm32"))]
const SETTING_UPDATE_INTERVAL: &[&str] = &[
    "interval",
    "check every",
    "hours",
    "frequency",
    "update check",
];
#[cfg(not(target_arch = "wasm32"))]
const SETTING_UPDATE_PRERELEASE: &[&str] = &[
    "prerelease",
    "pre-release",
    "beta",
    "release candidate",
    "rc",
];

const SETTING_ACCENT: &[&str] = &[
    "accent",
    "accent color",
    "gold",
    "theme color",
    "highlight color",
];
const SETTING_FONT_FAMILY: &[&str] = &["font family", "monospace", "font", "typeface", "hack"];
const SETTING_MONO_SIZE: &[&str] = &["mono", "monospace", "code font", "font size", "yaml"];

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
    "revoke",
    "accessibility",
    "notifications",
];

const DATA_LOCATION: &[&str] = &[
    "data folder",
    "data directory",
    sqyre_ports::APP_ID,
    sqyre_ports::APP_DIR_NAME,
    "location",
    "choose location",
    "open folder",
    "flatpak",
    "host",
    "sandbox",
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
const SETTING_EMPTY_HEADERS: &[&str] = &["empty", "program headers", "headers", "hide"];
const SETTING_WINDOW_BORDER: &[&str] = &["window border", "border", "outline"];
#[cfg(not(target_arch = "wasm32"))]
const SETTING_WINDOW_DECORATIONS: &[&str] = &[
    "os title bar",
    "title bar",
    "decorations",
    "borderless",
    "frameless",
    "window chrome",
];
const SETTING_RUN_GLOW: &[&str] = &["run button", "glow", "pulse", "animation"];
const SETTING_PREVIEW_OUTLINE: &[&str] = &[
    "preview",
    "preview outline",
    "desktop outline",
    "location",
    "location box",
    "tooltip",
    "monitor",
    "gold box",
];
const SETTING_FONT_SIZE: &[&str] = &["font size", "font", "text size"];
const SETTING_UI_SCALE: &[&str] = &["ui scale", "scale", "padding"];
const SETTING_SPACING: &[&str] = &[
    "spacing", "density", "padding", "margin", "gap", "compact", "spaced",
];
const SETTING_ACTION_COLORS: &[&str] =
    &["action colors", "macro tree", "colors", "colour", "pastel"];

const GENERAL_SETTINGS: &[&[&str]] = &[
    SETTING_LOG_META,
    #[cfg(all(not(target_arch = "wasm32"), feature = "native-runtime"))]
    SETTING_WORKER_THREADS,
    SETTING_LAST_MACRO,
    #[cfg(not(target_arch = "wasm32"))]
    SETTING_CLOSE_TO_TRAY,
    #[cfg(not(target_arch = "wasm32"))]
    SETTING_START_MINIMIZED,
    #[cfg(target_os = "linux")]
    SETTING_AUTOSTART,
    #[cfg(all(target_os = "linux", feature = "native-runtime"))]
    SETTING_SUSPEND_CAPTURE,
    #[cfg(all(not(target_arch = "wasm32"), feature = "native-runtime"))]
    SETTING_DIAGNOSTICS,
    #[cfg(all(not(target_arch = "wasm32"), feature = "native-runtime"))]
    SETTING_MEMORY_LOG,
];
const MACROS_SETTINGS: &[&[&str]] = &[
    SUBSECTION_PLAYBACK,
    SETTING_HIGHLIGHT_ACTION,
    SETTING_CLICK_FX,
    SETTING_RELEASE_HELD,
    SETTING_RESTORE_CURSOR,
    SETTING_OVERLAY_PRESS_STOPS,
    SETTING_OVERLAY_IN_DETECTION,
    SETTING_OVERLAY_OVER_SQYRE,
    SETTING_WHILE_BUDGET,
    SETTING_RUN_MACRO_DEPTH,
    SETTING_CLICK_FLASH_MS,
    SUBSECTION_RECORDING,
    SETTING_HIDE_WHILE_RECORDING,
    SETTING_RECORD_THRESHOLDS,
    SUBSECTION_HOTKEYS,
    SETTING_HOTKEY_TAGS_FOCUSED,
    #[cfg(not(target_arch = "wasm32"))]
    SETTING_PANIC_KEY,
    SETTING_RETRIGGER,
    SETTING_DEFAULT_TRIGGER,
    SUBSECTION_IMAGE_SEARCH,
    SETTING_VARIANT_EXIT_EARLY,
    SETTING_IMAGE_SEARCH_DISTANCE,
    SUBSECTION_ACTION_DEFAULTS,
    SETTING_DEFAULT_DELAYS,
    SETTING_LOG_IMAGE_EDGE,
    SETTING_ACTION_DEFAULTS_RESET,
];
const SOUND_SETTINGS: &[&[&str]] = &[
    SETTING_FINISH_SOUND,
    SETTING_UI_SOUNDS,
    SETTING_SOUND_VOLUME,
    #[cfg(not(target_arch = "wasm32"))]
    SETTING_SOUND_FILES,
];
#[cfg(all(not(target_arch = "wasm32"), feature = "native-runtime"))]
const PERMISSIONS_SETTINGS: &[&[&str]] = &[PERMISSIONS_PANEL];
const DATA_SETTINGS: &[&[&str]] = &[
    DATA_LOCATION,
    DATA_BACKUP,
    #[cfg(not(target_arch = "wasm32"))]
    SETTING_BACKUP_DIR,
];
#[cfg(not(target_arch = "wasm32"))]
const UPDATES_SETTINGS: &[&[&str]] = &[
    SETTING_UPDATE_VERSION,
    SETTING_AUTO_UPDATE,
    SETTING_UPDATE_ACTIONS,
    SETTING_UPDATE_INTERVAL,
    SETTING_UPDATE_PRERELEASE,
];
const APPEARANCE_SETTINGS: &[&[&str]] = &[
    SETTING_COMPACT_HEADERS,
    SETTING_EMPTY_HEADERS,
    SETTING_WINDOW_BORDER,
    #[cfg(not(target_arch = "wasm32"))]
    SETTING_WINDOW_DECORATIONS,
    SETTING_RUN_GLOW,
    SETTING_PREVIEW_OUTLINE,
    SETTING_ACCENT,
    SETTING_FONT_FAMILY,
    SETTING_MONO_SIZE,
    SETTING_FONT_SIZE,
    SETTING_UI_SCALE,
    SETTING_SPACING,
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

/// Checkbox that reports whether the caller should [`SettingsUi::mark_dirty`].
fn dirty_checkbox(
    ui: &mut egui::Ui,
    value: &mut bool,
    label: impl Into<egui::WidgetText>,
    tip: &str,
) -> bool {
    ui.checkbox(value, label).on_hover_text(tip).changed()
}

/// Integer `num_entry` that writes back and reports whether to mark dirty.
fn dirty_i32(
    ui: &mut egui::Ui,
    value: &mut i32,
    tip: &str,
    configure: impl FnOnce(crate::widgets::NumEntry<'_, i32>) -> crate::widgets::NumEntry<'_, i32>,
) -> bool {
    let mut v = *value;
    let changed = ui
        .add(configure(crate::widgets::num_entry(&mut v)))
        .on_hover_text(tip)
        .changed();
    if changed {
        *value = v;
    }
    changed
}

fn section_visible(q: &str, section_keywords: &[&str], settings: &[&[&str]]) -> bool {
    q.is_empty()
        || query_matches(q, section_keywords)
        || settings.iter().any(|kw| query_matches(q, kw))
}

/// Fits the widest section label, leaving the same gap between the label and the
/// splitter line as between the window edge and the label (never less than the
/// label plus the pane's own inset, so it cannot wrap).
fn sidebar_width(ui: &egui::Ui) -> f32 {
    let label_w = SettingsSection::all()
        .map(|s| {
            egui::WidgetText::from(s.label())
                .into_galley(ui, None, f32::INFINITY, egui::TextStyle::Button)
                .size()
                .x
        })
        .fold(0.0, f32::max);
    let pad = ui.spacing().button_padding.x;
    let button_w = label_w + 2.0 * pad;
    let edge_gap = ui.spacing().window_margin.leftf() + pad;
    let balanced = pad + label_w + edge_gap - crate::theme::PANEL_SPLITTER_W / 2.0;
    balanced.max(button_w + crate::widgets::sections::PANE_PAD)
}

/// Tip-named Reset chrome (Settings keeps checkbox prose; action verbs are icons).
fn settings_reset_btn(ui: &mut egui::Ui, tip: &str) -> egui::Response {
    crate::widgets::icon_button(ui, egui_phosphor::regular::ARROW_COUNTER_CLOCKWISE, tip)
}

// Folder/file pickers are desktop-only; wasm drops those call sites via `cfg!`
// / `#[cfg(not(wasm))]`, so the helpers are unused on that target.
#[cfg_attr(target_arch = "wasm32", expect(dead_code))]
fn settings_choose_btn(ui: &mut egui::Ui, tip: &str) -> egui::Response {
    crate::widgets::pick_icon_button(ui, tip)
}

#[cfg_attr(target_arch = "wasm32", expect(dead_code))]
fn settings_folder_btn(ui: &mut egui::Ui, tip: &str) -> egui::Response {
    crate::widgets::icon_button(ui, egui_phosphor::regular::FOLDER_OPEN, tip)
}

/// Bold title for a group of settings within a section page.
fn subsection_heading(ui: &mut egui::Ui, first: &mut bool, title: &str) {
    if !std::mem::take(first) {
        crate::theme::add_section_gap(ui);
    }
    ui.label(egui::RichText::new(title).strong());
}

/// Collapsed "Advanced" group, forced open while a non-empty query names one of its knobs.
fn advanced_header(
    ui: &mut egui::Ui,
    id_salt: &str,
    q: &str,
    knobs: &[&[&str]],
    add_contents: impl FnOnce(&mut egui::Ui),
) {
    let force_open = !q.is_empty() && knobs.iter().any(|kw| query_matches(q, kw));
    crate::widgets::collapsible_header("Advanced", id_salt)
        .open(force_open.then_some(true))
        .show(ui, add_contents);
}

fn appearance_section_visible(q: &str) -> bool {
    section_visible(q, SECTION_APPEARANCE, APPEARANCE_SETTINGS)
        || ACTION_COLOR_CATEGORIES
            .iter()
            .any(|&(_, label)| crate::pickers::fuzzy_match_fold(q, label))
}

/// If the user picked `.sqyre` itself, use it; otherwise treat the pick as its parent.
fn resolve_sqyre_dir_from_pick(picked: PathBuf) -> PathBuf {
    if picked
        .file_name()
        .is_some_and(|n| n == sqyre_ports::APP_DIR_NAME)
    {
        picked
    } else {
        picked.join(sqyre_ports::APP_DIR_NAME)
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

    #[cfg(not(target_arch = "wasm32"))]
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
                    #[cfg(not(target_arch = "wasm32"))]
                    let mut update = crate::update::UpdateManager::default();
                    let macro_hotkeys = sqyre_hotkeys::MacroHotkeyBridge::new();
                    settings.show(
                        ui.ctx(),
                        &mut SettingsData {
                            db: &mut db,
                            macros: &mut macros,
                            catalog: &mut catalog,
                        },
                        #[cfg(not(target_arch = "wasm32"))]
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
