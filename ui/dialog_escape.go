package ui

import (
	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/dialog"
	hook "github.com/luhrMan/gohook"
)

// AddDialogEscapeClose enables Escape to dismiss the dialog and restores the window key handler when it closes.
// See https://pkg.go.dev/github.com/ErikKalkoken/fyne-kx/dialog#AddDialogKeyHandler
func AddDialogEscapeClose(d dialog.Dialog, parent fyne.Window) {
	if d == nil || parent == nil {
		return
	}

	closeDialog := func() {
		d.Hide()
	}

	// Global fallback: while this dialog is open, Esc closes it even if focus is inside
	// a widget that swallows local key events.
	escCombo := []string{"esc"}
	hook.Register(hook.KeyDown, escCombo, func(hook.Event) {
		fyne.Do(closeDialog)
	})

	d.SetOnClosed(func() {
		// Unregister on a new goroutine to avoid modifying hook handlers while they
		// may be processing the current event callback.
		go hook.Unregister(hook.KeyDown, escCombo)
	})
}

// ShowErrorWithEscape shows a standard error dialog; Escape dismisses it.
func ShowErrorWithEscape(err error, parent fyne.Window) {
	d := dialog.NewError(err, parent)
	AddDialogEscapeClose(d, parent)
	d.Show()
}

// ShowInformationWithEscape shows an information dialog; Escape dismisses it.
func ShowInformationWithEscape(title, message string, parent fyne.Window) {
	d := dialog.NewInformation(title, message, parent)
	AddDialogEscapeClose(d, parent)
	d.Show()
}

// ShowConfirmWithEscape shows a confirmation dialog; Escape dismisses as "cancel" (callback with false).
func ShowConfirmWithEscape(title, message string, callback func(bool), parent fyne.Window) {
	d := dialog.NewConfirm(title, message, callback, parent)
	AddDialogEscapeClose(d, parent)
	d.Show()
}
