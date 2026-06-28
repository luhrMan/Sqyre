package services

import "fyne.io/fyne/v2"

// MacroPauseStatus describes the in-run pause banner shown while a Pause action waits.
type MacroPauseStatus struct {
	Active      bool
	Message     string
	ContinueKey string
}

var macroPauseStatusCallback func(MacroPauseStatus)

// SetMacroPauseStatusCallback registers the UI handler for the macro pause banner.
// The callback is always invoked on the Fyne UI thread.
func SetMacroPauseStatusCallback(fn func(MacroPauseStatus)) {
	macroPauseStatusCallback = fn
}

// NotifyMacroPause updates the pause banner. Safe to call from any goroutine.
func NotifyMacroPause(active bool, message, continueKey string) {
	if macroPauseStatusCallback == nil {
		return
	}
	fyne.Do(func() {
		macroPauseStatusCallback(MacroPauseStatus{
			Active:      active,
			Message:     message,
			ContinueKey: continueKey,
		})
	})
}
