//! Concise hover help for action editor fields.
//!
//! One short sentence per setting. Empty string means no tip.

use eframe::egui;

/// Apply hover text when `help` is non-empty.
pub fn tip(resp: egui::Response, help: &str) -> egui::Response {
    if help.is_empty() {
        resp
    } else {
        resp.on_hover_text(help)
    }
}

/// Label that shows `help` on hover when non-empty.
pub fn label(ui: &mut egui::Ui, text: &str, help: &str) -> egui::Response {
    tip(ui.label(text), help)
}

// --- Shared / common ---

pub const NAME: &str = "Optional label shown in the tree.";

// --- Wait ---

pub const WAIT_TIME: &str = "Pause duration in milliseconds before the next action.";

// --- Click ---

pub const CLICK_BUTTON: &str = "Which mouse button to press or release.";
pub const CLICK_STATE: &str = "Up = release; Down = press; Tap = press and release in one action.";

// --- Key ---

pub const KEY: &str = "Key name (e.g. enter, ctrl, a). Use Record to capture.";
pub const KEY_STATE: &str = "Up = release; Down = press; Tap = press and release in one action.";

// --- Type ---

pub const TYPE_TEXT: &str = "Text to type character by character. Supports ${var} refs.";
pub const TYPE_DELAY: &str = "Milliseconds between each character.";

// --- Move ---

pub const MOVE_POINT: &str = "Target point from the Data Editor (program~name).";
pub const MOVE_SMOOTH: &str = "Animate the cursor instead of teleporting.";
pub const MOVE_SMOOTH_LOW: &str = "Minimum fraction of the path used for easing (0–1).";
pub const MOVE_SMOOTH_HIGH: &str = "Maximum fraction of the path used for easing (0–1).";
pub const MOVE_SMOOTH_DELAY: &str = "Milliseconds between smooth-move steps.";

// --- Pause ---

pub const PAUSE_MESSAGE: &str = "Message shown while the macro is paused.";
pub const PAUSE_CONTINUE: &str = "Keys that resume the macro (one chord per line).";
pub const PAUSE_PASS_THROUGH: &str = "Let the continue key also reach the focused app.";

// --- Focus window ---

pub const FOCUS_TITLE: &str = "Window title to match (substring).";
pub const FOCUS_PROCESS: &str = "Executable path of the process that owns the window.";

// --- Run macro ---

pub const RUN_MACRO: &str = "Macro to run inline as a subroutine.";

// --- Set variable ---

pub const SET_VAR: &str = "Variable name to assign.";
pub const SET_VALUE: &str =
    "Plain text, ${ref}, or a math expression. Use f(x) to insert functions.";
pub const SET_FX: &str = "Insert a math function, constant, or operator.";
pub const SET_ADD_ASSIGNMENT: &str = "Add another variable assignment.";
pub const SET_REMOVE_ASSIGNMENT: &str = "Remove this assignment.";

// --- Save variable ---

pub const SAVE_VAR: &str = "Variable whose value is written out.";
pub const SAVE_DEST: &str = "File path, or clipboard to copy.";
pub const SAVE_APPEND: &str = "Append to the file instead of overwriting.";
pub const SAVE_NEWLINE: &str = "Add a newline after the value when appending.";

// --- Loop ---

pub const LOOP_COUNT: &str = "How many times to run child actions (number or ${var}).";
pub const LOOP_JUMP_MODE: &str =
    "Break exits the innermost loop; Continue skips to its next iteration.";

// --- While / If ---

pub const MATCH_ALL: &str = "All clauses must pass. Uncheck to require any one.";
pub const MAX_ITERATIONS: &str = "Hard stop for While (0 = use the default limit).";
pub const CLAUSE_LEFT: &str = "Left side of the comparison (value or ${var}).";
pub const CLAUSE_OP: &str = "Comparison operator.";
pub const CLAUSE_RIGHT: &str = "Right side of the comparison (value or ${var}).";
pub const CLAUSE_ADD: &str = "Add another condition clause.";
pub const CLAUSE_REMOVE: &str = "Remove this clause.";

// --- For each row ---

pub const FOREACH_START: &str = "First row to process (1-based).";
pub const FOREACH_END: &str = "Last row to process (empty = through the end).";
pub const FOREACH_SOURCE: &str = "List text, ${var}, or a file path when Is file is set.";
pub const FOREACH_OUTPUT: &str = "Variable that receives the current cell each row.";
pub const FOREACH_IS_FILE: &str = "Treat Source as a path and read lines from that file.";
pub const FOREACH_SKIP_BLANK: &str = "Skip empty lines in the source.";
pub const FOREACH_ADD_SOURCE: &str = "Add another column source.";
pub const FOREACH_REMOVE_SOURCE: &str = "Remove this source.";

