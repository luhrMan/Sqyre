package macro

import (
	"Squire/encoding"
	"Squire/internal/config"
	"Squire/internal/models/actions"
	"Squire/internal/utils"
	"fmt"
	"log"
	"slices"

	"fyne.io/fyne/v2"
	// hook "github.com/robotn/gohook"
	hook "github.com/luhrMan/gohook"
	"github.com/spf13/viper"
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

func (m *Macro) UnmarshalMacro(keystr string) error {
	err := config.ViperConfig.UnmarshalKey(
		keystr, &m,
		viper.DecodeHook(encoding.MacroDecodeHookFunc()),
	)
	if err != nil {
		log.Println("Error unmarshalling macro:")
		return err
	}
	log.Println("Unmarshalled macro: ", m.Name)
	log.Println("Unmarshalled actions: ", m.Root.SubActions)
	return nil
}

func EncodeMacros(d map[string]*Macro) error {
	config.ViperConfig.Set("macros", d)
	err := config.ViperConfig.WriteConfig()
	if err != nil {
		return fmt.Errorf("error marshalling macros: %v", err)
	}
	log.Println("Successfully encoded macros")
	return nil
}

func (m *Macro) HotkeyCallback() func(e hook.Event) {
	return func(e hook.Event) {
		log.Printf("pressed %v, executing %v", m.Hotkey, m.Name)
		m.ExecuteActionTree()
	}
}

func (m *Macro) RegisterHotkey() {
	hk := m.Hotkey
	if slices.Equal(hk, []string{}) {
		log.Println("do not register empty hotkeys!")
		return
	}
	log.Printf("registering hotkey %v for %v", hk, m.Name)
	hook.Register(hook.KeyDown, hk, func(e hook.Event) {
		log.Printf("pressed %v, executing %v", hk, m.Name)
		m.ExecuteActionTree()
	})
}
func (m *Macro) UnregisterHotkey() {
	hk := m.Hotkey
	log.Println("unregistering hotkey:", hk)
	hook.Unregister(hook.KeyDown, hk)
}

func GetMacro(s string) *Macro {
	keyStr := "macros" + "." + s // + "."
	var m = new(Macro)
	m.UnmarshalMacro(keyStr)
	return m
}

func GetMacros() map[string]*Macro {
	var (
		ps = make(map[string]*Macro)
		ss = config.ViperConfig.GetStringMap("macros")
	)
	for s := range ss {
		p := GetMacro(s)
		ps[s] = p
		log.Println("macro loaded", ps[s].Root)

	}
	log.Println("macros loaded", ps)
	return ps
}
