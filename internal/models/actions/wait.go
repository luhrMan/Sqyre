package actions

import (
	"Squire/internal/config"
	"fmt"
	"log"

	"github.com/go-vgo/robotgo"
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

func (a *Wait) Execute(ctx any) error {
	log.Printf("Waiting for %d milliseconds", a.Time)
	robotgo.MilliSleep(a.Time)
	return nil
}

func (a *Wait) String() string {
	return fmt.Sprintf("%s Wait for %d ms", config.GetEmoji("Wait"), a.Time)
}
