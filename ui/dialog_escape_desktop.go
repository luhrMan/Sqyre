//go:build !js

package ui

import (
	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/dialog"
	hook "github.com/luhrMan/gohook"
)

// AddDialogEscapeClose enables Escape to dismiss the dialog and restores the window key handler when it closes.
func AddDialogEscapeClose(d dialog.Dialog, parent fyne.Window) {
	if d == nil || parent == nil {
		return
	}

	closeDialog := func() {
		d.Hide()
	}

	escCombo := []string{"esc"}
	hook.Register(hook.KeyDown, escCombo, func(hook.Event) {
		fyne.Do(closeDialog)
	})

	d.SetOnClosed(func() {
		go hook.Unregister(hook.KeyDown, escCombo)
	})
}
