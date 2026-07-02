package recording

import (
	"Sqyre/internal/config"
	"time"

	"fyne.io/fyne/v2"
)

// hideBeforeCaptureDelay lets the event loop and compositor repaint after Fyne
// windows are hidden. Capture inside the same UI handler runs before that
// repaint, so screenshots would still include Sqyre.
const hideBeforeCaptureDelay = 200 * time.Millisecond

func hideAppWindowsDuringRecording(app fyne.App) []fyne.Window {
	if !hideAppDuringRecording(app) {
		return nil
	}
	return hideAppWindows(app)
}

func hideAppDuringRecording(app fyne.App) bool {
	if app == nil {
		return config.DefaultHideAppDuringRecording
	}
	return app.Preferences().BoolWithFallback(config.PrefHideAppDuringRecording, config.DefaultHideAppDuringRecording)
}

func hideAppWindows(app fyne.App) []fyne.Window {
	if app == nil {
		return nil
	}
	driver := app.Driver()
	if driver == nil {
		return nil
	}
	hidden := make([]fyne.Window, 0, len(driver.AllWindows()))
	for _, win := range driver.AllWindows() {
		if win == nil || skipHideForRecording(win) {
			continue
		}
		win.Hide()
		hidden = append(hidden, win)
	}
	return hidden
}

// scheduleAfterHidingApp runs fn on the main Fyne loop after Sqyre windows have had
// time to unmap when hidden is non-empty. When nothing was hidden, fn is still
// queued for the next loop tick so capture/window creation does not run inside the
// button handler that started recording.
func scheduleAfterHidingApp(hidden []fyne.Window, fn func()) *time.Timer {
	if len(hidden) == 0 {
		fyne.Do(fn)
		return nil
	}
	return time.AfterFunc(hideBeforeCaptureDelay, func() {
		fyne.Do(fn)
	})
}

// skipHideForRecording excludes Fyne-internal windows that must never be shown to
// the user. SystrayMonitor is created by fyne.io/fyne's GLFW driver for system
// tray lifecycle; it is created hidden and must not be restored with Show().
func skipHideForRecording(win fyne.Window) bool {
	return win.Title() == "SystrayMonitor"
}

func showAppWindows(windows []fyne.Window) {
	for _, win := range windows {
		if win != nil {
			win.Show()
		}
	}
}
