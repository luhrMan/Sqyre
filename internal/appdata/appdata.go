package appdata

import (
	"Sqyre/internal/models"
)

// ProgramStore is the program aggregate store used by the UI.
type ProgramStore interface {
	Get(name string) (*models.Program, error)
	Set(name string, p *models.Program) error
	Delete(name string) error
	GetAll() map[string]*models.Program
	GetAllKeys() []string
	New() *models.Program
	GetAllSortedByName() []*models.Program
}

// MacroStore is the macro aggregate store used by the UI.
type MacroStore interface {
	Get(name string) (*models.Macro, error)
	Set(name string, m *models.Macro) error
	Delete(name string) error
	GetAll() map[string]*models.Macro
	GetAllKeys() []string
	New() *models.Macro
}

var (
	programs ProgramStore
	macros   MacroStore
)

// Programs returns the active program store (set by Register).
func Programs() ProgramStore {
	if programs == nil {
		panic("appdata: Programs not registered")
	}
	return programs
}

// Macros returns the active macro store (set by Register).
func Macros() MacroStore {
	if macros == nil {
		panic("appdata: Macros not registered")
	}
	return macros
}

// Register installs program/macro stores and wires models.PersistProgram for nested repositories.
func Register(p ProgramStore, m MacroStore) {
	programs = p
	macros = m
	models.PersistProgram = func(prog *models.Program) error {
		return programs.Set(prog.GetKey(), prog)
	}
	ensureDefaultProgram(p)
}
