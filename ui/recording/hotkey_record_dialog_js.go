//go:build js

package recording

import (
	"time"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/dialog"
)

// ShowHotkeyRecordDialog is a no-op on WASM; global key capture is unavailable in the browser.
func ShowHotkeyRecordDialog(
	parent fyne.Window,
	stableDuration time.Duration,
	addDialogEscapeClose func(d dialog.Dialog, parent fyne.Window),
	onRecorded func(keys []string),
) {
	_ = stableDuration
	_ = onRecorded
	d := dialog.NewInformation("Hotkey recording",
		"Recording hotkeys from the system is not available in the browser. Enter the key combination as text in the hotkey field if needed.",
		parent)
	if addDialogEscapeClose != nil {
		addDialogEscapeClose(d, parent)
	}
	d.Show()
}
