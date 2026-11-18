//! Runtime / search / run-safety preferences.

use serde::{Deserialize, Serialize};

pub const DEFAULT_IMAGE_SEARCH_CLOSE_MATCHES_DISTANCE: i32 = 10;
/// Skip later icon variants once one hits on the same search/placement.
pub const DEFAULT_IMAGE_SEARCH_VARIANT_EXIT_EARLY: bool = true;
pub const DEFAULT_RELEASE_HELD_INPUTS_ON_END: bool = true;
/// Key that stops a running macro (rdev key name; empty disables panic stop).
pub const DEFAULT_PANIC_KEY: &str = "esc";
/// Length of the desktop click-marker flash.
pub const DEFAULT_CLICK_FLASH_MS: i32 = 360;
pub const MIN_CLICK_FLASH_MS: i32 = 50;
pub const MAX_CLICK_FLASH_MS: i32 = 5_000;
/// Keyboard delay seeded into newly created macros.
pub const DEFAULT_MACRO_KEYBOARD_DELAY_MS: i32 = 25;
/// Mouse delay seeded into newly created macros.
pub const DEFAULT_MACRO_MOUSE_DELAY_MS: i32 = 25;
/// Idle gap (ms) that becomes a Wait action while recording.
pub const DEFAULT_RECORD_WAIT_THRESHOLD_MS: i64 = 40;
pub const MIN_RECORD_WAIT_THRESHOLD_MS: i64 = 5;
pub const MAX_RECORD_WAIT_THRESHOLD_MS: i64 = 2_000;
/// Minimum mouse travel (px) recorded as a point while recording.
pub const DEFAULT_RECORD_MOVE_MIN_DISTANCE: i32 = 8;
pub const MIN_RECORD_MOVE_MIN_DISTANCE: i32 = 1;
pub const MAX_RECORD_MOVE_MIN_DISTANCE: i32 = 100;
/// Press duration (ms) at or below which a recording emits a Tap.
pub const DEFAULT_RECORD_TAP_MAX_MS: i64 = 200;
pub const MIN_RECORD_TAP_MAX_MS: i64 = 50;
pub const MAX_RECORD_TAP_MAX_MS: i64 = 2_000;
/// Snap radius (px) to existing catalog points while recording.
pub const DEFAULT_RECORD_SNAP_POINT_PX: i32 = 24;
pub const MIN_RECORD_SNAP_POINT_PX: i32 = 0;
pub const MAX_RECORD_SNAP_POINT_PX: i32 = 200;
/// Milliseconds between mem.log memory samples.
pub const DEFAULT_MEMORY_SAMPLE_INTERVAL_MS: i32 = 5_000;
pub const MIN_MEMORY_SAMPLE_INTERVAL_MS: i32 = 250;
pub const MAX_MEMORY_SAMPLE_INTERVAL_MS: i32 = 300_000;

/// Long-edge cap (px) for images stored in the action log.
pub const DEFAULT_LOG_IMAGE_MAX_EDGE: i32 = 640;
pub const MIN_LOG_IMAGE_MAX_EDGE: i32 = 64;
pub const MAX_LOG_IMAGE_MAX_EDGE: i32 = 8192;

/// Default While budget when a macro sets `max_iterations` ≤ 0.
pub const DEFAULT_WHILE_MAX_ITERATIONS: i32 = 100_000;
pub const MIN_WHILE_MAX_ITERATIONS: i32 = 1;
pub const MAX_WHILE_MAX_ITERATIONS: i32 = 10_000_000;
/// Default max nested RunMacro depth (including the top-level macro).
pub const DEFAULT_RUN_MACRO_MAX_DEPTH: i32 = 32;
pub const MIN_RUN_MACRO_MAX_DEPTH: i32 = 1;
pub const MAX_RUN_MACRO_MAX_DEPTH: i32 = 256;

/// Log entries kept per action in the Logs window (oldest dropped).
pub const DEFAULT_ACTION_LOG_MAX_ENTRIES: i32 = 5_000;
pub const MIN_ACTION_LOG_MAX_ENTRIES: i32 = 50;
pub const MAX_ACTION_LOG_MAX_ENTRIES: i32 = 50_000;

/// Soft cap for Rayon / search worker threads (used when the machine has more cores).
pub const DEFAULT_WORKER_THREADS_CAP: i32 = 16;
pub const MIN_WORKER_THREADS: i32 = 1;

pub fn available_worker_threads() -> i32 {
    std::thread::available_parallelism()
        .map(|n| n.get() as i32)
        .unwrap_or(1)
        .max(MIN_WORKER_THREADS)
}

