package actions

import (
	"image/color"
	"strings"
	"sync"
)

const (
	ActionColorKeyMouseKeyboard = "mouse_keyboard"
	ActionColorKeyDetection     = "detection"
	ActionColorKeyVariables     = "variables"
	ActionColorKeyMiscellaneous = "miscellaneous"
	ActionColorKeyWait          = "wait"
	ActionColorKeyDefault       = "default"
)

// ActionColorCategory describes one customizable action color group in settings.
type ActionColorCategory struct {
	Key   string
	Label string
}

// ActionColorCategories lists every action color group the user can customize.
var ActionColorCategories = []ActionColorCategory{
	{Key: ActionColorKeyMouseKeyboard, Label: "Mouse & Keyboard"},
	{Key: ActionColorKeyDetection, Label: "Detection"},
	{Key: ActionColorKeyVariables, Label: "Variables"},
	{Key: ActionColorKeyMiscellaneous, Label: "Miscellaneous"},
	{Key: ActionColorKeyWait, Label: "Wait"},
	{Key: ActionColorKeyDefault, Label: "Default"},
}

var (
	customColorsMu sync.RWMutex
	customColors   map[string]color.NRGBA
)

func ActionCategoryForType(actionType string) string {
	switch strings.ToLower(strings.TrimSpace(actionType)) {
	case "move", "click", "key", "type":
		return "Mouse & Keyboard"
	case "imagesearch", "ocr", "findpixel":
		return "Detection"
	case "setvariable", "calculate", "foreachrow", "savevariable":
		return "Variables"
	case "wait", "pause", "focuswindow", "runmacro", "loop", "conditional", "break", "continue":
		return "Miscellaneous"
	default:
		return ""
	}
}

func actionColorKey(actionType string) string {
	t := strings.ToLower(strings.TrimSpace(actionType))
	if t == "wait" || t == "pause" {
		return ActionColorKeyWait
	}
	switch ActionCategoryForType(t) {
	case "Mouse & Keyboard":
		return ActionColorKeyMouseKeyboard
	case "Detection":
		return ActionColorKeyDetection
	case "Variables":
		return ActionColorKeyVariables
	case "Miscellaneous":
		return ActionColorKeyMiscellaneous
	default:
		return ActionColorKeyDefault
	}
}

// DefaultActionPastelColor returns the built-in pastel color for an action type.
func DefaultActionPastelColor(actionType string, isDark bool) color.NRGBA {
	t := strings.ToLower(strings.TrimSpace(actionType))
	if t == "warning" {
		if isDark {
			return color.NRGBA{R: 0x8A, G: 0x5A, B: 0x2A, A: 0xFF}
		}
		return color.NRGBA{R: 0xF0, G: 0xC0, B: 0x6A, A: 0xFF}
	}
	category := ActionCategoryForType(t)
	isWait := t == "wait" || t == "pause"

	if isDark {
		if isWait {
			return color.NRGBA{R: 0x7B, G: 0x4E, B: 0x3E, A: 0xFF}
		}
		switch category {
		case "Mouse & Keyboard":
			return color.NRGBA{R: 0x5E, G: 0x6B, B: 0x4A, A: 0xFF}
		case "Detection":
			return color.NRGBA{R: 0x5A, G: 0x4A, B: 0x44, A: 0xFF}
		case "Variables":
			return color.NRGBA{R: 0x7A, G: 0x63, B: 0x45, A: 0xFF}
		case "Miscellaneous":
			return color.NRGBA{R: 0x6A, G: 0x5A, B: 0x3F, A: 0xFF}
		default:
			return color.NRGBA{R: 0x5C, G: 0x54, B: 0x49, A: 0xFF}
		}
	}
	if isWait {
		return color.NRGBA{R: 0xC9, G: 0x8D, B: 0x6A, A: 0xFF}
	}
	switch category {
	case "Mouse & Keyboard":
		return color.NRGBA{R: 0xA1, G: 0xB0, B: 0x7A, A: 0xFF}
	case "Detection":
		return color.NRGBA{R: 0xB4, G: 0x9A, B: 0x84, A: 0xFF}
	case "Variables":
		return color.NRGBA{R: 0xC7, G: 0xAE, B: 0x7B, A: 0xFF}
	case "Miscellaneous":
		return color.NRGBA{R: 0xB8, G: 0x9A, B: 0x6A, A: 0xFF}
	default:
		return color.NRGBA{R: 0xB2, G: 0xA4, B: 0x8E, A: 0xFF}
	}
}

// ActionPastelColor returns the display color for an action type, using a user
// override when one is set.
func ActionPastelColor(actionType string, isDark bool) color.NRGBA {
	key := actionColorKey(actionType)
	customColorsMu.RLock()
	c, ok := customColors[key]
	customColorsMu.RUnlock()
	if ok {
		return c
	}
	return DefaultActionPastelColor(actionType, isDark)
}

// SetCustomActionColor stores a user-chosen color for a category key.
func SetCustomActionColor(categoryKey string, c color.NRGBA) {
	customColorsMu.Lock()
	defer customColorsMu.Unlock()
	if customColors == nil {
		customColors = make(map[string]color.NRGBA)
	}
	customColors[categoryKey] = c
}

// ClearCustomActionColor removes a user override for a category key.
func ClearCustomActionColor(categoryKey string) {
	customColorsMu.Lock()
	defer customColorsMu.Unlock()
	delete(customColors, categoryKey)
}

// ClearAllCustomActionColors removes every user override.
func ClearAllCustomActionColors() {
	customColorsMu.Lock()
	defer customColorsMu.Unlock()
	customColors = nil
}
