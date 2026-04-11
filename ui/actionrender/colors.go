package actionrender

import (
	"image/color"
	"strings"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/theme"
)

func ActionCategoryForType(actionType string) string {
	switch strings.ToLower(strings.TrimSpace(actionType)) {
	case "move", "click", "key", "type":
		return "Mouse & Keyboard"
	case "imagesearch", "ocr", "findpixel":
		return "Detection"
	case "setvariable", "calculate", "datalist", "savevariable":
		return "Variables"
	case "wait", "focuswindow", "runmacro", "loop":
		return "Miscellaneous"
	default:
		return ""
	}
}

func ActionPastelColor(actionType string) color.NRGBA {
	t := strings.ToLower(strings.TrimSpace(actionType))
	category := ActionCategoryForType(t)
	isWait := t == "wait"

	isDark := fyne.CurrentApp().Settings().ThemeVariant() == theme.VariantDark
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
