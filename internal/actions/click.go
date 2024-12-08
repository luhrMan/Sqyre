package actions

import (
        "Dark-And-Darker/internal/utils"
        "fmt"
        "github.com/go-vgo/robotgo"
        "log"
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
        return fmt.Sprintf("%s %s click", utils.GetEmoji("Click"), a.Button)
}
