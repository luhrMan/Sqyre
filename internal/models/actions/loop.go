package actions

import (
	"fmt"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/theme"
)

type Loop struct {
	// Count is the number of iterations: int (literal) or string (variable reference e.g. "${count}").
	Count           interface{}
	*AdvancedAction `yaml:",inline" mapstructure:",squash"`
}

func NewLoop(count interface{}, name string, subActions []ActionInterface) *Loop {
	if name == "root" {
		r := &Loop{
			AdvancedAction: newAdvancedAction(name, "loop", subActions),
			Count:          1,
		}
		r.uid = ""
		r.SetParent(nil)
		return r
	}
	if count == nil {
		count = 1
	}
	return &Loop{
		AdvancedAction: newAdvancedAction(name, "loop", subActions),
		Count:          count,
	}
}

func (a *Loop) String() string {
	return fmt.Sprintf("%s | iterations: %v", a.Name, a.Count)
}

func (a *Loop) Icon() fyne.Resource {
	return theme.ViewRefreshIcon()
}
