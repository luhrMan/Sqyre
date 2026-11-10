//! Static hover/tooltip strings for app chrome outside the action editor.
//!
//! Action-editor field help has its own registry in
//! [`crate::action_tooltip::help`] (`h::`); this module is the same idea for
//! window, panel, picker, and shared-widget tooltips so the strings live in
//! one place instead of being inlined at each control. Dynamic tips
//! (`format!`, state-dependent text) stay inline at the call site.
//!
//! Convention: `ht::CONST`, one short sentence per string, grouped by surface.

// --- Number or variable fields ---

/// Appended to a number-or-variable field's hover help.
pub const NUM_OR_VAR_SWITCH: &str = "Right-click to switch between a number and a variable.";
/// Menu entry: show the numeric stepper.
pub const NUM_OR_VAR_NUMBER: &str = "Edit as a number.";
/// Menu entry when the current text is not a plain number.
pub const NUM_OR_VAR_NUMBER_REPLACE: &str = "Replace this with a number.";
/// Menu entry: show the variable text field.
pub const NUM_OR_VAR_VARIABLE: &str = "Edit as a variable or expression.";

// --- Action edit tooltip ---

/// Close button on the pinned action editor.
pub const EDIT_TIP_CLOSE: &str = "Close the editor without saving";

// --- Action logs ---

/// Copy button in the logs header.
pub const LOGS_COPY_FILTERED: &str = "Copy the filtered log lines";
/// Item row: opens the per-item detail view.
pub const LOGS_ITEM_STEPS: &str = "Click to view processing steps and where it was found";
/// Detail view navigation.
pub const LOGS_PREV_ITEM: &str = "Previous item (←)";
/// Detail view navigation.
pub const LOGS_NEXT_ITEM: &str = "Next item (→)";
/// Context screenshot thumbnail.
pub const LOGS_ZOOM_TIP: &str = "Click to zoom — wheel zooms, drag pans";
/// Zoom view reset (0 is the keyboard shortcut).
pub const LOGS_FIT_VIEW: &str = "Fit image in viewport (0)";

// --- Data editor ---

/// Mask upload button on the coordinate form.
pub const ED_UPLOAD_IMAGE: &str = "Replace this mask with a PNG from disk.";
/// Mask remove-image icon on the Masks form.
pub const ED_REMOVE_MASK_IMAGE: &str =
    "Delete the PNG and use shape geometry instead. Image mask mode hides shape fields while a PNG is on disk.";
/// Window pick control on the program form.
pub const ED_PICK_WINDOW: &str = "Pick window…";
/// Context menu on a disabled list row.
pub const ED_ROW_DISABLED: &str = "Disabled — right-click to enable.";
/// Split-pane divider.
pub const ED_RESIZE_PANES: &str = "Drag to resize panes — Left/Right arrows when focused";
/// Record floater on the point preview — arm a one-click X/Y capture.
pub const ED_REC_POINT: &str = "Click on screen to capture X/Y";
/// Record floater on the search-area preview — arm a two-click area capture.
pub const ED_REC_AREA: &str = "Two clicks: opposite corners of the area";
/// Record floater on the capture-region preview — arm a two-click capture.
pub const ED_REC_CAPTURE: &str = "Two clicks: opposite corners of the capture region";
/// Record floater on the pixel-check preview — arm a two-click area capture.
#[cfg(all(not(target_arch = "wasm32"), feature = "native-runtime"))]
pub const ED_REC_SEARCH: &str = "Two clicks: opposite corners of the search area";
/// "Add running programs" icon right of the programs search bar.
#[cfg(all(not(target_arch = "wasm32"), feature = "native-runtime"))]
pub const ED_ADD_ALL_PROGRAMS: &str =
    "Add every program with an open window that has an icon, named after its process. Programs already in the list are skipped.";

// --- Prompt builder ---

/// Generate button for the AI prompt draft.
pub const PROMPT_GENERATE: &str = "Build a schema-aware prompt for an external AI tool";
/// Copy the generated prompt to the clipboard.
pub const PROMPT_COPY: &str = "Copy prompt";
/// Validate pasted AI YAML before import.
pub const PROMPT_VALIDATE: &str = "Validate pasted YAML";
/// Import button for pasted YAML.
pub const PROMPT_IMPORT: &str = "Decode, validate, and add as a new macro";

