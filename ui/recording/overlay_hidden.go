package recording

import (
	"Sqyre/internal/config"

	"fyne.io/fyne/v2"
)

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
