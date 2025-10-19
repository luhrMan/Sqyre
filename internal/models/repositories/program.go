package repositories

import (
	"Squire/internal/models/coordinates"
	"Squire/internal/models/items"
	"Squire/internal/models/program"
	"fmt"
	"log"
)

func GetProgram(s string) *program.Program {
	var (
		keyStr = "programs" + "." + s + "."
		err    error
		errStr = "problem here lol"
	)
	var p = &program.Program{
		Items:       map[string]*items.Item{},
		Coordinates: map[string]*coordinates.Coordinates{},
	}
	err = v.UnmarshalKey(keyStr+"name", &p.Name)
	if err != nil {
		log.Fatalf(errStr, err)
	}

	err = v.UnmarshalKey(keyStr+"items", &p.Items)
	if err != nil {
		log.Fatalf(errStr, err)
	}

	err = v.UnmarshalKey(keyStr+"coordinates", &p.Coordinates)
	if err != nil {
		log.Fatalf(errStr, err)
	}

	return p
}

func GetPrograms() map[string]*program.Program {
	var (
		ps = make(map[string]*program.Program)
		ss = v.GetStringMap("programs")
	)
	for s := range ss {
		p := GetProgram(s)
		ps[s] = p
	}
	log.Println("programs loaded", ps)
	return ps
}

func EncodePrograms(d map[string]*program.Program) error {
	v.Set("programs", d)
	err := v.WriteConfig()
	if err != nil {
		return fmt.Errorf("error marshalling programs: %v", err)
	}
	log.Println("Successfully encoded programs")
	return nil
}
