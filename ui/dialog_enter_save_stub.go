//go:build nohook

package ui

import (
	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/dialog"
)

// AddActionDialogEnterSave is a no-op when built with -tags=nohook.
func AddActionDialogEnterSave(_ dialog.Dialog, _ fyne.Window, _ func()) {}
