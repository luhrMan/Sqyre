package actions

import "strings"

// ActionTypeLabel returns the human-readable name for an action type key.
func ActionTypeLabel(actionType string) string {
	switch strings.ToLower(strings.TrimSpace(actionType)) {
	case "move":
		return "Mouse Move"
	case "click":
		return "Click"
	case "key":
		return "Key"
	case "type":
		return "Type"
	case "wait":
		return "Wait"
	case "pause":
		return "Pause"
	case "focuswindow":
		return "Focus window"
	case "runmacro":
		return "Run macro"
	case "conditional":
		return "If"
	case "loop":
		return "Loop"
	case "break":
		return "Break"
	case "continue":
		return "Continue"
	case "imagesearch":
		return "Image Search"
	case "ocr":
		return "OCR"
	case "findpixel":
		return "Find pixel"
	case "setvariable":
		return "Set"
	case "foreachrow":
		return "For each row"
	case "savevariable":
		return "Save to"
	default:
		return actionType
	}
}

// ActionTypeDescription returns a concise, one-line explanation of what an
// action type does, suitable for a tooltip. Returns "" for unknown types.
func ActionTypeDescription(actionType string) string {
	switch strings.ToLower(strings.TrimSpace(actionType)) {
	case "move":
		return "Moves the mouse cursor to a target position."
	case "click":
		return "Clicks a mouse button at the current cursor position."
	case "key":
		return "Presses or releases a single keyboard key."
	case "type":
		return "Types out a string of text, one character at a time."
	case "wait":
		return "Pauses for a fixed number of milliseconds, then continues."
	case "pause":
		return "Halts the macro until you press the continue key."
	case "focuswindow":
		return "Brings a window to the front, matched by program and title."
	case "runmacro":
		return "Runs another macro inline as a sub-routine."
	case "conditional":
		return "Runs its sub-actions only when the conditions are true."
	case "loop":
		return "Repeats its sub-actions a set number of times."
	case "break":
		return "Exits the innermost enclosing loop immediately."
	case "continue":
		return "Skips to the next iteration of the enclosing loop."
	case "imagesearch":
		return "Searches a screen region for images and saves match coordinates."
	case "ocr":
		return "Reads text from a screen region and saves it to variables."
	case "findpixel":
		return "Scans a region for a pixel color and saves its coordinates."
	case "setvariable":
		return "Assigns a value or expression result to a variable."
	case "foreachrow":
		return "Runs its sub-actions once per row of a list source."
	case "savevariable":
		return "Writes a variable's value out to a file or the clipboard."
	default:
		return ""
	}
}
