package custom_widgets

import (
	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/widget"
)

// ConfigureSingleLineEntry opts a single-line entry out of Fyne's default
// truncation scroll wrapper, which can show an unnecessary vertical scrollbar
// when font size is large relative to the field's inner padding.
func ConfigureSingleLineEntry(e *widget.Entry) {
	e.Wrapping = fyne.TextWrapOff
	e.Scroll = fyne.ScrollNone
}

// NewFormEntry creates a single-line entry suitable for labels, dialogs, and
// settings forms.
func NewFormEntry() *widget.Entry {
	e := widget.NewEntry()
	ConfigureSingleLineEntry(e)
	return e
}
