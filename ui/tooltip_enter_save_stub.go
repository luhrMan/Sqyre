//go:build nohook

package ui

import "fyne.io/fyne/v2"

// RegisterActionTooltipEnterSave is a no-op when built with -tags=nohook.
func RegisterActionTooltipEnterSave(_ fyne.Window, _ func()) func() {
	return func() {}
}
