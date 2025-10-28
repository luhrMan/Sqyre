package repositories

import (
	"Squire/internal/models/macro"
	"sync"
)

type MacroRepository[T macro.Macro] struct {
	*repository[T]

	// macros map[string]*macro.Macro
}

var mr *MacroRepository[macro.Macro]
var macroInit sync.Once

func MacroRepo() *MacroRepository[macro.Macro] {
	macroInit.Do(func() {
		mr = &MacroRepository[macro.Macro]{
			repository: &repository[macro.Macro]{
				model:  "macros",
				models: make(map[string]*macro.Macro),
			},
		}
		mr.models = macro.DecodeAll()
	})
	return mr
}

// func (r *MacroRepository) Refresh() {
// 	r.macros = macro.DecodeAll()
// }