// --- Macro recording review ---

/// Toggle controlling whether copied/saved steps are also cataloged.
pub const REC_SAVE_CATALOG: &str = "Save into catalog on Copy / Save selected";
/// Recorded click coordinates.
pub const REC_SCREEN_X: &str = "Screen X (physical pixels)";
/// Recorded click coordinates.
pub const REC_SCREEN_Y: &str = "Screen Y (physical pixels)";

// --- YAML builder ---

/// Check YAML against the macro schema.
pub const YAML_VALIDATE: &str = "Validate YAML";
/// Discard edits and reload the tree's current macro.
pub const YAML_RELOAD_TREE: &str = "Reload from Tree";
/// Restore the saved draft that no longer matches the tree.
pub const YAML_RESTORE_DRAFT: &str = "Restore Draft";
/// Import as a new macro.
pub const YAML_IMPORT_NEW: &str = "Add this YAML as a new macro";
/// Import over the selected macro.
pub const YAML_IMPORT_REPLACE: &str = "Replace the selected macro with this YAML";

// --- Permissions ---

/// Refresh button in the desktop panel.
#[cfg(all(
    not(target_arch = "wasm32"),
    not(target_os = "android"),
    feature = "native-runtime"
))]
pub const PERM_REFRESH: &str = "Re-run capability checks (portal dialogs may appear on Wayland).";
/// Open the OS shortcut settings for a hotkey conflict.
#[cfg(all(
    not(target_arch = "wasm32"),
    not(target_os = "android"),
    feature = "native-runtime"
))]
pub const PERM_SHORTCUT_SETTINGS: &str =
    "Open your desktop's shortcut settings to choose the keys that run your macros.";
/// Share-screen button under Wayland.
#[cfg(all(
    not(target_arch = "wasm32"),
    not(target_os = "android"),
    feature = "native-runtime"
))]
pub const PERM_PORTAL_PICKER: &str =
    "Open the desktop portal picker to choose which screens Sqyre can capture.";
/// Revoke the saved portal grant.
#[cfg(all(
    not(target_arch = "wasm32"),
    not(target_os = "android"),
    feature = "native-runtime"
))]
pub const PERM_FORGET_GRANT: &str =
    "Stop capturing and forget the saved portal grant. Sqyre will ask again the next time it needs screen access.";

// --- Pickers ---

/// Zoom view reset (no keyboard shortcut here).
pub const PICK_FIT_VIEW: &str = "Fit image in viewport";
/// Select-all toggle above a filtered grid.
pub const PICK_SELECT_ALL: &str =
    "Select all visible items, or deselect all if all visible are selected";
/// Screen-pick retry using the previous app.
#[cfg(all(target_os = "android", feature = "native-runtime"))]
pub const PICK_RETAKE_APP: &str = "Switch to the previous app and take a new picture of it";
/// App dropdown when no previous screen has been saved.
#[cfg(any(all(target_os = "android", feature = "native-runtime"), test))]
pub const PICK_CHOOSE_APP: &str = "Choose which app to take a screenshot of";

// --- Overlay panel ---

/// Android capture permission prompt buttons.
#[cfg(all(target_os = "android", feature = "native-runtime"))]
pub const OVL_REC_PROMPT: &str = "Show Android's screen recording prompt.";
/// Retry variant shown after a dismiss.
#[cfg(all(target_os = "android", feature = "native-runtime"))]
pub const OVL_REC_PROMPT_AGAIN: &str = "Show Android's screen recording prompt again.";

// --- Macro list ---

/// Row context menu.
pub const MLIST_ROW_OPTIONS: &str = "Right-click for options.";
/// Search field hover.
pub const MLIST_SEARCH_TIP: &str =
    "Filter by macro name or tag. ↑↓ moves the selection, Enter edits it.";

// --- Toolbar ---

/// Command palette button.
pub const TB_PALETTE: &str = "Command palette (Ctrl+K)";

// --- Variables panel ---

/// Declared-variable row context menu.
pub const VARS_ROW_MENU: &str = "Right-click to edit or remove.";

// --- Shared widgets ---

/// Magnifier icon on the shared search field.
pub const FIELD_SEARCH: &str = "Search";
/// Tag chip in reorderable lists.
pub const TAG_CHIP_ORDER: &str = "Drag to reorder. Right-click for options.";
/// Tag chip on the action include/exclude picker.
pub const TAG_CHIP_TOGGLE: &str = "Click to toggle include/exclude. Right-click for more options.";