/// Default worker-thread count: `min(16, available CPUs)`.
pub fn default_worker_threads() -> i32 {
    available_worker_threads().min(DEFAULT_WORKER_THREADS_CAP)
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HotkeyWhileRunning {
    /// Extra presses are dropped while the macro runs.
    #[default]
    Ignore,
    /// Pressing the hotkey again stops the run.
    Stop,
    /// Pressing the hotkey again restarts the macro.
    Restart,
}

impl HotkeyWhileRunning {
    /// All selectable values, in dropdown order.
    pub const ALL: [Self; 3] = [Self::Ignore, Self::Stop, Self::Restart];

    /// Dropdown display name.
    pub fn label(self) -> &'static str {
        match self {
            Self::Ignore => "Ignore",
            Self::Stop => "Stop the run",
            Self::Restart => "Restart",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RuntimePrefs {
    #[serde(default)]
    pub save_meta_images: bool,

    /// Log entries kept per action in the Logs window (oldest dropped).
    #[serde(default = "default_action_log_max_entries")]
    pub action_log_max_entries: i32,

    #[serde(default)]
    pub highlight_active_action: bool,

    /// Flash a marker on the desktop where each automated mouse press lands.
    #[serde(default)]
    pub click_effect: bool,

    /// Release keys/buttons still held from Down/hold actions when a macro ends.
    #[serde(default = "default_release_held_inputs_on_end")]
    pub release_held_inputs_on_end: bool,

    /// Safety budget for While actions with `max_iterations` ≤ 0.
    #[serde(default = "default_while_max_iterations")]
    pub while_max_iterations: i32,

    /// Max nested RunMacro depth (including the top-level macro).
    #[serde(default = "default_run_macro_max_depth")]
    pub run_macro_max_depth: i32,

    #[serde(default = "default_close_matches")]
    pub image_search_close_matches_distance: i32,

    /// When true, stop trying later icon variants after one hits on the same search.
    #[serde(default = "default_variant_exit_early")]
    pub image_search_variant_exit_early: bool,

    /// Per-action-type blank templates for the Add Action picker (YAML action maps).
    #[serde(default, skip_serializing_if = "std::collections::BTreeMap::is_empty")]
    pub action_defaults: std::collections::BTreeMap<String, serde_yaml::Mapping>,

    /// Active macro-list hotkey tag filters (empty = no hotkeys; `""` entry = untagged).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub hotkey_tag_filters: Vec<String>,

    /// When true, focusing a Program sets `hotkey_tag_filters` to that program's macro tags,
    /// which always include its name (cleared when no Program owns focus). When false, filters
    /// are manual only.
    #[serde(default)]
    pub hotkey_tags_while_focused: bool,

    /// Name of the macro selected when Sqyre last ran; reselected on startup.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub last_selected_macro: String,

    /// Rayon worker threads for image search, find-pixel, and capture conversion.
    /// Applied at process start (`build_global`); changing it needs a restart.
    #[serde(default = "default_worker_threads")]
    pub worker_threads: i32,

    /// First-start permissions popup was confirmed; later launches skip it.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub permissions_intro_done: bool,

    /// Write diag.log diagnostics to the data directory.
    #[serde(default)]
    pub diagnostic_logging: bool,

    /// Sleep Wayland screen sharing after the desktop sits still.
    /// The next capture shows a loading icon while the stream wakes.
    #[serde(default = "default_suspend_screen_capture")]
    pub suspend_screen_capture_when_idle: bool,

    /// Write mem.log memory samples to the data directory.
    #[serde(default)]
    pub memory_logging: bool,

    /// Milliseconds between mem.log samples when `memory_logging` is on.
    #[serde(default = "default_memory_sample_interval")]
    pub memory_logging_interval_ms: i32,

    /// Reselect the macro that was selected on exit.
    #[serde(default = "default_restore_last_macro")]
    pub restore_last_macro: bool,

    /// Move the cursor back to its pre-run position when a macro ends.
    #[serde(default)]
    pub restore_cursor_on_end: bool,

    /// Click-marker flash length in milliseconds.
    #[serde(default = "default_click_flash_ms")]
    pub click_flash_ms: i32,

    /// Long-edge cap (px) for images stored in the action log.
    #[serde(default = "default_log_image_max_edge")]
    pub log_image_max_edge: i32,

    /// Keyboard delay seeded into newly created macros.
    #[serde(default = "default_macro_keyboard_delay")]
    pub default_keyboard_delay_ms: i32,

    /// Mouse delay seeded into newly created macros.
    #[serde(default = "default_macro_mouse_delay")]
    pub default_mouse_delay_ms: i32,

    /// Single key that stops a running macro (rdev name like `esc`, `f12`;
    /// empty disables panic stop).
    #[serde(default = "default_panic_key_string")]
    pub panic_key: String,

    /// What a running macro's hotkey does on repeat presses.
    #[serde(default)]
    pub hotkey_while_running: HotkeyWhileRunning,

    /// `press`/`release` trigger seeded into newly created macros.
    #[serde(default = "default_hotkey_trigger")]
    pub default_hotkey_trigger: String,

    /// Idle gap (ms) that becomes a Wait action while recording.
    #[serde(default = "default_record_wait_threshold")]
    pub record_wait_threshold_ms: i64,

    /// Minimum mouse travel (px) recorded as a point while recording.
    #[serde(default = "default_record_move_min_distance")]
    pub record_move_min_distance: i32,

    /// Press duration (ms) at or below which a recording emits a Tap.
    #[serde(default = "default_record_tap_max")]
    pub record_tap_max_ms: i64,

    /// Snap radius (px) to existing catalog points while recording.
    #[serde(default = "default_record_snap_point")]
    pub record_snap_point_px: i32,
}

fn default_release_held_inputs_on_end() -> bool {
    DEFAULT_RELEASE_HELD_INPUTS_ON_END
}
fn default_action_log_max_entries() -> i32 {
    DEFAULT_ACTION_LOG_MAX_ENTRIES
}
fn default_while_max_iterations() -> i32 {
    DEFAULT_WHILE_MAX_ITERATIONS
}
fn default_run_macro_max_depth() -> i32 {
    DEFAULT_RUN_MACRO_MAX_DEPTH
}
fn default_close_matches() -> i32 {
    DEFAULT_IMAGE_SEARCH_CLOSE_MATCHES_DISTANCE
}
fn default_variant_exit_early() -> bool {
    DEFAULT_IMAGE_SEARCH_VARIANT_EXIT_EARLY
}
fn default_restore_last_macro() -> bool {
    true
}
fn default_memory_sample_interval() -> i32 {
    DEFAULT_MEMORY_SAMPLE_INTERVAL_MS
}
fn default_log_image_max_edge() -> i32 {
    DEFAULT_LOG_IMAGE_MAX_EDGE
}
fn default_click_flash_ms() -> i32 {
    DEFAULT_CLICK_FLASH_MS
}
fn default_macro_keyboard_delay() -> i32 {
    DEFAULT_MACRO_KEYBOARD_DELAY_MS
}
fn default_macro_mouse_delay() -> i32 {
    DEFAULT_MACRO_MOUSE_DELAY_MS
}
fn default_panic_key_string() -> String {
    DEFAULT_PANIC_KEY.to_string()
}
fn default_hotkey_trigger() -> String {
    sqyre_domain::HOTKEY_TRIGGER_PRESS.to_string()
}
fn default_record_wait_threshold() -> i64 {
    DEFAULT_RECORD_WAIT_THRESHOLD_MS
}
fn default_record_move_min_distance() -> i32 {
    DEFAULT_RECORD_MOVE_MIN_DISTANCE
}
fn default_record_tap_max() -> i64 {
    DEFAULT_RECORD_TAP_MAX_MS
}
fn default_record_snap_point() -> i32 {
    DEFAULT_RECORD_SNAP_POINT_PX
}
fn default_suspend_screen_capture() -> bool {
    true
}

impl Default for RuntimePrefs {
    fn default() -> Self {
        Self {
            save_meta_images: false,
            action_log_max_entries: DEFAULT_ACTION_LOG_MAX_ENTRIES,
            highlight_active_action: false,
            click_effect: false,
            release_held_inputs_on_end: DEFAULT_RELEASE_HELD_INPUTS_ON_END,
            while_max_iterations: DEFAULT_WHILE_MAX_ITERATIONS,
            run_macro_max_depth: DEFAULT_RUN_MACRO_MAX_DEPTH,
            image_search_close_matches_distance: DEFAULT_IMAGE_SEARCH_CLOSE_MATCHES_DISTANCE,
            image_search_variant_exit_early: DEFAULT_IMAGE_SEARCH_VARIANT_EXIT_EARLY,
            action_defaults: std::collections::BTreeMap::new(),
            hotkey_tag_filters: Vec::new(),
            hotkey_tags_while_focused: false,
            last_selected_macro: String::new(),
            worker_threads: default_worker_threads(),
            permissions_intro_done: false,
            diagnostic_logging: false,
            suspend_screen_capture_when_idle: true,
            memory_logging: false,
            memory_logging_interval_ms: DEFAULT_MEMORY_SAMPLE_INTERVAL_MS,
            restore_last_macro: true,
            restore_cursor_on_end: false,
            click_flash_ms: DEFAULT_CLICK_FLASH_MS,
            log_image_max_edge: DEFAULT_LOG_IMAGE_MAX_EDGE,
            default_keyboard_delay_ms: DEFAULT_MACRO_KEYBOARD_DELAY_MS,
            default_mouse_delay_ms: DEFAULT_MACRO_MOUSE_DELAY_MS,
            panic_key: DEFAULT_PANIC_KEY.to_string(),
            hotkey_while_running: HotkeyWhileRunning::default(),
            default_hotkey_trigger: sqyre_domain::HOTKEY_TRIGGER_PRESS.to_string(),
            record_wait_threshold_ms: DEFAULT_RECORD_WAIT_THRESHOLD_MS,
            record_move_min_distance: DEFAULT_RECORD_MOVE_MIN_DISTANCE,
            record_tap_max_ms: DEFAULT_RECORD_TAP_MAX_MS,
            record_snap_point_px: DEFAULT_RECORD_SNAP_POINT_PX,
        }
    }
}

impl RuntimePrefs {
    /// Clamp numeric ranges to settings UI bounds.
    pub fn clamp(&mut self) {
        self.image_search_close_matches_distance =
            self.image_search_close_matches_distance.clamp(0, 100);
        if self.while_max_iterations <= 0 {
            self.while_max_iterations = DEFAULT_WHILE_MAX_ITERATIONS;
        }
        self.while_max_iterations = self
            .while_max_iterations
            .clamp(MIN_WHILE_MAX_ITERATIONS, MAX_WHILE_MAX_ITERATIONS);
        if self.run_macro_max_depth <= 0 {
            self.run_macro_max_depth = DEFAULT_RUN_MACRO_MAX_DEPTH;
        }
        self.run_macro_max_depth = self
            .run_macro_max_depth
            .clamp(MIN_RUN_MACRO_MAX_DEPTH, MAX_RUN_MACRO_MAX_DEPTH);
        if self.action_log_max_entries <= 0 {
            self.action_log_max_entries = DEFAULT_ACTION_LOG_MAX_ENTRIES;
        }
        self.action_log_max_entries = self
            .action_log_max_entries
            .clamp(MIN_ACTION_LOG_MAX_ENTRIES, MAX_ACTION_LOG_MAX_ENTRIES);
        if self.worker_threads <= 0 {
            self.worker_threads = default_worker_threads();
        }
        self.worker_threads = self
            .worker_threads
            .clamp(MIN_WORKER_THREADS, available_worker_threads());
        if self.click_flash_ms <= 0 {
            self.click_flash_ms = DEFAULT_CLICK_FLASH_MS;
        }
        self.click_flash_ms = self
            .click_flash_ms
            .clamp(MIN_CLICK_FLASH_MS, MAX_CLICK_FLASH_MS);
        self.log_image_max_edge = self
            .log_image_max_edge
            .clamp(MIN_LOG_IMAGE_MAX_EDGE, MAX_LOG_IMAGE_MAX_EDGE);
        self.default_keyboard_delay_ms = self.default_keyboard_delay_ms.clamp(0, 60_000);
        self.default_mouse_delay_ms = self.default_mouse_delay_ms.clamp(0, 60_000);
        self.panic_key = self.panic_key.trim().to_ascii_lowercase();
        self.default_hotkey_trigger =
            sqyre_domain::Macro::parse_hotkey_trigger(&self.default_hotkey_trigger);
        if self.record_wait_threshold_ms < MIN_RECORD_WAIT_THRESHOLD_MS {
            self.record_wait_threshold_ms = DEFAULT_RECORD_WAIT_THRESHOLD_MS;
        }
        self.record_wait_threshold_ms = self
            .record_wait_threshold_ms
            .clamp(MIN_RECORD_WAIT_THRESHOLD_MS, MAX_RECORD_WAIT_THRESHOLD_MS);
        if self.record_move_min_distance < MIN_RECORD_MOVE_MIN_DISTANCE {
            self.record_move_min_distance = DEFAULT_RECORD_MOVE_MIN_DISTANCE;
        }
        self.record_move_min_distance = self
            .record_move_min_distance
            .clamp(MIN_RECORD_MOVE_MIN_DISTANCE, MAX_RECORD_MOVE_MIN_DISTANCE);
        if self.record_tap_max_ms < MIN_RECORD_TAP_MAX_MS {
            self.record_tap_max_ms = DEFAULT_RECORD_TAP_MAX_MS;
        }
        self.record_tap_max_ms = self
            .record_tap_max_ms
            .clamp(MIN_RECORD_TAP_MAX_MS, MAX_RECORD_TAP_MAX_MS);
        self.record_snap_point_px = self
            .record_snap_point_px
            .clamp(MIN_RECORD_SNAP_POINT_PX, MAX_RECORD_SNAP_POINT_PX);
        self.memory_logging_interval_ms = self
            .memory_logging_interval_ms
            .clamp(MIN_MEMORY_SAMPLE_INTERVAL_MS, MAX_MEMORY_SAMPLE_INTERVAL_MS);
    }
}
