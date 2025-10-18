package actions

import (
	"Squire/internal/config"
	"fmt"
	"log"

	"github.com/go-vgo/robotgo"
)

type Click struct {
	*BaseAction `yaml:",inline" mapstructure:",squash"`
	Button      bool
}

func NewClick(button bool) *Click {
	return &Click{
		BaseAction: newBaseAction("click"),
		Button:     button,
	}
}

func (a *Click) Execute(ctx any) error {
	log.Printf("%s click", LeftOrRight(a.Button))
	robotgo.Click(LeftOrRight(a.Button))
	return nil
}

func (a *Click) String() string {
	return fmt.Sprintf("%s %s click", config.GetEmoji("Click"), LeftOrRight(a.Button))
}

func LeftOrRight(b bool) string {
	if b {
		return "right"
	}
	return "left"
}
