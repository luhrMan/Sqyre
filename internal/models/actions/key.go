package actions

import (
	"Squire/internal/config"
	"fmt"
	"log"

	"github.com/go-vgo/robotgo"
)

type Key struct {
	*BaseAction `yaml:",inline" mapstructure:",squash"`
	Key         string
	State       bool
}

func NewKey(key string, state bool) *Key {
	return &Key{
		BaseAction: newBaseAction("key"),
		Key:        key,
		State:      state,
	}
}

func (a *Key) Execute(ctx any) error {
	log.Printf("Key: %s %s", a.Key, a.State)
	if a.State {
		err := robotgo.KeyDown(a.Key)
		if err != nil {
			return err
		}
	} else {
		err := robotgo.KeyUp(a.Key)
		if err != nil {
			return err
		}
	}
	return nil
}

func (a *Key) String() string {
	return fmt.Sprintf("%s Key: %s %s ", config.GetEmoji("Key"), a.Key, UpOrDown(a.State))
}

func UpOrDown(b bool) string {
	if b {
		return "down"
	}
	return "up"
}
