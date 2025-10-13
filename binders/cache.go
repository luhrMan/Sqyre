package binders

import (
	model "Squire/internal/programs"
	"Squire/internal/programs/macro"
	"sync"
)

var (
	programs map[string]*model.Program
	once     sync.Once
	macros   []*macro.Macro
)
