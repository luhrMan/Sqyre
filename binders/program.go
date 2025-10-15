package binders

import (
	"Squire/internal/config"
	model "Squire/internal/programs"
	"Squire/internal/programs/macro"

	"fyne.io/fyne/v2/data/binding"
)

type ProgramBinding struct {
	*model.Program
	ItemBindings       []binding.Struct
	PointsBindings     []binding.Struct
	SearchAreaBindings []binding.Struct
}

func InitPrograms() {
	once.Do(func() {
		programs = model.GetPrograms()
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

func BindProgram(p *model.Program) {
	boundPrograms[p.Name] = &ProgramBinding{
		Program:            p,
		PointsBindings:     []binding.Struct{},
		SearchAreaBindings: []binding.Struct{},
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
}

func GetProgram(s string) *model.Program {
	if p, ok := GetPrograms()[s]; ok {
		return p
	}
	return nil
}

func GetPrograms() map[string]*model.Program {
	return programs
}

func GetBoundPrograms() map[string]*ProgramBinding {
	return boundPrograms
}
