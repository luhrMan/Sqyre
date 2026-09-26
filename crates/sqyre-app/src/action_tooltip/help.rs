//! Concise hover help for action editor fields.
//!
//! One short sentence per setting. Empty string means no tip.
//! Prefer a `?` icon ([`icon`] / [`label`]) over inline weak helper paragraphs.

use eframe::egui;

/// Apply hover text when `help` is non-empty.
pub fn tip(resp: egui::Response, help: &str) -> egui::Response {
    if help.is_empty() {
        resp
    } else {
        resp.on_hover_text(help)
    }
}

/// Small `?` that shows `help` on hover. Draws nothing when `help` is empty.
pub fn icon(ui: &mut egui::Ui, help: &str) -> Option<egui::Response> {
    if help.is_empty() {
        return None;
    }
    let size = ui.text_style_height(&egui::TextStyle::Small);
    Some(
        ui.add(
            egui::Label::new(
                egui::RichText::new(egui_phosphor::regular::QUESTION)
                    .weak()
                    .size(size),
            )
            .sense(egui::Sense::hover()),
        )
        .on_hover_text(help),
    )
}

/// Label with an adjacent `?` help icon when `help` is non-empty.
pub fn label(ui: &mut egui::Ui, text: &str, help: &str) -> egui::Response {
    if help.is_empty() {
        return ui.label(text);
    }
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 2.0;
        let resp = ui.label(text);
        icon(ui, help);
        resp
    })
    .inner
}

/// Heading with an adjacent `?` help icon when `help` is non-empty.
pub fn heading(ui: &mut egui::Ui, text: &str, help: &str) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 4.0;
        ui.heading(text);
        icon(ui, help);
    });
}

// --- Shared / common ---

pub const NAME: &str = "Optional label shown in the tree.";

// --- Wait ---

pub const WAIT_TIME: &str = "Pause duration in milliseconds before the next action.";

// --- Click ---

pub const CLICK_BUTTON: &str =
    "Click a region on the mouse: left, right, middle (wheel), or scroll (body).";
pub const CLICK_STATE: &str = "Up = release; Down = press; Tap = press and release in one action.";

// --- Key ---

pub const KEY: &str = "Key name (e.g. enter, ctrl, a). Use Record to capture.";
pub const KEY_STATE: &str = "Up = release; Down = press; Tap = press and release in one action.";

// --- Type ---

pub const TYPE_TEXT: &str = "Text to type character by character. Supports ${var} refs.";
pub const TYPE_DELAY: &str = "Milliseconds between each character.";

// --- Move ---

pub const MOVE_POINT: &str = "Target point from the Data Editor (program~name). Required to run.";
pub const MOVE_SMOOTH: &str = "Animate the cursor instead of teleporting.";
pub const MOVE_SMOOTH_LOW: &str = "Minimum smooth-move duration in seconds.";
pub const MOVE_SMOOTH_HIGH: &str = "Maximum smooth-move duration in seconds.";
pub const MOVE_SMOOTH_DELAY: &str =
    "Milliseconds between smooth-move steps (Linux). Windows uses a fixed 10 ms step.";

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
pub const CLAUSE_LEFT: &str =
    "Left side of the comparison (text, ${var}, or expression). Spaces are allowed in text.";
pub const CLAUSE_OP: &str = "Comparison operator.";
pub const CLAUSE_RIGHT: &str =
    "Right side of the comparison (text, ${var}, or expression). Spaces are allowed in text.";
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
pub const FOREACH_CELLS: &str =
    "Collection cell range to visit (1×1 cells, row-major). Sets CellX/CellY and Cell Bounds vars.";

// --- Detection shared ---

pub const SEARCH_AREA: &str = "Screen region to scan (from the Data Editor). Required to run.";
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
pub const IS_TARGET_TAGS: &str =
    "Include (+) or exclude (−) catalog item tags. An item must match every + tag and none of the − tags (exact names, all programs). Click +/− on a chip to toggle.";
