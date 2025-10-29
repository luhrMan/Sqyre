package repositories

import (
	"Squire/internal/models"
	"Squire/internal/models/serialize"
	"log"
	"sync"
)

type MacroRepository[T models.Macro] struct {
	*repository[models.Macro]

	// macros map[string]*macro.Macro
}

var mr *MacroRepository[models.Macro]
var macroInit sync.Once

func MacroRepo() *MacroRepository[models.Macro] {
	macroInit.Do(func() {
		mr = &MacroRepository[models.Macro]{
			repository: &repository[models.Macro]{
				model:  "macros",
				models: make(map[string]*models.Macro),
			},
		}
		mr.models = mr.DecodeAll()
	})
	return mr
}

func (r *MacroRepository[T]) Decode(s string) (*models.Macro, error) {
	return r.m.Decode(s)
}

func (r *MacroRepository[T]) DecodeAll() map[string]*models.Macro {
	var (
		mm = make(map[string]*models.Macro)
		ss = serialize.GetViper().GetStringMap("macros")
	)
	for s, m := range ss {
		m, _ = r.Decode(s)
		mm[s] = m.(*models.Macro)
	}
	log.Printf("Successfully decoded all %v: %v", "macros", mm)
	return mm
}

// func (r *MacroRepository) Refresh() {
// 	r.macros = macro.DecodeAll()
// }
