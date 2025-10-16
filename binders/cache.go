package binders

import (
	"Squire/internal/models/macro"
	"Squire/internal/models/program"
	"sync"
)

var (
	programs      map[string]*program.Program
	boundPrograms map[string]*ProgramBinding
	boundMacros   map[string]*MacroBinding
	once          sync.Once
	macros        map[string]*macro.Macro
)