pub const IS_SEARCH_SORTING: &str =
    "Primary order for searching and displaying Items. Dragging an item switches Sort by to Manual.";
pub const IS_SORT_THEN: &str =
    "Secondary order within each Sort by group (e.g. Tags then Name A→Z).";
pub const IS_TAG_PRIORITY: &str =
    "When Sort by is Tags: drag chips to set priority. Unmatched items keep Then-order at the end.";
pub const IS_TOLERANCE: &str =
    "How close a match must be to count as a hit. Higher scores are better; values are usually between 0 and 1.";
pub const IS_TOLERANCE_SQDIFF: &str =
    "How different the screen can be from the template and still count as a hit. Lower scores are better; 0 is a perfect match.";
pub const IS_TOLERANCE_UNNORMED: &str =
    "Score threshold with raw (unscaled) scores. For Default/Correlation, higher is better; for Difference, lower is better.";
pub const IS_METHOD: &str =
    "How Sqyre scores a match. Default (recommended) works for most cases. Difference methods treat lower scores as better.";
pub const IS_BLUR: &str = "Blur radius applied before matching (reduces noise).";

// --- OCR ---

pub const OCR_TARGET: &str =
    "Text that must appear for the branch to run. Empty = always read once at search center. Multiple occurrences each run the branch.";
pub const OCR_OUTPUT: &str = "Variable that receives the recognized text.";
pub const OCR_BLUR: &str = "Blur radius before OCR.";
pub const OCR_MIN_THRESHOLD: &str =
    "Darkest pixels to keep before reading text (0–255). Raise to drop dim noise.";
pub const OCR_RESIZE: &str = "Scale factor applied to the region before OCR.";
pub const OCR_GRAYSCALE: &str = "Convert the region to grayscale before OCR.";
pub const OCR_OTSU: &str = "Auto-pick a black/white cutoff so text stands out before reading.";
pub const OCR_INVERT: &str =
    "Swap light and dark after the cutoff (for light text on dark backgrounds).";

// --- Find pixel ---

pub const PIXEL_COLOR: &str =
    "Target hex color (RRGGBB). Nearby matching pixels are clustered into one hit each; Match order applies. Use Record to sample the screen.";
pub const PIXEL_TOLERANCE: &str = "Allowed per-channel color distance from the target.";

// --- Navigate Select ---

pub const NAV_PROGRAM: &str = "Program whose Atlas this navigator uses.";
pub const NAV_ATLAS: &str = "Atlas name within the program (group of Collections).";
pub const NAV_CHORD_UP: &str = "Keys that move selection up (one per line).";
pub const NAV_CHORD_DOWN: &str = "Keys that move selection down (one per line).";
pub const NAV_CHORD_LEFT: &str = "Keys that move selection left (one per line).";
pub const NAV_CHORD_RIGHT: &str = "Keys that move selection right (one per line).";
pub const NAV_CHORD_SELECT: &str = "Keys that confirm the current cell (one per line).";
pub const NAV_CHORD_BACK: &str = "Keys that exit navigation (one per line).";
pub const NAV_WRAP: &str =
    "Wrap to the opposite edge of the current Collection when there is no connected neighbor.";
pub const NAV_MOVE_CURSOR: &str = "Move the mouse cursor to the selected cell.";
pub const NAV_SMOOTH: &str = "Smooth the cursor when Move cursor with nav is on.";
pub const NAV_PASS_THROUGH: &str = "Let navigation keys also reach the focused app.";
pub const NAV_HOLD_REPEAT: &str = "Repeat movement while a chord is held.";
pub const NAV_SELECT_DEVICE: &str = "mouse or keyboard for the Select action.";
pub const NAV_SELECT_BUTTON: &str = "Mouse button used when Select device is mouse.";
pub const NAV_SELECT_KEY: &str = "Key used when Select device is keyboard.";
pub const NAV_SELECT_PRESS: &str =
    "click = down+up; down or hold keeps the button/key down; up releases.";
