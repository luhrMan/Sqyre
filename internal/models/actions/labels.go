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
	case "semanticsearch":
		return "Semantic Search"
	case "setvariable":
		return "Set"
	case "calculate":
		return "Calculate"
	case "foreachrow":
		return "For each row"
	case "savevariable":
		return "Save to"
	default:
		return actionType
	}
}
