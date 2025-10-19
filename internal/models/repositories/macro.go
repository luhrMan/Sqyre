package repositories

import (
	"Squire/internal/models/macro"

	"fmt"
	"log"

	"github.com/spf13/viper"
)

func GetMacro(s string) (*macro.Macro, error) {
	keystr := "macros" + "." + s // + "."
	var m = new(macro.Macro)
	err := v.UnmarshalKey(
		keystr, &m,
		viper.DecodeHook(MacroDecodeHookFunc()),
	)
	if err != nil {
		log.Println("Error unmarshalling macro:")
		return nil, err
	}
	log.Println("Unmarshalled macro: ", m.Name)
	log.Println("Unmarshalled actions: ", m.Root.SubActions)
	return m, nil
}

func GetMacros() map[string]*macro.Macro {
	var (
		ps = make(map[string]*macro.Macro)
		ss = v.GetStringMap("macros")
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

func EncodeMacros(d map[string]*macro.Macro) error {
	v.Set("macros", d)
	err := v.WriteConfig()
	if err != nil {
		return fmt.Errorf("error marshalling macros: %v", err)
	}
	log.Println("Successfully encoded macros")
	return nil
}
