package macro

import (
	"Squire/internal/models/actions"
	// hook "github.com/robotn/gohook"
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

// func (m *Macro) ExecuteActionTree(ctx ...any) { //error
// 	go func() {
// 		// fyne.Do(func() {
// 		// 	services.MacroActiveIndicator().Show()
// 		// 	services.MacroActiveIndicator().Start()
// 		// })
// 		err := m.Root.Execute(ctx)
// 		if err != nil {
// 			log.Println(err)
// 			return
// 		}
// 		// fyne.Do(func() {
// 		// 	services.MacroActiveIndicator().Stop()
// 		// 	services.MacroActiveIndicator().Hide()
// 		// })
// 	}()
// }

// func (m *Macro) HotkeyCallback() func(e hook.Event) {
// 	return func(e hook.Event) {
// 		log.Printf("pressed %v, executing %v", m.Hotkey, m.Name)
// 		m.ExecuteActionTree()
// 	}
// }

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
