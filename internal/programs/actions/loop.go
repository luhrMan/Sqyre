package actions

import (
	"Squire/internal/config"
	"Squire/internal/utils"
	"fmt"
	"log"
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
	var progress, progressStep float64
	if a.Name == "root" {
		progressStep = (100.0 / float64(len(a.GetSubActions()))) / 100
	}

	for i := range a.Count {
		fmt.Printf("Loop iteration %d\n", i+1)
		for j, action := range a.GetSubActions() {
			if a.Name == "root" {
				progress = progressStep * float64(j+1)
				log.Println(progress)
				utils.MacroProgressBar().SetValue(progress)
			}
			if err := action.Execute(ctx); err != nil {
				return err
			}
		}
	}
	return nil
}

func (a *Loop) String() string {
	return fmt.Sprintf("%s | %s%d", a.Name, config.GetEmoji("Loop"), a.Count)
}

func (a *Loop) GetType() string { return a.Type }
