package binders

import (
	"sync"
)

var (
	boundPrograms map[string]*ProgramBinding
	// boundMacros   BoundMacros
	once sync.Once
	// macros map[string]*macro.Macro
)
