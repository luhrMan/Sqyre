package ui

import (
	"sync/atomic"

	"fyne.io/fyne/v2"
)

var appInForeground atomic.Bool

// InitAppForegroundTracking registers lifecycle hooks so global hotkeys can tell
// whether Sqyre is the foreground application.
func InitAppForegroundTracking(app fyne.App) {
	if app == nil {
		return
	}
	appInForeground.Store(true)
	app.Lifecycle().SetOnEnteredForeground(func() {
		appInForeground.Store(true)
	})
	app.Lifecycle().SetOnExitedForeground(func() {
		appInForeground.Store(false)
	})
}

// AppInForeground reports whether Sqyre currently has foreground input focus.
func AppInForeground() bool {
	return appInForeground.Load()
}

// SetAppInForegroundForTesting sets foreground state in unit tests.
func SetAppInForegroundForTesting(inForeground bool) {
	appInForeground.Store(inForeground)
}
