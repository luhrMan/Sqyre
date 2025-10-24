package macro

import (
	"Squire/internal/models/actions"
	"Squire/internal/models/serialize"
	"fmt"
	"log"

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

func GetMacro(s string) (*Macro, error) {

	keystr := "macros" + "." + s // + "."
	var m = new(Macro)
	err := serialize.GetViper().UnmarshalKey(
		keystr, &m,
		viper.DecodeHook(serialize.MacroDecodeHookFunc()),
	)
	if err != nil {
		log.Println("Error unmarshalling macro:")
		return nil, err
	}
	log.Println("Unmarshalled macro: ", m.Name)
	log.Println("Unmarshalled actions: ", m.Root.SubActions)
	return m, nil
}

func GetMacros() map[string]*Macro {
	var (
		ps = make(map[string]*Macro)
		ss = serialize.GetViper().GetStringMap("macros")
	)
	for s := range ss {
		p, err := GetMacro(s)
		if err != nil {
			log.Println("macro could not be loaded: ", err)
			break
		}
		ps[s] = p
		log.Println("macro loaded", ps[s].Root)

	}
	log.Println("macros loaded", ps)
	return ps
}

func EncodeMacros(d map[string]*Macro) error {
	serialize.GetViper().Set("macros", d)
	err := serialize.GetViper().WriteConfig()
	if err != nil {
		return fmt.Errorf("error marshalling macros: %v", err)
	}
	log.Println("Successfully encoded macros")
	return nil
}
