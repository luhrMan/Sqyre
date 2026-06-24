//go:build nohook

package recording

import (
	"time"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/dialog"
)

// ShowHotkeyRecordDialog is unavailable when built with -tags=nohook (headless tests without X11/gohook).
func ShowHotkeyRecordDialog(
	parent fyne.Window,
	_ time.Duration,
	_ func(d dialog.Dialog, parent fyne.Window),
	_ func(keys []string),
) {
	if parent == nil {
		return
	}
	dialog.NewInformation("Record hotkey", "Hotkey recording requires a display and is disabled in this build.", parent).Show()
}
