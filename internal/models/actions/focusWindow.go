package actions

import (
	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/theme"
)

// FocusWindow activates a specific window identified by executable path and title.
// Both fields are stable across restarts and distinguish windows that share a process name.
type FocusWindow struct {
	*BaseAction `yaml:",inline" mapstructure:",squash"`
	// ProcessPath is the application executable path (e.g. /usr/bin/firefox).
	ProcessPath string `mapstructure:"processpath"`
	// WindowTitle is the window title at selection time (e.g. "Inbox - Mozilla Thunderbird").
	WindowTitle string `mapstructure:"windowtitle"`
}

func NewFocusWindow(processPath, windowTitle string) *FocusWindow {
	return &FocusWindow{
		BaseAction:  newBaseAction("focuswindow"),
		ProcessPath: processPath,
		WindowTitle: windowTitle,
	}
}

func (a *FocusWindow) String() string {
	return stringifyParams(a.parameters())
}

func (a *FocusWindow) Display() fyne.CanvasObject {
	return displayFromParams(a.parameters())
}

func (a *FocusWindow) parameters() []actionParam {
	title := a.WindowTitle
	if title == "" {
		title = "not set"
	}
	path := a.ProcessPath
	if path == "" {
		path = "not set"
	}
	return []actionParam{
		newParam("Type", a.GetType()),
		newParam("Title", title),
		newParam("App", path),
	}
}

func (a *FocusWindow) Icon() fyne.Resource {
	return theme.VisibilityIcon()
}
