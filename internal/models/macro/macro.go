package macro

import (
	"Squire/internal/models/actions"
)

type Macro struct {
	Name        string        `mapstructure:"name"`
	Root        *actions.Loop `mapstructure:"root"`
	GlobalDelay int           `mapstructure:"globaldelay"`
	Hotkey      []string      `mapstructure:"hotkey"`
}

func NewMacro(name string, delay int, hotkey []string) *Macro {
	return &Macro{
		Name:        name,
		Root:        actions.NewLoop(1, "root", []actions.ActionInterface{}),
		GlobalDelay: delay,
		Hotkey:      hotkey,
	}
}
