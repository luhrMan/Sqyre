package ui

import (
	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/dialog"
)

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
