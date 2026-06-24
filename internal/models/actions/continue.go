package actions

import (
	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/theme"
)

// Continue skips the rest of the current loop iteration and advances to the next
// iteration of the innermost enclosing loop.
type Continue struct {
	*BaseAction `yaml:",inline" mapstructure:",squash"`
}

func NewContinue() *Continue {
	return &Continue{
		BaseAction: newBaseAction("continue"),
	}
}

func (a *Continue) String() string {
	return stringifyParams(a.parameters())
}

func (a *Continue) Display() fyne.CanvasObject {
	return displayFromParams(a.parameters())
}

func (a *Continue) parameters() []actionParam {
	return []actionParam{
		newParam("Type", a.GetType()),
	}
}

func (a *Continue) Icon() fyne.Resource {
	return theme.MediaSkipNextIcon()
}
