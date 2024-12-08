package actions

import (
        "Dark-And-Darker/internal/utils"
        "fmt"
        "github.com/go-vgo/robotgo"
        "log"
)

type Wait struct {
        baseAction     //`json:"baseaction"`
        Time       int `json:"waittime"`
}

func NewWait(time int) *Wait {
        return &Wait{
                baseAction: newBaseAction(),
                Time:       time,
        }
}

func (a *Wait) Execute(ctx interface{}) error {
        log.Printf("Waiting for %d milliseconds", a.Time)
        robotgo.MilliSleep(a.Time)
        return nil
}

func (a *Wait) String() string {
        return fmt.Sprintf("%s Wait for %d ms", utils.GetEmoji("Wait"), a.Time)
}