// --- Detection shared ---

pub const SEARCH_AREA: &str = "Screen region to scan (from the Data Editor).";
pub const REPEAT_MODE: &str =
    "once = single try; wait* = silent poll then one branch; repeat* = run branch each pass.";
pub const WAIT_SECONDS: &str = "How long to keep retrying (seconds). Required for wait modes.";
pub const WAIT_INTERVAL: &str = "Milliseconds between detection retries.";
pub const WAIT_MAX_ITER: &str = "Cap on repeat-mode iterations (0 = default 100).";
pub const OUT_X: &str = "Variable that receives the match X coordinate.";
pub const OUT_Y: &str = "Variable that receives the match Y coordinate.";
pub const ORDER_GROUPING: &str =
    "How multiple matches are grouped before ordering (Image Search, OCR occurrences, clustered Find Pixel).";
pub const ORDER_HORIZONTAL: &str = "Left-to-right or right-to-left among matches.";
pub const ORDER_VERTICAL: &str = "Top-to-bottom or bottom-to-top among matches.";
pub const ELSE_BRANCH: &str =
    "Child actions under Else run when the condition is false (If) or the target is not found (detection).";

// --- Image search ---

pub const IS_ITEMS: &str = "Template images to find (from the Data Editor).";
pub const IS_TOLERANCE: &str =
    "Score threshold for a hit. For CCOEFF/CCORR (and normed): higher is better; for SQDIFF*: lower is better. Normed methods are typically 0–1.";
pub const IS_TOLERANCE_SQDIFF: &str =
    "Maximum score to accept (lower = better). For SQDIFF_NORMED, 0 = perfect match.";
pub const IS_TOLERANCE_UNNORMED: &str =
    "Raw score threshold (not 0–1). Higher is better for CCORR/CCOEFF; lower is better for SQDIFF.";
pub const IS_METHOD: &str =
    "OpenCV template-match method. Default CCOEFF_NORMED. SQDIFF* treat lower scores as better.";
pub const IS_BLUR: &str = "Blur radius applied before matching (reduces noise).";

// --- OCR ---

pub const OCR_TARGET: &str =
    "Text that must appear for the branch to run. Empty = always read once at search center. Multiple occurrences each run the branch.";
pub const OCR_OUTPUT: &str = "Variable that receives the recognized text.";
pub const OCR_BLUR: &str = "Blur radius before OCR.";
pub const OCR_MIN_THRESHOLD: &str = "Minimum pixel intensity kept before OCR (0–255).";
pub const OCR_RESIZE: &str = "Scale factor applied to the region before OCR.";
pub const OCR_GRAYSCALE: &str = "Convert the region to grayscale before OCR.";
pub const OCR_OTSU: &str = "Apply Otsu thresholding before OCR.";
pub const OCR_INVERT: &str = "Invert light/dark after thresholding.";

// --- Find pixel ---

pub const PIXEL_COLOR: &str =
    "Target hex color (RRGGBB). Nearby matching pixels are clustered into one hit each; Match order applies. Use Record to sample the screen.";
pub const PIXEL_TOLERANCE: &str = "Allowed per-channel color distance from the target.";

// --- Navigate Select ---

pub const NAV_PROGRAM: &str = "Program whose collection/graph this navigator uses.";
pub const NAV_GRAPH: &str = "Graph name within the program.";
pub const NAV_CHORD_UP: &str = "Keys that move selection up (one per line).";
pub const NAV_CHORD_DOWN: &str = "Keys that move selection down (one per line).";
pub const NAV_CHORD_LEFT: &str = "Keys that move selection left (one per line).";
pub const NAV_CHORD_RIGHT: &str = "Keys that move selection right (one per line).";
pub const NAV_CHORD_SELECT: &str = "Keys that confirm the current cell (one per line).";
pub const NAV_CHORD_BACK: &str = "Keys that exit navigation (one per line).";
pub const NAV_WRAP: &str = "Wrap to the opposite edge when moving past a boundary.";
pub const NAV_MOVE_CURSOR: &str = "Move the mouse cursor to the selected cell.";
pub const NAV_SMOOTH: &str = "Smooth the cursor when Move cursor with nav is on.";
pub const NAV_PASS_THROUGH: &str = "Let navigation keys also reach the focused app.";
pub const NAV_HOLD_REPEAT: &str = "Repeat movement while a chord is held.";
pub const NAV_SELECT_DEVICE: &str = "mouse or keyboard for the Select action.";
pub const NAV_SELECT_BUTTON: &str = "Mouse button used when Select device is mouse.";
pub const NAV_SELECT_KEY: &str = "Key used when Select device is keyboard.";
pub const NAV_SELECT_PRESS: &str = "click = down+up; down or up for a single edge.";
pub const NAV_IN_GRAPH: &str = "Optional starting graph override.";
pub const NAV_IN_ROW: &str = "Optional starting row override.";
pub const NAV_IN_COL: &str = "Optional starting column override.";
pub const NAV_IN_COLLECTION: &str = "Optional starting collection override.";
pub const NAV_OUT_REF: &str = "Variable for the selected cell reference.";
pub const NAV_OUT_GRAPH: &str = "Variable for the current graph name.";
pub const NAV_OUT_ROW: &str = "Variable for the current row.";
pub const NAV_OUT_COL: &str = "Variable for the current column.";
pub const NAV_OUT_COLLECTION: &str = "Variable for the current collection.";
pub const NAV_KEY_CHILDREN: &str =
    "Nest Nav Key actions under this node for custom chord branches.";

