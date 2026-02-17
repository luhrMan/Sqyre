package actions

import (
	"fmt"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/theme"
)

// FocusWindow activates/focuses a window chosen by the user. The user can pick from
// a list of active windows (process names) or type a name (e.g. partial match) themselves.
type FocusWindow struct {
	*BaseAction `yaml:",inline" mapstructure:",squash"`
	// WindowTarget is the window/process name to focus (e.g. "chrome", "code", or a custom string).
	WindowTarget string `mapstructure:"windowtarget"`
}

func NewFocusWindow(windowTarget string) *FocusWindow {
	return &FocusWindow{
		BaseAction:   newBaseAction("focuswindow"),
		WindowTarget: windowTarget,
	}
}

func (a *FocusWindow) String() string {
	if a.WindowTarget == "" {
		return "Focus window (not set)"
	}
	return fmt.Sprintf("Focus: %s", a.WindowTarget)
}

func (a *FocusWindow) Icon() fyne.Resource {
	return theme.VisibilityIcon()
}
