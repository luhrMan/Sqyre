package recording

import (
	"fmt"
	"slices"
	"strings"
	"sync"
	"sync/atomic"
	"time"

	"Sqyre/internal/services"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/container"
	"fyne.io/fyne/v2/dialog"
	"fyne.io/fyne/v2/layout"
	"fyne.io/fyne/v2/theme"
	"fyne.io/fyne/v2/widget"
	hook "github.com/luhrMan/gohook"
)

// ShowHotkeyRecordDialog shows keys currently held (via gohook). If the same non-empty
// set stays held for stableDuration without changing, onRecorded runs and the dialog closes.
// Esc dismisses without calling onRecorded.
func ShowHotkeyRecordDialog(
	parent fyne.Window,
	stableDuration time.Duration,
	addDialogEscapeClose func(d dialog.Dialog, parent fyne.Window),
	onRecorded func(keys []string),
) {
	if parent == nil || onRecorded == nil {
		return
	}

	sec := int(stableDuration.Round(time.Second) / time.Second)
	if sec < 1 {
		sec = 1
	}
	hint := widget.NewLabel(fmt.Sprintf("Hold your hotkey. When it stays unchanged for %d seconds, it will be saved.\nPress Esc to cancel.", sec))
	hint.Wrapping = fyne.TextWrapWord

	keysLabel := widget.NewLabel("(no keys)")
	keysLabel.Wrapping = fyne.TextWrapWord
	keysLabel.TextStyle = fyne.TextStyle{Monospace: true}

	statusLabel := widget.NewLabel("")

	progress := widget.NewProgressBar()
	progress.Max = stableDuration.Seconds()

	cancelBtn := widget.NewButtonWithIcon("Cancel", theme.CancelIcon(), nil)
	cancelBtn.Importance = widget.MediumImportance

	content := container.NewVBox(
		hint,
		widget.NewSeparator(),
		keysLabel,
		progress,
		statusLabel,
		container.NewHBox(layout.NewSpacer(), cancelBtn),
	)

	d := dialog.NewCustomWithoutButtons("Record hotkey", content, parent)
	if addDialogEscapeClose != nil {
		addDialogEscapeClose(d, parent)
	}

	done := make(chan struct{})
	var stopOnce sync.Once
	stop := func() { stopOnce.Do(func() { close(done) }) }
	d.SetOnClosed(stop)

	// If we saved while keys were still held, resume macro hooks only after that chord is
	// released; otherwise the newly registered hotkey fires immediately.
	var pendingResumeChord []string
	d.SetOnClosed(func() {
		k := pendingResumeChord
		pendingResumeChord = nil
		if len(k) > 0 {
			kk := append([]string(nil), k...)
			go func() {
				for !hook.ChordFullyReleased(kk) {
					time.Sleep(8 * time.Millisecond)
				}
				services.ResumeMacroHotkeys()
			}()
			return
		}
		services.ResumeMacroHotkeys()
	})

	services.SuspendMacroHotkeys()

	cancelBtn.OnTapped = func() {
		d.Hide()
	}

	var lastNames []string
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
				names := hook.PressedKeyNames()
				if !slices.Equal(names, lastNames) {
					lastNames = append([]string(nil), names...)
					stableStart = time.Now()
				}
				elapsed := time.Since(stableStart)
				display := strings.Join(names, " + ")
				if display == "" {
					display = "(no keys)"
				}
				var prog float64
				var status string
				if len(names) > 0 {
					prog = elapsed.Seconds()
					if prog > progress.Max {
						prog = progress.Max
					}
					remain := stableDuration - elapsed
					if remain < 0 {
						remain = 0
					}
					status = fmt.Sprintf("Stable for %.1f s — %.1f s until save", elapsed.Seconds(), remain.Seconds())
				} else {
					status = "Press and hold your hotkey combination."
				}

				fyne.Do(func() {
					if finished.Load() {
						return
					}
					keysLabel.SetText(display)
					progress.SetValue(prog)
					statusLabel.SetText(status)
				})

				if len(names) > 0 && elapsed >= stableDuration && finished.CompareAndSwap(false, true) {
					keysCopy := append([]string(nil), names...)
					fyne.Do(func() {
						pendingResumeChord = append([]string(nil), keysCopy...)
						onRecorded(keysCopy)
						d.Hide()
					})
					return
				}
			}
		}
	}()

	d.Show()
}
