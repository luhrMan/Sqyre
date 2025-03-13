package actions

import (
	"Squire/internal/data"
	"fmt"
	"log"

	"github.com/go-vgo/robotgo"
)

type Click struct {
	baseAction        //`json:"baseaction"`
	Button     string `json:"button"`
}

func NewClick(button string) *Click {
	return &Click{
		baseAction: newBaseAction(),
		Button:     button,
	}
}

func (a *Click) Execute(ctx interface{}) error {
	log.Printf("%s click", a.Button)
	robotgo.Click(a.Button)
	return nil
}

func (a *Click) String() string {
	return fmt.Sprintf("%s %s click", data.GetEmoji("Click"), a.Button)
}
