package actions

import (
	"Squire/internal/data"
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
		r.SetUID("")
		r.SetParent(nil)
		return r
	}
	return &Loop{
		AdvancedAction: newAdvancedAction(name, "loop", subActions),
		Count:          count,
	}
}

func (a *Loop) Execute(ctx any) error {
	for i := range a.Count {
		fmt.Printf("Loop iteration %d\n", i+1)
		for _, action := range a.GetSubActions() {
			if err := action.Execute(ctx); err != nil {
				return err
			}
		}
	}
	return nil
}

func (a *Loop) String() string {
	return fmt.Sprintf("%s | %s%d", a.Name, data.GetEmoji("Loop"), a.Count)
}

func (a *Loop) GetType() string { return a.Type }
