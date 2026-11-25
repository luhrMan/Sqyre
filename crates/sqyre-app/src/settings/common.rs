//! Shared Settings UI helpers and search keywords.

use eframe::egui;
use sqyre_domain::ACTION_COLOR_CATEGORIES;
use std::path::PathBuf;

use super::SettingsSection;

pub(crate) const SECTION_GENERAL: &[&str] = &["general", "application", "behavior"];
pub(crate) const SECTION_MACROS: &[&str] = &["macros", "macro"];
pub(crate) const SUBSECTION_PLAYBACK: &[&str] = &["playback", "running", "execution"];
pub(crate) const SUBSECTION_RECORDING: &[&str] = &["recording", "record"];
pub(crate) const SUBSECTION_HOTKEYS: &[&str] = &["hotkeys", "hotkey", "shortcuts"];
pub(crate) const SUBSECTION_IMAGE_SEARCH: &[&str] = &["image search", "detection"];
pub(crate) const SECTION_SOUND: &[&str] = &["sound", "audio", "cue"];
#[cfg(feature = "native-runtime")]
pub(crate) const SECTION_PERMISSIONS: &[&str] = &[
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
pub(crate) const SECTION_DATA: &[&str] = &["data", "folder", "backup", "restore", "archive"];

pub(crate) const SECTION_UPDATES: &[&str] = &["updates", "github", "release", "version"];
pub(crate) const SECTION_APPEARANCE: &[&str] = &["appearance", "theme", "display"];

pub(crate) const SETTING_LOG_META: &[&str] = &[
    "log meta images",
    "meta images",
    "ocr",
    "debug frames",
    "memory",
    "action log",
    "log entries",
    "log cap",
];
pub(crate) const SETTING_HIGHLIGHT_ACTION: &[&str] =
    &["highlight", "executing action", "active action", "tint"];
pub(crate) const SETTING_CLICK_FX: &[&str] = &[
    "click effect",
    "click",
    "flash",
    "marker",
    "cursor",
    "clicks land",
];
pub(crate) const SETTING_HIDE_WHILE_RECORDING: &[&str] =
    &["hide", "recording", "points", "search areas", "snapshot"];
pub(crate) const SETTING_RELEASE_HELD: &[&str] = &[
    "release",
    "held keys",
    "held buttons",
    "inputs",
    "macro ends",
];
pub(crate) const SETTING_OVERLAY_PRESS_STOPS: &[&str] =
    &["overlay", "overlay button", "stop macro", "press"];
pub(crate) const SETTING_OVERLAY_IN_DETECTION: &[&str] = &[
    "overlay",
    "overlay button",
    "detection",
    "image search",
    "find pixel",
    "ocr",
];
pub(crate) const SETTING_OVERLAY_OVER_SQYRE: &[&str] = &[
    "overlay",
    "overlay button",
    "over sqyre",
    "on top",
    "show overlay",
    "sqyre window",
];
pub(crate) const SETTING_HOTKEY_TAGS_FOCUSED: &[&str] = &[
    "hotkey",
    "hotkeys",
    "tags",
    "focused",
    "focus",
    "program",
    "while focused",
];
#[cfg(feature = "native-runtime")]
pub(crate) const SETTING_WORKER_THREADS: &[&str] = &[
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
pub(crate) const SETTING_WHILE_BUDGET: &[&str] = &[
    "while",
    "safety budget",
    "iterations",
    "loop",
    "max_iterations",
    "endless",
    "stop endless",
    "advanced",
];
pub(crate) const SETTING_RUN_MACRO_DEPTH: &[&str] = &[
    "run macro",
    "nesting",
    "depth",
    "recursion",
    "nested",
    "advanced",
];
pub(crate) const SETTING_IMAGE_SEARCH_DISTANCE: &[&str] = &[
    "image search",
    "close-match",
    "close match",
    "distance",
    "duplicate",
    "nearby",
    "advanced",
];
pub(crate) const SETTING_VARIANT_EXIT_EARLY: &[&str] = &[
    "variant",
    "variants",
    "exit early",
    "early exit",
    "first match",
    "image search",
    "icon variants",
];

pub(crate) const SETTING_FINISH_SOUND: &[&str] = &["finish sound", "macro finishes", "complete"];
pub(crate) const SETTING_UI_SOUNDS: &[&str] = &["ui sounds", "adding", "deleting", "add/delete"];
pub(crate) const SETTING_SOUND_VOLUME: &[&str] = &["volume", "sound volume"];

pub(crate) const SETTING_SOUND_FILES: &[&str] =
    &["sound file", "custom sound", "mp3", "cue file", "built-in"];

#[cfg(feature = "native-runtime")]
pub(crate) const SETTING_DIAGNOSTICS: &[&str] = &[
    "diagnostic",
    "diagnostics",
    "diag",
    "diag.log",
    "debug",
    "troubleshoot",
    "crash",
    "log file",
];
#[cfg(feature = "native-runtime")]
pub(crate) const SETTING_MEMORY_LOG: &[&str] = &[
    "memory",
    "memory log",
    "mem.log",
    "rss",
    "leak",
    "sample",
    "interval",
];

pub(crate) const SETTING_CLOSE_TO_TRAY: &[&str] =
    &["close", "close to tray", "tray", "minimize", "quit", "exit"];

pub(crate) const SETTING_START_MINIMIZED: &[&str] =
    &["start minimized", "tray", "startup", "launch", "hidden"];
#[cfg(target_os = "linux")]
pub(crate) const SETTING_AUTOSTART: &[&str] =
    &["autostart", "launch on login", "login", "boot", "startup"];
#[cfg(all(target_os = "linux", feature = "native-runtime"))]
pub(crate) const SETTING_SUSPEND_CAPTURE: &[&str] = &[
    "sleep",
    "idle",
    "screen capture",
    "screen sharing",
    "wake",
    "suspend",
    "pipewire",
];
pub(crate) const SETTING_LAST_MACRO: &[&str] =
    &["restore", "last macro", "remember", "reopen", "selected"];

pub(crate) const SETTING_RESTORE_CURSOR: &[&str] =
    &["cursor", "restore cursor", "mouse position", "restore"];
pub(crate) const SETTING_CLICK_FLASH_MS: &[&str] = &[
    "click",
    "flash",
    "marker",
    "duration",
    "milliseconds",
    "advanced",
];
pub(crate) const SETTING_RECORD_THRESHOLDS: &[&str] = &[
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

pub(crate) const SETTING_PANIC_KEY: &[&str] = &[
    "panic",
    "panic key",
    "stop key",
    "esc",
    "escape",
    "abort",
    "emergency",
    "record",
];
pub(crate) const SETTING_RETRIGGER: &[&str] = &[
    "hotkey",
    "while running",
    "restart",
    "retrigger",
    "already running",
    "running macro",
];
pub(crate) const SETTING_DEFAULT_TRIGGER: &[&str] =
    &["hotkey", "trigger", "press", "release", "new macro", "fire"];

pub(crate) const SUBSECTION_ACTION_DEFAULTS: &[&str] =
    &["defaults", "new actions", "new macros", "action defaults"];
pub(crate) const SETTING_DEFAULT_DELAYS: &[&str] = &[
    "delay",
    "delays",
    "keyboard",
    "mouse",
    "new macro",
    "default",
];
pub(crate) const SETTING_LOG_IMAGE_EDGE: &[&str] = &[
    "log image",
    "max edge",
    "image size",
    "meta image",
    "annotated",
    "advanced",
];
pub(crate) const SETTING_ACTION_DEFAULTS_RESET: &[&str] = &[
    "reset",
    "saved defaults",
    "action defaults",
    "save as default",
];

pub(crate) const SETTING_BACKUP_DIR: &[&str] = &[
    "backup folder",
    "backup directory",
    "backups folder",
    "destination",
    "backup location",
];

pub(crate) const SETTING_UPDATE_INTERVAL: &[&str] = &[
    "interval",
    "check every",
    "hours",
    "frequency",
    "update check",
];

pub(crate) const SETTING_UPDATE_PRERELEASE: &[&str] = &[
    "prerelease",
    "pre-release",
    "beta",
    "release candidate",
    "rc",
];

pub(crate) const SETTING_ACCENT: &[&str] = &[
    "accent",
    "accent color",
    "gold",
    "theme color",
    "highlight color",
];
pub(crate) const SETTING_FONT_FAMILY: &[&str] =
    &["font family", "monospace", "font", "typeface", "hack"];
pub(crate) const SETTING_MONO_SIZE: &[&str] =
    &["mono", "monospace", "code font", "font size", "yaml"];

#[cfg(feature = "native-runtime")]
pub(crate) const PERMISSIONS_PANEL: &[&str] = &[
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

pub(crate) const DATA_LOCATION: &[&str] = &[
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
pub(crate) const DATA_BACKUP: &[&str] = &[
    "backup",
    "automatic backups",
    "interval",
    "keep at most",
    "restore",
    "archive",
    "zip",
];

pub(crate) const SETTING_UPDATE_VERSION: &[&str] = &["current version", "version"];

pub(crate) const SETTING_AUTO_UPDATE: &[&str] =
    &["check for updates", "startup", "auto update", "automatic"];

pub(crate) const SETTING_UPDATE_ACTIONS: &[&str] = &[
    "check now",
    "download",
    "install",
    "restart",
    "update status",
];

pub(crate) const SETTING_COMPACT_HEADERS: &[&str] =
    &["compact", "program headers", "icons", "headers"];
pub(crate) const SETTING_EMPTY_HEADERS: &[&str] = &["empty", "program headers", "headers", "hide"];
pub(crate) const SETTING_WINDOW_BORDER: &[&str] = &["window border", "border", "outline"];

pub(crate) const SETTING_WINDOW_DECORATIONS: &[&str] = &[
    "os title bar",
    "title bar",
    "decorations",
    "borderless",
    "frameless",
    "window chrome",
];
pub(crate) const SETTING_RUN_GLOW: &[&str] = &["run button", "glow", "pulse", "animation"];
pub(crate) const SETTING_PREVIEW_OUTLINE: &[&str] = &[
    "preview",
    "preview outline",
    "desktop outline",
    "location",
    "location box",
    "tooltip",
    "monitor",
    "gold box",
];
pub(crate) const SETTING_FONT_SIZE: &[&str] = &["font size", "font", "text size"];
pub(crate) const SETTING_UI_SCALE: &[&str] = &["ui scale", "scale", "padding"];
pub(crate) const SETTING_SPACING: &[&str] = &[
    "spacing", "density", "padding", "margin", "gap", "compact", "spaced",
];
pub(crate) const SETTING_ACTION_COLORS: &[&str] =
    &["action colors", "macro tree", "colors", "colour", "pastel"];

pub(crate) const GENERAL_SETTINGS: &[&[&str]] = &[
    SETTING_LOG_META,
    #[cfg(feature = "native-runtime")]
    SETTING_WORKER_THREADS,
    SETTING_LAST_MACRO,
    SETTING_CLOSE_TO_TRAY,
    SETTING_START_MINIMIZED,
    #[cfg(target_os = "linux")]
    SETTING_AUTOSTART,
    #[cfg(all(target_os = "linux", feature = "native-runtime"))]
    SETTING_SUSPEND_CAPTURE,
    #[cfg(feature = "native-runtime")]
    SETTING_DIAGNOSTICS,
    #[cfg(feature = "native-runtime")]
    SETTING_MEMORY_LOG,
];
pub(crate) const MACROS_SETTINGS: &[&[&str]] = &[
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
pub(crate) const SOUND_SETTINGS: &[&[&str]] = &[
    SETTING_FINISH_SOUND,
    SETTING_UI_SOUNDS,
    SETTING_SOUND_VOLUME,
    SETTING_SOUND_FILES,
];
#[cfg(feature = "native-runtime")]
pub(crate) const PERMISSIONS_SETTINGS: &[&[&str]] = &[PERMISSIONS_PANEL];
pub(crate) const DATA_SETTINGS: &[&[&str]] = &[DATA_LOCATION, DATA_BACKUP, SETTING_BACKUP_DIR];

pub(crate) const UPDATES_SETTINGS: &[&[&str]] = &[
    SETTING_UPDATE_VERSION,
    SETTING_AUTO_UPDATE,
    SETTING_UPDATE_ACTIONS,
    SETTING_UPDATE_INTERVAL,
    SETTING_UPDATE_PRERELEASE,
];
pub(crate) const APPEARANCE_SETTINGS: &[&[&str]] = &[
    SETTING_COMPACT_HEADERS,
    SETTING_EMPTY_HEADERS,
    SETTING_WINDOW_BORDER,
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

pub(crate) fn query_matches(q: &str, keywords: &[&str]) -> bool {
    keywords
        .iter()
        .any(|k| crate::pickers::fuzzy_match_fold(q, k))
}

pub(crate) fn setting_visible(q: &str, section_hit: bool, keywords: &[&str]) -> bool {
    q.is_empty() || section_hit || query_matches(q, keywords)
}

/// Checkbox that reports whether the caller should [`SettingsUi::mark_dirty`].
pub(crate) fn dirty_checkbox(
    ui: &mut egui::Ui,
    value: &mut bool,
    label: impl Into<egui::WidgetText>,
    tip: &str,
) -> bool {
    ui.checkbox(value, label).on_hover_text(tip).changed()
}

/// Integer `num_entry` that writes back and reports whether to mark dirty.
pub(crate) fn dirty_i32(
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

pub(crate) fn section_visible(q: &str, section_keywords: &[&str], settings: &[&[&str]]) -> bool {
    q.is_empty()
        || query_matches(q, section_keywords)
        || settings.iter().any(|kw| query_matches(q, kw))
}

/// Fits the widest section label, leaving the same gap between the label and the
/// splitter line as between the window edge and the label (never less than the
/// label plus the pane's own inset, so it cannot wrap).
pub(crate) fn sidebar_width(ui: &egui::Ui) -> f32 {
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
pub(crate) fn settings_reset_btn(ui: &mut egui::Ui, tip: &str) -> egui::Response {
    crate::widgets::icon_button(ui, egui_phosphor::regular::ARROW_COUNTER_CLOCKWISE, tip)
}

// Folder/file pickers are desktop-only; wasm drops those call sites via `cfg!`
// / `#[cfg(not(wasm))]`, so the helpers are unused on that target.

pub(crate) fn settings_choose_btn(ui: &mut egui::Ui, tip: &str) -> egui::Response {
    crate::widgets::pick_icon_button(ui, tip)
}

pub(crate) fn settings_folder_btn(ui: &mut egui::Ui, tip: &str) -> egui::Response {
    crate::widgets::icon_button(ui, egui_phosphor::regular::FOLDER_OPEN, tip)
}

/// Bold title for a group of settings within a section page.
pub(crate) fn subsection_heading(ui: &mut egui::Ui, first: &mut bool, title: &str) {
    if !std::mem::take(first) {
        crate::theme::add_section_gap(ui);
    }
    ui.label(egui::RichText::new(title).strong());
}

/// Collapsed "Advanced" group, forced open while a non-empty query names one of its knobs.
pub(crate) fn advanced_header(
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

pub(crate) fn appearance_section_visible(q: &str) -> bool {
    section_visible(q, SECTION_APPEARANCE, APPEARANCE_SETTINGS)
        || ACTION_COLOR_CATEGORIES
            .iter()
            .any(|&(_, label)| crate::pickers::fuzzy_match_fold(q, label))
}

/// If the user picked `.sqyre` itself, use it; otherwise treat the pick as its parent.
pub(crate) fn resolve_sqyre_dir_from_pick(picked: PathBuf) -> PathBuf {
    if picked
        .file_name()
        .is_some_and(|n| n == sqyre_ports::APP_DIR_NAME)
    {
        picked
    } else {
        picked.join(sqyre_ports::APP_DIR_NAME)
    }
}
