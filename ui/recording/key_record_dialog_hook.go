//go:build !nohook

package recording

import (
	"sync"
	"sync/atomic"
	"time"

	"Sqyre/internal/hookkeys"
	"Sqyre/internal/macrohotkey"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/container"
	"fyne.io/fyne/v2/dialog"
	"fyne.io/fyne/v2/layout"
	"fyne.io/fyne/v2/theme"
	"fyne.io/fyne/v2/widget"
	hook "github.com/luhrMan/gohook"
)

// ShowKeyRecordDialog captures a single keyboard key. The first time a key other
// than Escape is pressed, onRecorded runs and the dialog closes. Escape cancels.
func ShowKeyRecordDialog(
	parent fyne.Window,
	onRecorded func(key string),
) {
	if parent == nil || onRecorded == nil {
		return
	}

	hint := widget.NewLabel("Press the key you want to use.\nThe first key you press is saved.\nUse Cancel to dismiss without saving.")
	hint.Wrapping = fyne.TextWrapWord

	keyLabel := widget.NewLabel("(no key)")
	keyLabel.Wrapping = fyne.TextWrapWord
	keyLabel.TextStyle = fyne.TextStyle{Monospace: true}

	statusLabel := widget.NewLabel("")

	cancelBtn := widget.NewButtonWithIcon("Cancel", theme.CancelIcon(), nil)
	cancelBtn.Importance = widget.MediumImportance

	content := container.NewVBox(
		hint,
		widget.NewSeparator(),
		keyLabel,
		statusLabel,
		container.NewHBox(layout.NewSpacer(), cancelBtn),
	)

	d := dialog.NewCustomWithoutButtons("Record key", content, parent)

	macrohotkey.SuspendMacroHotkeys()

	keysReader, err := hookkeys.NewReader()
	if err != nil {
		macrohotkey.ResumeMacroHotkeys()
		dialog.NewError(err, parent).Show()
		return
	}
	var closeKeysReader sync.Once
	closeReader := func() {
		closeKeysReader.Do(func() { keysReader.Close() })
	}

	done := make(chan struct{})
	var stopOnce sync.Once
	stop := func() { stopOnce.Do(func() { close(done) }) }

	var pendingReleaseKey string
	var finished atomic.Bool
	recordKey := func(key string) {
		if key == "" || !finished.CompareAndSwap(false, true) {
			return
		}
		fyne.Do(func() {
			keyLabel.SetText(key)
			statusLabel.SetText("")
			pendingReleaseKey = key
			onRecorded(key)
			d.Hide()
		})
	}

	// Escape is a recordable key like any other, but the OS-level hook dispatches it
	// only to the most recently registered handler. Register a topmost one so Esc is
	// captured here as the recorded key and never falls through to the action
	// tooltip's edit-mode Esc handler beneath it (which would close the editor too).
	var escOnce sync.Once
	var unregisterEsc func() = func() {}
	unregisterEsc = macrohotkey.RegisterEscapeHandler(func() {
		recordKey("esc")
	})

	d.SetOnClosed(func() {
		escOnce.Do(unregisterEsc)
		setKeyRecordSessionActive(false)
		stop()
		k := pendingReleaseKey
		pendingReleaseKey = ""
		if k != "" {
			kk := k
			go func() {
				defer closeReader()
				for !hookkeys.ChordFullyReleased(keysReader, []string{kk}) {
					time.Sleep(8 * time.Millisecond)
				}
				macrohotkey.ResumeMacroHotkeys()
			}()
			return
		}
		closeReader()
		macrohotkey.ResumeMacroHotkeys()
	})

	cancelBtn.OnTapped = func() {
		d.Hide()
	}

	go func() {
		tick := time.NewTicker(50 * time.Millisecond)
		defer tick.Stop()
		for {
			select {
			case <-done:
				return
			case <-tick.C:
				if finished.Load() {
					return
				}
				names := keysReader.PressedKeyNames()
				if len(names) == 0 {
					names = hook.PressedKeyNames()
				}
				var name string
				if len(names) >= 1 {
					name = names[0]
				}
				// Esc is captured by the dedicated topmost handler above; the poller
				// owns every other key.
				if name != "" && name != "esc" {
					recordKey(name)
					return
				}

				fyne.Do(func() {
					if finished.Load() {
						return
					}
					keyLabel.SetText("(no key)")
					statusLabel.SetText("Press the key you want to record.")
				})
			}
		}
	}()

	setKeyRecordSessionActive(true)
	d.Show()
}
