package models

import (
	"Squire/internal/models/actions"
)

type Macro struct {
	Name        string        `mapstructure:"name"`
	Root        *actions.Loop `mapstructure:"root"`
	GlobalDelay int           `mapstructure:"globaldelay"`
	Hotkey      []string      `mapstructure:"hotkey"`
}

// NewMacro creates a new Macro instance with the given parameters.
// The macro is initialized with an empty root loop.
func NewMacro(name string, delay int, hotkey []string) *Macro {
	return &Macro{
		Name:        name,
		Root:        actions.NewLoop(1, "root", []actions.ActionInterface{}),
		GlobalDelay: delay,
		Hotkey:      hotkey,
	}
}
