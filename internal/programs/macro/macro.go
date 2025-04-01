package macro

import (
	"Squire/encoding"
	"Squire/internal/config"
	"Squire/internal/programs/actions"
	"log"
	"strconv"

	"github.com/spf13/viper"
)

type Macro struct {
	Name        string
	Root        *actions.Loop
	GlobalDelay int
	Hotkey      []string
}

func NewMacro(name string, delay int, hotkey []string) *Macro {
	return &Macro{
		Name:        name,
		Root:        actions.NewLoop(1, "root", []actions.ActionInterface{}),
		GlobalDelay: delay,
		Hotkey:      hotkey,
	}
}

func (m *Macro) SetHotkey(hk []string) {
	m.Hotkey = hk
}

func (m *Macro) ExecuteActionTree(ctx ...any) { //error
	err := m.Root.Execute(ctx)
	if err != nil {
		log.Println(err)
		return
	}
}

func (m *Macro) UnmarshalMacro(i int) error {
	log.Println("Unmarshalling macro")
	err := config.ViperConfig.UnmarshalKey(
		"programs"+"."+
			config.DarkAndDarker+"."+
			"macros"+"."+
			strconv.Itoa(i), &m,
		viper.DecodeHook(encoding.MacroDecodeHookFunc()),
	)
	if err != nil {
		log.Println("Error unmarshalling macro:")
		return err
	}
	return nil
}
