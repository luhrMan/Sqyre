package macro

import (
	"Squire/encoding"
	"Squire/internal/config"
	"Squire/internal/programs/actions"
	"Squire/internal/utils"
	"log"
	"strconv"

	"fyne.io/fyne/v2"
	// hook "github.com/robotn/gohook"
	hook "github.com/luhrMan/gohook"
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

func (m *Macro) ExecuteActionTree(ctx ...any) { //error
	go func() {
		fyne.Do(func() {
			utils.MacroActiveIndicator().Show()
			utils.MacroActiveIndicator().Start()
		})
		err := m.Root.Execute(ctx)
		if err != nil {
			log.Println(err)
			return
		}
		fyne.Do(func() {
			utils.MacroActiveIndicator().Stop()
			utils.MacroActiveIndicator().Hide()
		})
	}()
}

func (m *Macro) UnmarshalMacro(i int) error {
	log.Println("Unmarshalling macro", m.Name)
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

func (m *Macro) HotkeyCallback() func(e hook.Event) {
	return func(e hook.Event) {
		log.Printf("pressed %v, executing %v", m.Hotkey, m.Name)
		m.ExecuteActionTree()
	}
}

// func (m *Macro) RegisterHotkey() {
// 	hk := m.Hotkey
// 	if slices.Equal(hk, []string{}) {
// 		log.Println("do not register empty hotkeys!")
// 		return
// 	}
// 	log.Printf("registering hotkey %v for %v", hk, m.Name)
// 	hook.Register(hook.KeyDown, hk, func(e hook.Event) {
// 		log.Printf("pressed %v, executing %v", hk, m.Name)
// 		m.ExecuteActionTree()
// 	})
// }
// func (m *Macro) UnregisterHotkey() {
// 	hk := m.Hotkey
// 	log.Println("unregistering hotkey:", hk)
// 	hook.Unregister(hook.KeyDown, hk)
// }
