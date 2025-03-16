package internal

import (
	"Squire/internal/actions"
	"log"
)

type Macro struct {
	Name string

	Root        *actions.Loop
	GlobalDelay int
	Hotkey      string
}

func NewMacro(name string, root *actions.Loop, delay int, hotkey string) *Macro {
	return &Macro{
		Name:        name,
		Root:        root,
		GlobalDelay: delay,
		Hotkey:      hotkey,
	}
}

func (m *Macro) SetHotkey(s string) {
	m.Hotkey = s
}

func (m *Macro) ExecuteActionTree(ctx ...any) { //error
	err := m.Root.Execute(ctx)
	if err != nil {
		log.Println(err)
		return
	}
}
