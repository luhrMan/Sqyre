package actions

import (
	"fmt"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/theme"
)

type Loop struct {
	Count           int
	*AdvancedAction `yaml:",inline" mapstructure:",squash"`
}

func NewLoop(count int, name string, subActions []ActionInterface) *Loop {
	if name == "root" {
		r := &Loop{
			AdvancedAction: newAdvancedAction(name, "loop", subActions),
			Count:          1,
		}
		r.uid = ""
		r.SetParent(nil)
		return r
	}
	return &Loop{
		AdvancedAction: newAdvancedAction(name, "loop", subActions),
		Count:          count,
	}
}

func (a *Loop) String() string {
	return fmt.Sprintf("%s | %d", a.Name, a.Count)
}

func (a *Loop) Icon() fyne.Resource {
	return theme.ViewRefreshIcon()
}
