package actions

import (
	"Squire/internal/config"
	"fmt"
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
	return fmt.Sprintf("%s | %s%d", a.Name, config.GetEmoji("Loop"), a.Count)
}

func (a *Loop) GetType() string { return a.Type }
