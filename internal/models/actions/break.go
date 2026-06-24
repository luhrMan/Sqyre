package actions

import (
	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/theme"
)

// Break exits the innermost enclosing loop (Loop, ForEachRow, or ImageSearch
// per-match iteration).
type Break struct {
	*BaseAction `yaml:",inline" mapstructure:",squash"`
}

func NewBreak() *Break {
	return &Break{
		BaseAction: newBaseAction("break"),
	}
}

func (a *Break) String() string {
	return stringifyParams(a.parameters())
}

func (a *Break) Display() fyne.CanvasObject {
	return displayFromParams(a.parameters())
}

func (a *Break) parameters() []actionParam {
	return []actionParam{
		newParam("Type", a.GetType()),
	}
}

func (a *Break) Icon() fyne.Resource {
	return theme.MediaStopIcon()
}
