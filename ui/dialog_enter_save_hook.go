//go:build !nohook

package ui

import (
	"Sqyre/ui/completionentry"
	"Sqyre/ui/custom_widgets"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/dialog"
	hook "github.com/luhrMan/gohook"
)

func shouldSaveActionDialogOnEnter(parent fyne.Window) bool {
	if parent == nil {
		return false
	}
	focused := parent.Canvas().Focused()
	if completionentry.IsNavListFocused(focused) {
		return false
	}
	if custom_widgets.IsMultiLineTextFocused(focused) {
		return false
	}
	return true
}

// AddActionDialogEnterSave registers a global Enter handler that saves the action dialog.
// Enter is ignored while a completion list or multi-line text field has focus.
func AddActionDialogEnterSave(d dialog.Dialog, parent fyne.Window, onSave func()) {
	if d == nil || parent == nil || onSave == nil {
		return
	}

	enterCombo := []string{"enter"}
	hook.Register(hook.KeyDown, enterCombo, func(hook.Event) {
		if !shouldSaveActionDialogOnEnter(parent) {
			return
		}
		fyne.Do(onSave)
	})

	d.SetOnClosed(func() {
		go hook.Unregister(hook.KeyDown, enterCombo)
	})
}
