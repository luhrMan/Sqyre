package binders

import (
	"Squire/internal/config"
	"Squire/internal/models/macro"
	"Squire/internal/models/program"
	"log"

	"fyne.io/fyne/v2/data/binding"
)

type ProgramBinding struct {
	*program.Program
	ItemBindings       []binding.Struct
	PointsBindings     []binding.Struct
	SearchAreaBindings []binding.Struct
}

func InitPrograms() {
	once.Do(func() {
		programs = program.GetPrograms()
		macros = macro.GetMacros()
		boundPrograms = map[string]*ProgramBinding{}
		boundMacros = map[string]*MacroBinding{}
		BindPrograms()
		BindMacros()
	})
}

func BindPrograms() {
	for _, program := range GetPrograms() {
		BindProgram(program)
	}
}

func BindProgram(p *program.Program) {
	boundPrograms[p.Name] = &ProgramBinding{
		Program:            p,
		PointsBindings:     []binding.Struct{},
		SearchAreaBindings: []binding.Struct{},
		ItemBindings:       []binding.Struct{},
	}

	points := p.Coordinates[config.MainMonitorSizeString].Points
	boundPrograms[p.Name].PointsBindings = make([]binding.Struct, len(points)) // Do not use slice append to build boundFriends list, for some reason! Strange effects...
	counter := 0
	for _, point := range points {
		boundPoint := binding.BindStruct(&point)
		boundPrograms[p.Name].PointsBindings[counter] = boundPoint
		counter++
	}

	sas := p.Coordinates[config.MainMonitorSizeString].SearchAreas
	boundPrograms[p.Name].SearchAreaBindings = make([]binding.Struct, len(sas)) // Do not use slice append to build boundFriends list, for some reason! Strange effects...
	counter = 0
	for _, sa := range sas {
		boundSA := binding.BindStruct(&sa)
		boundPrograms[p.Name].SearchAreaBindings[counter] = boundSA
		counter++
	}
	log.Println(p.Items)
	items := p.GetItemsMap()
	boundPrograms[p.Name].ItemBindings = make([]binding.Struct, len(items)) // Do not use slice append to build boundFriends list, for some reason! Strange effects...
	counter = 0
	for _, i := range items {
		boundItem := binding.BindStruct(&i)
		boundPrograms[p.Name].ItemBindings[counter] = boundItem
		counter++
	}
}

func GetProgram(s string) *program.Program {
	if p, ok := GetPrograms()[s]; ok {
		return p
	}
	return nil
}

func GetPrograms() map[string]*program.Program {
	return programs
}

func GetBoundPrograms() map[string]*ProgramBinding {
	return boundPrograms
}
