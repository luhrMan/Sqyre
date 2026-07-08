//go:build !nohook

// Package dialogs centralizes modal dialog helpers (Escape-to-dismiss, error/info/
// confirm shells, modal popup wrapping) so package ui and its subpackages can share
// them directly instead of passing callbacks through WireDeps.
package dialogs

import (
	"Sqyre/internal/macrohotkey"
	"Sqyre/ui/recording"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/dialog"
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
	unregisterEsc := macrohotkey.RegisterEscapeHandler(func() {
		if recording.KeyRecordSessionActive() {
			return
		}
		fyne.Do(closeDialog)
	})

	d.SetOnClosed(func() {
		unregisterEsc()
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