// --- Settings: macro run ---

/// Logs window entry cap.
pub const SET_LOG_CAP: &str =
    "Per-action cap for the Logs window; the oldest entries are dropped past it.";
/// Highlight the running action's tree row.
pub const SET_RUN_ROW_HL: &str = "Scroll and tint the tree row of the action running now.";
/// Gold click marker on the desktop.
pub const SET_CLICK_MARKER: &str =
    "During a run, a shrinking gold square briefly appears around each mouse press on the desktop (Click actions and Navigate selects).";
/// Release held keys/buttons on run end.
pub const SET_RELEASE_KEYS: &str =
    "When enabled, any key or mouse button still held from Down/hold actions is released when the macro finishes, stops, or errors.";
/// Overlay stop behavior.
pub const SET_OVL_STOP: &str =
    "While a macro is running, pressing an overlay button stops it. When off, a busy button lets the click through.";
/// Include overlay buttons in search captures.
pub const SET_CAPTURE_BUTTONS: &str =
    "When on, Image Search, Find Pixel, and OCR captures include the buttons. When off, the buttons stay hidden for the whole macro so those captures see the screen underneath.";
/// Button visibility while Sqyre is focused.
pub const SET_BUTTONS_FRONT: &str =
    "When off, buttons hide while the Sqyre window is in front. When on, General buttons stay on top of Sqyre. Buttons for another app still show only while that app is in front. Placing a button in the Overlay editor still shows it.";
/// While-loop safety budget.
pub const SET_WHILE_BUDGET: &str =
    "While safety budget: used when a While action has max_iterations ≤ 0. Prevents runaway loops.";
/// Run Macro nesting cap.
pub const SET_RUN_DEPTH: &str =
    "Maximum nested Run Macro calls (including the top-level macro). Cycles are always rejected.";
/// Hide Sqyre windows while recording.
pub const SET_HIDE_ON_RECORD: &str =
    "When enabled, Sqyre windows are hidden before the desktop snapshot used by recording.";
/// Sync hotkey tags to the focused program's macro tags.
pub const SET_HOTKEY_TAGS: &str =
    "When enabled, focusing a Program sets the hotkey tag selection to that program's macro tags, including its name. Hotkeys turn off when no Program owns focus. When disabled, tag selection is manual only (macro list headers).";
/// Stop variant matching on first hit.
pub const SET_VARIANT_STOP: &str =
    "When enabled, stop trying later icon variants after one hits on the same search (or collection cell). Disable to match every variant.";
/// Image search duplicate distance.
pub const SET_DEDUP_PX: &str =
    "Image search close-match distance: ignore duplicate matches within this many pixels.";

// --- Settings: sound ---

/// Completion cue.
pub const SET_FINISH_CUE: &str =
    "Plays a short cue when a top-level macro run completes successfully.";
/// Add/delete cues.
pub const SET_ADD_DEL_CUES: &str =
    "Plays short cues when macros, actions, or data-editor entities are added or deleted.";
/// Cue volume slider.
pub const SET_CUE_VOLUME: &str = "Volume for finish and add/delete cue sounds.";

// --- Settings: data location & backups ---

/// Flatpak host-folder picker.
#[cfg(not(target_arch = "wasm32"))]
pub const SET_HOST_FOLDER: &str =
    "Opens a folder picker (portal grant). Select your host ~/.sqyre folder, or its parent (Home). For a permanent Flatpak permission you can also run:\nflatpak override --user --filesystem=~/.sqyre:create com.sqyre.app";
/// Reset to the Flatpak sandbox path.
#[cfg(not(target_arch = "wasm32"))]
pub const SET_FLATPAK_RESET: &str =
    "Clear the custom path and return to the private Flatpak data directory (~/.var/app/com.sqyre.app/.sqyre).";
/// Automatic backup toggle.
#[cfg(not(target_arch = "wasm32"))]
pub const SET_AUTO_BACKUP: &str =
    "Periodically zip the data directory into the backups folder when Sqyre is running.";
