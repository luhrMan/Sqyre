//go:build nohook

package recording

import (
	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/dialog"
)

// ShowKeyRecordDialog is unavailable when built with -tags=nohook (headless tests without X11/gohook).
func ShowKeyRecordDialog(
	parent fyne.Window,
	_ func(key string),
) {
	if parent == nil {
		return
	}
	dialog.NewInformation("Record key", "Key recording requires a display and is disabled in this build.", parent).Show()
}
