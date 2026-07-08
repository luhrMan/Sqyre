//go:build nohook

package dialogs

import (
	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/dialog"
)

// AddDialogEscapeClose is a no-op when built with -tags=nohook (headless tests without X11/gohook).
func AddDialogEscapeClose(_ dialog.Dialog, _ fyne.Window) {}

// ShowErrorWithEscape shows a standard error dialog without a global Escape hook.
func ShowErrorWithEscape(err error, parent fyne.Window) {
	dialog.NewError(err, parent).Show()
}

// ShowInformationWithEscape shows an information dialog without a global Escape hook.
func ShowInformationWithEscape(title, message string, parent fyne.Window) {
	dialog.NewInformation(title, message, parent).Show()
}

// ShowConfirmWithEscape shows a confirmation dialog without a global Escape hook.
func ShowConfirmWithEscape(title, message string, callback func(bool), parent fyne.Window) {
	dialog.NewConfirm(title, message, callback, parent).Show()
}
