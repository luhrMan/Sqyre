package actions

import (
	"Squire/internal/data"
	"fmt"
	"log"

	"github.com/go-vgo/robotgo"
)

type Wait struct {
	baseAction
	Time int `json:"waittime"`
}

func NewWait(time int) *Wait {
	return &Wait{
		baseAction: newBaseAction(),
		Time:       time,
	}
}

func (a *Wait) Execute(ctx any) error {
	log.Printf("Waiting for %d milliseconds", a.Time)
	robotgo.MilliSleep(a.Time)
	return nil
}

func (a *Wait) String() string {
	return fmt.Sprintf("%s Wait for %d ms", data.GetEmoji("Wait"), a.Time)
}
