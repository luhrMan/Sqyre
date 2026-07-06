package custom_widgets

import (
	"sync"

	"Sqyre/internal/macrohotkey"
	"Sqyre/ui/recording"

	"fyne.io/fyne/v2"
)

var (
	tooltipEscMu         sync.Mutex
	tooltipEscUnregister func()
)

// ActivateTooltipEscapeDismiss registers a global Escape handler that dismisses the active tooltip.
// Only one tooltip dismiss callback is active at a time; a new activation replaces the previous.
func ActivateTooltipEscapeDismiss(dismiss func()) {
	if dismiss == nil {
		return
	}
	tooltipEscMu.Lock()
	defer tooltipEscMu.Unlock()
	deactivateTooltipEscapeLocked()
	d := dismiss
	tooltipEscUnregister = macrohotkey.RegisterEscapeHandler(func() {
		if recording.KeyRecordSessionActive() {
			return
		}
		fyne.Do(d)
	})
}

// DeactivateTooltipEscapeDismiss removes the active tooltip Escape handler, if any.
func DeactivateTooltipEscapeDismiss() {
	tooltipEscMu.Lock()
	defer tooltipEscMu.Unlock()
	deactivateTooltipEscapeLocked()
}

func deactivateTooltipEscapeLocked() {
	if tooltipEscUnregister != nil {
		tooltipEscUnregister()
		tooltipEscUnregister = nil
	}
}
