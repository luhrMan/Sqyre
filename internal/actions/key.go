package actions

import (
        "Squire/internal/utils"
        "fmt"
        "github.com/go-vgo/robotgo"
        "log"
)

type Key struct {
        baseAction        //`json:"baseaction"`
        Key        string `json:"key"`
        State      string `json:"state"`
}

func NewKey(key, state string) *Key {
        return &Key{
                baseAction: newBaseAction(),
                Key:        key,
                State:      state,
        }
}

func (a *Key) Execute(ctx interface{}) error {
        log.Printf("Key: %s %s", a.Key, a.State)
        switch a.State {
        case "Up":
                err := robotgo.KeyUp(a.Key)
                if err != nil {
                        return err
                }
        case "Down":
                err := robotgo.KeyDown(a.Key)
                if err != nil {
                        return err
                }
        }
        return nil
}

func (a *Key) String() string {
        return fmt.Sprintf("%s Key: %s %s ", utils.GetEmoji("Key"), a.Key, a.State)
}
