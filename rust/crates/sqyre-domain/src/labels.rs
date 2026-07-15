//! Human-readable action type labels (Go `actions.ActionTypeLabel`).

pub fn action_type_label(action_type: &str) -> &'static str {
    match action_type.trim().to_ascii_lowercase().as_str() {
        "move" => "Mouse Move",
        "click" => "Click",
        "key" => "Key",
        "type" => "Type",
        "wait" => "Wait",
        "pause" => "Pause",
        "focuswindow" => "Focus window",
        "runmacro" => "Run macro",
        "conditional" => "If",
        "loop" => "Loop",
        "while" => "While",
        "break" => "Break",
        "continue" => "Continue",
        "imagesearch" => "Image Search",
        "ocr" => "OCR",
        "findpixel" => "Find pixel",
        "setvariable" => "Set",
        "calculate" => "Calculate",
        "foreachrow" => "For each row",
        "savevariable" => "Save to",
        "navigateselect" => "Navigate Select",
        _ => "Unknown",
    }
}

pub fn action_type_description(action_type: &str) -> &'static str {
    match action_type.trim().to_ascii_lowercase().as_str() {
        "move" => "Moves the mouse cursor to a target position.",
        "click" => "Clicks a mouse button at the current cursor position.",
        "key" => "Presses or releases a single keyboard key.",
        "type" => "Types out a string of text, one character at a time.",
        "wait" => "Pauses for a fixed number of milliseconds, then continues.",
        "pause" => "Halts the macro until you press the continue key.",
        "focuswindow" => "Brings a window to the front, matched by program and title.",
        "runmacro" => "Runs another macro inline as a sub-routine.",
        "conditional" => "Runs its sub-actions only when the conditions are true.",
        "loop" => "Repeats its sub-actions a set number of times.",
        "while" => "Repeats its sub-actions while conditions remain true.",
        "break" => "Exits the innermost enclosing loop immediately.",
        "continue" => "Skips to the next iteration of the enclosing loop.",
        "imagesearch" => "Searches a screen region for images and saves match coordinates.",
        "ocr" => "Reads text from a screen region; runs nested actions when the target is found.",
        "findpixel" => "Scans a region for a pixel color; runs nested actions when found.",
        "setvariable" => "Assigns a value to a variable in memory.",
        "calculate" => "Evaluates a math expression and stores the result in a variable.",
        "foreachrow" => "Runs its sub-actions once per row of a list source.",
        "savevariable" => "Writes a variable's value out to a file or the clipboard.",
        "navigateselect" => "Navigates a collection grid with chords and confirms a cell.",
        _ => "",
    }
}
