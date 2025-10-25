package repositories

import "Squire/internal/models/macro"

type MacroRepository struct {
	macros map[string]*macro.Macro
}

var mr *MacroRepository

func MacroRepo() *MacroRepository {
	return mr
}

func (r *MacroRepository) Init() {
	r = &MacroRepository{
		macros: make(map[string]*macro.Macro),
	}
	r.macros = macro.DecodeAll()
	mr = r
}
func (r *MacroRepository) Get(s string) *macro.Macro {
	return r.macros[s]
}
func (r *MacroRepository) GetAll() map[string]*macro.Macro {
	return r.macros
}
func (r *MacroRepository) Set(m *macro.Macro) {
	// r.EncodeMacro(m)
}
func (r *MacroRepository) SetAll() error {
	e := macro.EncodeAll(r.GetAll())
	if e != nil {
		return e
	}
	return nil
}