pub const NAV_IN_ATLAS: &str = "Optional starting Atlas override (variable or name).";
pub const NAV_IN_ROW: &str = "Optional starting row override.";
pub const NAV_IN_COL: &str = "Optional starting column override.";
pub const NAV_IN_COLLECTION: &str = "Optional starting Collection within the Atlas.";
pub const NAV_OUT_REF: &str = "Variable for the selected cell reference.";
pub const NAV_OUT_ATLAS: &str = "Variable for the current Atlas name.";
pub const NAV_OUT_ROW: &str = "Variable for the current row.";
pub const NAV_OUT_COL: &str = "Variable for the current column.";
pub const NAV_OUT_COLLECTION: &str = "Variable for the current collection.";
pub const NAV_KEY_CHILDREN: &str =
    "Nest Nav Key actions under this node for custom chord branches.";

// --- Nav Key ---

pub const NAV_KEY_EXIT: &str = "Leave Navigate Select after this branch finishes.";
pub const NAV_KEY_CHORD: &str = "Keys that trigger this branch (one per line).";

// --- Data editor ---

pub const DE_NAME: &str =
    "Unique name within this program. Cannot contain < > : \" / \\ | ? * or end with a period.";
pub const DE_RUNNING_PROGRAM: &str =
    "Process and window title that must own focus for this program's overlay buttons to show.";
pub const DE_PROGRAM_MACRO_TAGS: &str =
    "When Settings → while focused is on, these tags become the hotkey selection while this program owns focus. Same labels as macro tags.";
pub const DE_COLS: &str =
    "Grid columns this item occupies in a collection. Image Search uses this footprint (0 = 1).";
pub const DE_ROWS: &str =
    "Grid rows this item occupies in a collection. Image Search uses this footprint (0 = 1).";
pub const DE_STACK_MAX: &str = "Max stacked instances when capturing variants (0 = unset).";
pub const DE_MASK: &str = "Optional mask applied during image search.";
pub const DE_TAGS: &str = "Labels for filtering items in pickers.";
pub const DE_POINT_COORDS: &str =
    "X/Y are relative to the monitor; integers or ${var} expressions.";
pub const DE_POINT_X: &str = "X coordinate (number or expression).";
pub const DE_POINT_Y: &str = "Y coordinate (number or expression).";
pub const DE_AREA_BOUNDS: &str =
    "Bounds are relative to one monitor; integers or ${var} expressions.";
pub const DE_AREA_LEFT: &str = "Left edge X of the search area.";
pub const DE_AREA_TOP: &str = "Top edge Y of the search area.";
pub const DE_AREA_RIGHT: &str = "Right edge X of the search area.";
pub const DE_AREA_BOTTOM: &str = "Bottom edge Y of the search area.";
pub const DE_BOUNDS_PREVIEW: &str =
    "Bounds overlay the preview edges; they are relative to one monitor.";
pub const DE_SCREENCAP_INTRO: &str =
    "Set monitor-relative LeftX/TopY/RightX/BottomY (type or screen-record), name the capture, then Save writes the framed preview to images/ScreenCap. New Item creates a catalog item with Name, Tags, Cols/Rows/Stack max, and Mask, using the capture as Original.";
pub const DE_SCREENCAP_REF: &str =
    "Optional. Picking a search area or collection cell loads its bounds into LeftX/TopY/RightX/BottomY and suggests a filename.";
pub const DE_SCREENCAP_NEW_ITEM: &str =
    "Create a catalog item in the selected program using Name, Tags, Cols/Rows/Stack max, and Mask, with the preview screenshot as the Original icon.";
pub const DE_PIXELCHECK_INTRO: &str =
    "Select an item, set a search area (reference or inline coords), tune match settings, then inspect the similarity heatmap.";
pub const DE_PIXELCHECK_BOUNDS: &str =
    "Bounds overlay the preview edges; relative to one monitor; integers or ${var}.";
