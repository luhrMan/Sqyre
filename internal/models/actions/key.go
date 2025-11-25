package actions

import (
	"fmt"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/theme"
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
	return fmt.Sprintf("%s %s ", a.Key, UpOrDown(a.State))
}

func (a *Key) Icon() fyne.Resource {
	if a.State {
		return theme.DownloadIcon()
	}
	return theme.UploadIcon()
}

func UpOrDown(b bool) string {
	if b {
		return "down"
	}
	return "up"
}
