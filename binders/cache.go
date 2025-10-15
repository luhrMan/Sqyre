package binders

import (
	model "Squire/internal/programs"
	"Squire/internal/programs/macro"
	"sync"
)

var (
	programs      map[string]*model.Program
	boundPrograms map[string]*ProgramBinding
	boundMacros   map[string]*MacroBinding
	once          sync.Once
	macros        map[string]*macro.Macro
)
