//go:build !nohook

package ui

import (
	"Sqyre/internal/macrohotkey"

	"fyne.io/fyne/v2"
)

// RegisterActionTooltipEnterSave registers a global Enter handler that submits the
// pinned action tooltip edit form. Enter is ignored while completion is active or
// a multi-line text field has focus.
func RegisterActionTooltipEnterSave(parent fyne.Window, onSave func()) func() {
	if parent == nil || onSave == nil {
		return func() {}
	}
	return macrohotkey.RegisterEnterHandler(func() {
		if !shouldSaveTooltipOnEnter(parent) {
			return
		}
		fyne.Do(onSave)
	})
}
