package actions

import (
	"fmt"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/theme"
)

type Wait struct {
	*BaseAction `yaml:",inline" mapstructure:",squash"`
	Time        int
}

func NewWait(time int) *Wait {
	return &Wait{
		BaseAction: newBaseAction("wait"),
		Time:       time,
	}
}

func (a *Wait) String() string {
	return stringifyParams(a.parameters())
}

func (a *Wait) Display() fyne.CanvasObject {
	return displayFromParams(a.parameters())
}

func (a *Wait) parameters() []actionParam {
	return []actionParam{
		newParam("Type", a.GetType()),
		newParam("Time", fmt.Sprintf("%d ms", a.Time)),
	}
}

func (a *Wait) Icon() fyne.Resource {
	return theme.HistoryIcon()
}
