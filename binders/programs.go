package binders

import (
	"Squire/internal/config"
	"Squire/internal/models/program"
	"Squire/internal/models/repositories"

	"fyne.io/fyne/v2/data/binding"
)

type ProgramBinding struct {
	*program.Program
	ItemBindings       map[string]binding.Struct
	PointsBindings     map[string]binding.Struct
	SearchAreaBindings map[string]binding.Struct
}

func InitPrograms() {
	once.Do(func() {
		boundPrograms = map[string]*ProgramBinding{}
		BindPrograms()
	})
}

func GetBoundPrograms() map[string]*ProgramBinding {
	return boundPrograms
}

func BindPrograms() {
	for _, program := range repositories.ProgramRepo().GetAll() {
		BindProgram(program)
	}
}

func BindProgram(p *program.Program) {
	boundPrograms[p.Name] = &ProgramBinding{
		Program:            p,
		PointsBindings:     map[string]binding.Struct{},
		SearchAreaBindings: map[string]binding.Struct{},
		ItemBindings:       map[string]binding.Struct{},
	}
	for s, point := range p.Coordinates[config.MainMonitorSizeString].Points {
		boundPrograms[p.Name].PointsBindings[s] = binding.BindStruct(&point)
	}
	for s, sa := range p.Coordinates[config.MainMonitorSizeString].SearchAreas {
		boundPrograms[p.Name].SearchAreaBindings[s] = binding.BindStruct(&sa)
	}
	for s, i := range p.GetItemsMap() {
		boundPrograms[p.Name].ItemBindings[s] = binding.BindStruct(i)
	}
}
