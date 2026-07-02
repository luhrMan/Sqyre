//go:build !nohook

package ui

import (
	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/dialog"
	hook "github.com/luhrMan/gohook"
)

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
