package custom_widgets

import (
	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/widget"
)

// IsMultiLineTextFocused reports whether focus is on a multi-line text entry.
func IsMultiLineTextFocused(f fyne.Focusable) bool {
	if f == nil {
		return false
	}
	switch e := f.(type) {
	case *VarEntry:
		return e.MultiLine
	case *widget.Entry:
		return e.MultiLine
	default:
		return false
	}
}
