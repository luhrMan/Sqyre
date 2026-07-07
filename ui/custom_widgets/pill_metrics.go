package custom_widgets

import (
	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/theme"
)

// PillTextSize matches the caption size used by display pills.
func PillTextSize() float32 {
	if app := fyne.CurrentApp(); app != nil {
		return app.Settings().Theme().Size(theme.SizeNameCaptionText)
	}
	return theme.CaptionTextSize()
}

// PillLineHeight is the text line height used inside display and editable pills.
func PillLineHeight() float32 {
	return fyne.MeasureText("Mg", PillTextSize(), fyne.TextStyle{}).Height
}
