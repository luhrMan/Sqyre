//go:build !nohook

package recording

import (
	"fmt"
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

const keyRecordStableDuration = 300 * time.Millisecond

// ShowKeyRecordDialog captures a single keyboard key. When exactly one key stays
// held for keyRecordStableDuration, onRecorded runs and the dialog closes.
func ShowKeyRecordDialog(
	parent fyne.Window,
	_ func(d dialog.Dialog, parent fyne.Window),
	onRecorded func(key string),
) {
	if parent == nil || onRecorded == nil {
		return
	}

	hint := widget.NewLabel("Press the key you want to use.\nWhen one key stays held briefly, it will be saved.\nUse Cancel to dismiss without saving.")
	hint.Wrapping = fyne.TextWrapWord

	keyLabel := widget.NewLabel("(no key)")
	keyLabel.Wrapping = fyne.TextWrapWord
	keyLabel.TextStyle = fyne.TextStyle{Monospace: true}

	statusLabel := widget.NewLabel("")

	progress := widget.NewProgressBar()
	progress.Max = keyRecordStableDuration.Seconds()

	cancelBtn := widget.NewButtonWithIcon("Cancel", theme.CancelIcon(), nil)
	cancelBtn.Importance = widget.MediumImportance

	content := container.NewVBox(
		hint,
		widget.NewSeparator(),
		keyLabel,
		progress,
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
	d.SetOnClosed(func() {
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

	var lastName string
	stableStart := time.Now()
	var finished atomic.Bool

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
				if len(names) == 1 {
					name = names[0]
				}
				if name != lastName {
					lastName = name
					stableStart = time.Now()
				}
				elapsed := time.Since(stableStart)
				display := name
				if display == "" {
					display = "(no key)"
				}
				var prog float64
				var status string
				switch {
				case len(names) > 1:
					prog = 0
					status = "Release extra keys — only one key is recorded."
				case name != "":
					prog = elapsed.Seconds()
					if prog > progress.Max {
						prog = progress.Max
					}
					remain := max(keyRecordStableDuration-elapsed, 0)
					status = fmt.Sprintf("Stable for %.1f s — %.1f s until save", elapsed.Seconds(), remain.Seconds())
				default:
					status = "Press the key you want to record."
				}

				fyne.Do(func() {
					if finished.Load() {
						return
					}
					keyLabel.SetText(display)
					progress.SetValue(prog)
					statusLabel.SetText(status)
				})

				if name != "" && len(names) == 1 && elapsed >= keyRecordStableDuration && finished.CompareAndSwap(false, true) {
					keyCopy := name
					fyne.Do(func() {
						pendingReleaseKey = keyCopy
						onRecorded(keyCopy)
						d.Hide()
					})
					return
				}
			}
		}
	}()

	setKeyRecordSessionActive(true)
	d.Show()
}
