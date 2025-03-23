package actions

import (
	"Squire/internal/data"
	"fmt"
	"log"

	"github.com/go-vgo/robotgo"
)

type Key struct {
	*BaseAction `yaml:",inline" mapstructure:",squash"`
	Key         string
	State       string
}

func NewKey(key, state string) *Key {
	return &Key{
		BaseAction: newBaseAction("key"),
		Key:        key,
		State:      state,
	}
}

func (a *Key) Execute(ctx any) error {
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
	return fmt.Sprintf("%s Key: %s %s ", data.GetEmoji("Key"), a.Key, a.State)
}

func UpOrDown(b bool) string {
	if b {
		return "Down"
	}
	return "Up"
}
