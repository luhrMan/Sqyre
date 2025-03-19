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

func newRoot() *actions.Loop {
	r := actions.NewLoop(1, "root", []actions.ActionInterface{})
	r.UpdateBaseAction("", nil)
	return r
}

func NewMacro(name string, delay int, hotkey string) Macro {
	return Macro{
		Name:        name,
		Root:        newRoot(),
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