pub const DE_COLLECTION_AREA: &str = "Search area used when capturing this collection.";
pub const DE_COLLECTION_ROWS: &str = "Number of rows in the collection grid.";
pub const DE_COLLECTION_COLS: &str = "Number of columns in the collection grid.";
pub const DE_ATLAS_MEMBERS: &str =
    "Collections included in this Atlas. Neighbors are derived from their on-screen positions.";
pub const DE_ATLAS_PLANE: &str =
    "Monitors behind Collections; neighbors from search-area positions.";
pub const DE_MASK_SHAPE: &str =
    "Rectangle or circle geometry for the mask. Numeric fields accept literals or ${var} expressions.";
pub const DE_MASK_IMAGE_MODE: &str =
    "Image mask mode — shape fields are hidden while a PNG is on disk.";
pub const DE_MASK_INVERSE: &str = "When on, only the shape region is kept; the rest is masked out.";
pub const DE_PREVIEW_ZOOM: &str = "Scroll to zoom; drag to pan when zoomed.";
pub const DE_COLLECTION_CELL_ZOOM: &str =
    "Scroll to zoom; drag to pan when zoomed; click/drag selects cells at 100%.";
pub const DE_OVERLAY_INTRO: &str =
    "General buttons stay on screen when enabled. Other programs show only while their bound process and window title own focus (bind a window on the Programs tab). The selected button is previewed on screen while you edit.";
pub const DE_OVERLAY_MACRO: &str =
    "Macro launched when the overlay button is clicked (also used as the button name).";
pub const DE_OVERLAY_ENABLED: &str =
    "When off, this button is hidden from the screen (still editable in the detail form).";
pub const DE_OVERLAY_POINT: &str =
    "Optional catalog point for button location. When set, X/Y are used only if the point cannot be resolved.";
pub const DE_OVERLAY_X: &str = "Button X on the desktop (pixels). Ignored when a point is set.";
pub const DE_OVERLAY_Y: &str = "Button Y on the desktop (pixels). Ignored when a point is set.";
pub const DE_OVERLAY_SIZE: &str = "Button size in pixels.";
pub const DE_OVERLAY_RADIUS: &str = "Corner roundness of the button.";
pub const DE_OVERLAY_BORDER: &str = "Border thickness of the button.";
pub const DE_OVERLAY_ICON: &str =
    "Optional Phosphor icon glyph on the button. Click the preview to choose from the library.";
pub const DE_OVERLAY_ICON_PICKER: &str = "Search Phosphor icons by name, then click to select.";
pub const DE_OVERLAY_ICON_HOVER: &str = "Icon color when the pointer is over the button.";
pub const DE_OVERLAY_GATE: &str =
    "When enabled, the button only appears while a matching image is found in the search area (checked in the background).";
pub const DE_OVERLAY_GATE_AREA: &str =
    "Catalog search area to capture each poll (program~name). Resolved with live monitor slots.";
pub const DE_OVERLAY_GATE_ITEMS: &str =
    "Catalog items (icon templates) to look for — same as Image Search targets.";
pub const DE_OVERLAY_GATE_INTERVAL: &str =
    "Milliseconds between capture+match polls. Polls run on a background thread so they do not wait for the main window.";
pub const DE_OVERLAY_ALPHA_NONE: &str = "Alpha 0 = fully transparent / none.";
pub const SETTING_LOG_META: &str =
    "When enabled, image search / OCR keep debug frames in action logs (in memory). Warning: can be very memory intensive.";

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
    "Labels that group macros in the list. Use each tag's Hotkeys key icon to include that group in the hotkey filter; none are active until a key is filled.";
pub const META_HOTKEY_PRESS: &str = "Fire when the hotkey is pressed.";
pub const META_HOTKEY_RELEASE: &str = "Fire when the hotkey is released.";
pub const META_HOTKEY_CLEAR: &str = "Remove the global hotkey from this macro.";
