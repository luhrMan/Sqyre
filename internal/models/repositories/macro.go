package repositories

import (
	"Squire/internal/models/macro"
	"fmt"
	"strings"
	"sync"
)

type MacroRepository struct {
	macros map[string]*macro.Macro
}

var mr *MacroRepository
var macroInit sync.Once

func MacroRepo() *MacroRepository {
	macroInit.Do(func() {
		mr = &MacroRepository{
			macros: make(map[string]*macro.Macro),
		}
		mr.macros = macro.DecodeAll()
	})
	return mr
}

func (r *MacroRepository) Refresh() {
	r.macros = macro.DecodeAll()
}

func (r *MacroRepository) Get(s string) (*macro.Macro, error) {
	if v, ok := r.macros[strings.ToLower(s)]; ok {
		return v, nil
	}
	return nil, fmt.Errorf("FUCK")
}
func (r *MacroRepository) GetAll() map[string]*macro.Macro {
	return r.macros
}

func (r *MacroRepository) GetAllAsStringSlice() []string {
	keys := make([]string, len(r.macros))

	i := 0
	for _, k := range r.macros {
		keys[i] = k.Name
		i++
	}
	return keys
}

func (r *MacroRepository) Set(m *macro.Macro) {
	r.macros[strings.ToLower(m.Name)] = m
	macro.EncodeAll(r.macros)
}
func (r *MacroRepository) SetAll() error {
	e := macro.EncodeAll(r.macros)
	if e != nil {
		return e
	}
	return nil
}

func (r *MacroRepository) Delete(s string) {
	delete(r.macros, strings.ToLower(s))
	macro.EncodeAll(r.macros)
}