// --- Nav Key ---

pub const NAV_KEY_EXIT: &str = "Leave Navigate Select after this branch finishes.";
pub const NAV_KEY_CHORD: &str = "Keys that trigger this branch (one per line).";

// --- Data editor ---

pub const DE_NAME: &str = "Unique name within this program.";
pub const DE_RUNNING_PROGRAM: &str =
    "Process and window title that must be focused for overlay buttons.";
pub const DE_COLS: &str = "Grid columns for this item (0 = unset).";
pub const DE_ROWS: &str = "Grid rows for this item (0 = unset).";
pub const DE_STACK_MAX: &str = "Max stacked instances when capturing variants (0 = unset).";
pub const DE_MASK: &str = "Optional mask applied during image search.";
pub const DE_TAGS: &str = "Labels for filtering items in pickers.";
pub const DE_POINT_X: &str = "X coordinate (number or expression).";
pub const DE_POINT_Y: &str = "Y coordinate (number or expression).";
pub const DE_AREA_LEFT: &str = "Left edge X of the search area.";
pub const DE_AREA_TOP: &str = "Top edge Y of the search area.";
pub const DE_AREA_RIGHT: &str = "Right edge X of the search area.";
pub const DE_AREA_BOTTOM: &str = "Bottom edge Y of the search area.";
pub const DE_COLLECTION_AREA: &str = "Search area used when capturing this collection.";
pub const DE_COLLECTION_ROWS: &str = "Number of rows in the collection grid.";
pub const DE_COLLECTION_COLS: &str = "Number of columns in the collection grid.";
pub const DE_MASK_SHAPE: &str = "Rectangle or circle geometry for the mask.";
pub const DE_OVERLAY_LABEL: &str = "Text shown on the overlay button.";
pub const DE_OVERLAY_MACRO: &str = "Macro launched when the overlay button is clicked.";
pub const DE_OVERLAY_X: &str = "Button X on the overlay (pixels).";
pub const DE_OVERLAY_Y: &str = "Button Y on the overlay (pixels).";
pub const DE_OVERLAY_SIZE: &str = "Button size in pixels.";
pub const DE_OVERLAY_RADIUS: &str = "Corner roundness of the button.";
pub const DE_OVERLAY_BORDER: &str = "Border thickness of the button.";
pub const DE_OVERLAY_ICON: &str = "Optional Phosphor icon glyph on the button.";

// --- Variables panel ---

pub const VAR_NAME: &str = "Name used as ${name} in actions.";
pub const VAR_TYPE: &str = "auto = infer; text or number for strict typing.";
pub const VAR_INITIAL: &str = "Value seeded into the runtime store when the macro starts.";
pub const VAR_DESC: &str = "Optional note shown in the variables list.";
pub const VAR_TAB_RUNTIME: &str = "Live values while a macro runs (last snapshot when idle).";
pub const VAR_TAB_BUILTINS: &str = "Names set automatically by the runtime or certain actions.";

// --- Macro meta / list ---

pub const META_NAME: &str = "Display name of this macro (must be unique).";
pub const META_TAGS: &str =
    "Labels that group macros in the list. Selecting a tag header enables hotkeys only for that group.";
pub const META_HOTKEY_PRESS: &str = "Fire when the hotkey is pressed.";
pub const META_HOTKEY_RELEASE: &str = "Fire when the hotkey is released.";
pub const META_HOTKEY_CLEAR: &str = "Remove the global hotkey from this macro.";
