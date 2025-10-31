package binders

import (
	"sync"
)

var (
	boundPrograms map[string]*ProgramBinding
	once          sync.Once
)
