package actions

import (
	"Squire/internal/config"
	"fmt"
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

func (a *Key) String() string {
	return fmt.Sprintf("%s Key: %s %s ", config.GetEmoji("Key"), a.Key, UpOrDown(a.State))
}

func UpOrDown(b bool) string {
	if b {
		return "down"
	}
	return "up"
}