/// Backup interval.
#[cfg(not(target_arch = "wasm32"))]
pub const SET_BACKUP_HOURS: &str = "Hours between automatic backups while Sqyre is open.";
/// Backup retention cap.
#[cfg(not(target_arch = "wasm32"))]
pub const SET_BACKUP_KEEP: &str =
    "Oldest managed sqyre-backup-*.zip files are deleted beyond this count.";

// --- Settings: updates ---

/// Check-on-startup toggle.
#[cfg(not(target_arch = "wasm32"))]
pub const SET_UPDATE_CHECK: &str =
    "Queries GitHub Releases for a newer Sqyre build when the app starts.";
/// "Check now" button.
#[cfg(not(target_arch = "wasm32"))]
pub const SET_UPDATE_NOW: &str = "Query GitHub Releases for a newer build.";

// --- Settings: appearance ---

/// Icon-only program headers.
pub const SET_ICON_HEADERS: &str =
    "When a program has a process icon, list headers show only the icon with the child count on the right (name on hover). Macro list tag headers attached to a program's Macro tags do the same.";
/// Keep empty program headers in lists.
pub const SET_KEEP_HEADERS: &str =
    "In the Data Editor lists, keep a header for every program even when it has nothing on that tab (for example, no items or no points).";
/// Main-window outline.
pub const SET_WIN_OUTLINE: &str = "Show a thin gold outline around the edge of the main window.";
/// OS title bar / decorations.
#[cfg(not(target_arch = "wasm32"))]
pub const SET_WIN_DECORATIONS: &str = "Show the system title bar around the main window. When off, drag the top bar to move, double-click it to maximize, and grab the window edges to resize.";
/// Run button glow.
pub const SET_RUN_GLOW: &str =
    "Show a green glow around the Run button when a macro is ready to run; it pulses while hovered.";
/// Preview desktop outline.
pub const SET_PREVIEW_OUTLINE: &str =
    "While an image preview tooltip is open, draw a gold box on the desktop at the point or search area it shows. Find markers pinned from the Logs window still outline.";
/// Font size.
pub const SET_FONT_SIZE: &str = "Base text size for labels, buttons, and form fields.";
/// UI scale.
pub const SET_UI_SCALE: &str =
    "Scale padding, icons, and other non-text UI elements relative to the display's own scale (1.0 = default). Ctrl+`+`/`-` adjusts it anywhere; Ctrl+`0` resets.";
/// Spacing density.
pub const SET_DENSITY: &str =
    "Space between sections and elements. Compact: 4px gaps. Normal: 8px between sections, 4px inside. Spaced: 12px between sections, 8px inside.";
/// Auto-compact on narrow windows.
pub const SET_AUTO_COMPACT: &str =
    "Switch to Compact spacing automatically when the window is narrow (phones, small windows).";
/// Accent color picker.
pub const SET_ACCENT: &str =
    "Gold accent used for focused outlines, the Run glow, and overlay button defaults.";
/// UI font family.
pub const SET_FONT_FAMILY: &str =
    "Text font for the interface. Monospace uses the code font for everything.";
/// Monospace font size.
pub const SET_MONO_SIZE: &str =
    "Text size for code/YAML editors (and the whole interface when the font family is Monospace).";

// --- Settings: application ---

/// Idle screen-capture suspend.
#[cfg(all(target_os = "linux", feature = "native-runtime"))]
pub const SET_SUSPEND_CAPTURE: &str =
    "After the screen sits still, Wayland screen sharing sleeps. The next capture shows a loading icon while it wakes. Turn this off to keep sharing ready.";
/// Shown while a slept screen-share stream is waking for a capture.
pub const CAPTURE_WAKE: &str = "Waking screen capture…";
/// Diagnostic log toggle.
#[cfg(all(not(target_arch = "wasm32"), feature = "native-runtime"))]
pub const SET_DIAG_LOG: &str =
    "Append detailed diagnostics to diag.log in the data folder (same as SQYRE_DIAG=1). Helps troubleshoot capture and input problems.";
/// Memory log toggle.
#[cfg(all(not(target_arch = "wasm32"), feature = "native-runtime"))]
pub const SET_MEM_LOG: &str =
    "Sample process memory to mem.log in the data folder (same as SQYRE_MEM=1).";
