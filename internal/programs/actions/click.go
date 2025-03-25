package actions

import (
	"Squire/internal/config"
	"fmt"
	"log"

	"github.com/go-vgo/robotgo"
)

type Click struct {
	*BaseAction `yaml:",inline" mapstructure:",squash"`
	Button      string
}

func NewClick(button string) *Click {
	return &Click{
		BaseAction: newBaseAction("click"),
		Button:     button,
	}
}

func (a *Click) Execute(ctx any) error {
	log.Printf("%s click", a.Button)
	robotgo.Click(a.Button)
	return nil
}

func (a *Click) String() string {
	return fmt.Sprintf("%s %s click", config.GetEmoji("Click"), a.Button)
}

func LeftOrRight(b bool) string {
	if b {
		return "right"
	}
	return "left"
}