/// Memory sample interval.
#[cfg(all(not(target_arch = "wasm32"), feature = "native-runtime"))]
pub const SET_MEM_INTERVAL: &str = "Milliseconds between memory samples.";
/// Close-to-tray toggle.
#[cfg(not(target_arch = "wasm32"))]
pub const SET_CLOSE_TO_TRAY: &str =
    "Closing the window hides Sqyre in the system tray instead of quitting. Use Quit from the tray menu to exit.";
/// Start minimized toggle.
#[cfg(not(target_arch = "wasm32"))]
pub const SET_START_MINIMIZED: &str =
    "Start with the window hidden in the system tray instead of shown.";
/// Launch-on-login toggle.
#[cfg(all(not(target_arch = "wasm32"), target_os = "linux"))]
pub const SET_AUTOSTART: &str =
    "Install a desktop autostart entry so Sqyre launches when you log in.";
/// Restore last macro toggle.
pub const SET_LAST_MACRO: &str = "Re-select the macro that was selected when Sqyre last closed.";

// --- Settings: playback defaults ---

/// Click marker duration.
pub const SET_CLICK_FLASH_MS: &str =
    "How long the click marker stays on the desktop, in milliseconds.";
/// Cursor restore toggle.
#[cfg(any(feature = "native-runtime", target_os = "windows"))]
pub const SET_RESTORE_CURSOR: &str =
    "Move the mouse cursor back to where it was when the macro run ends.";
/// Default key/mouse delays for new macros.
pub const SET_DEFAULT_DELAYS: &str =
    "Delay after each key press / mouse action assigned to macros created from now on. Existing macros keep their own delays.";
/// Logged image size cap.
pub const SET_LOG_IMAGE_EDGE: &str =
    "Largest edge (pixels) for annotated images saved into the action log. Smaller values save memory.";
/// Reset saved action defaults.
pub const SET_RESET_ACTION_DEFAULTS: &str =
    "Forget the per-type default actions saved in the Add Action picker (right-click a tile there to edit them).";
/// Default trigger for new macro hotkeys.
pub const SET_DEFAULT_TRIGGER: &str =
    "Whether a new macro's hotkey fires on key press or on release.";
/// Panic/stop key.
#[cfg(not(target_arch = "wasm32"))]
pub const SET_PANIC_KEY: &str =
    "Key that stops a running macro (pressed alone, with no modifiers). Clear it to disable.";
/// Hotkey-while-running policy.
pub const SET_RETRIGGER: &str =
    "What pressing a running macro's hotkey does: ignore it, stop the run, or restart the macro.";

// --- Settings: recording ---

/// Recording wait threshold.
pub const SET_REC_WAIT: &str =
    "Pauses at least this long are written into the recording as Wait actions.";
/// Recording move distance.
pub const SET_REC_MOVE: &str =
    "Mouse moves shorter than this many pixels are dropped between recorded clicks.";
/// Recording tap cutoff.
pub const SET_REC_TAP: &str =
    "Press-and-release shorter than this is recorded as a click; longer becomes down/up.";
/// Recording snap radius.
pub const SET_REC_SNAP: &str =
    "Recorded clicks within this many pixels of a saved catalog point reuse that point.";

// --- Settings: sound files ---

/// Custom finish sound file.
#[cfg(not(target_arch = "wasm32"))]
pub const SET_FINISH_FILE: &str =
    "MP3 played when a macro finishes. Empty uses the built-in sound.";
/// Custom add sound file.
#[cfg(not(target_arch = "wasm32"))]
pub const SET_ADD_FILE: &str = "MP3 played when items are added. Empty uses the built-in sound.";
/// Custom delete sound file.
#[cfg(not(target_arch = "wasm32"))]
pub const SET_DELETE_FILE: &str =
    "MP3 played when items are deleted. Empty uses the built-in sound.";

// --- Settings: backup folder ---

/// Backup destination folder.
#[cfg(not(target_arch = "wasm32"))]
pub const SET_BACKUP_DIR: &str =
    "Folder that receives automatic and manual backup zips. Empty uses <data folder>/backups.";

// --- Settings: updates ---

/// Update check interval.
#[cfg(not(target_arch = "wasm32"))]
pub const SET_UPDATE_INTERVAL: &str =
    "Hours between automatic update checks while Sqyre is running. 0 checks once per launch.";
/// Prerelease update toggle.
#[cfg(not(target_arch = "wasm32"))]
pub const SET_UPDATE_PRERELEASE: &str =
    "Offer beta and release-candidate builds in addition to stable releases.";
