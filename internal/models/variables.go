package models

import (
	"strings"
)

// VariableStore is the runtime variable store for a macro. It holds the live
// values produced while a macro executes (initial declaration values, action
// outputs, monitor builtins, etc.). It is not persisted directly; the persisted
// source of truth for user-declared variables is Macro.VariableDecls.
type VariableStore struct {
	Variables map[string]any `yaml:"variables"`
}

// NewVariableStore creates a new VariableStore
func NewVariableStore() *VariableStore {
	return &VariableStore{
		Variables: make(map[string]any),
	}
}

func preferVariableName(a, b string) string {
	aLower := a == strings.ToLower(a)
	bLower := b == strings.ToLower(b)
	if aLower && !bLower {
		return b
	}
	if bLower && !aLower {
		return a
	}
	return a
}

func (vs *VariableStore) findKey(name string) (string, bool) {
	if vs.Variables == nil {
		return "", false
	}
	if _, ok := vs.Variables[name]; ok {
		return name, true
	}
	lower := strings.ToLower(name)
	for k := range vs.Variables {
		if strings.ToLower(k) == lower {
			return k, true
		}
	}
	return "", false
}

// Set sets a variable value (name is trimmed so "foundX" and " foundX " match).
// Reuses an existing key when the name matches case-insensitively.
func (vs *VariableStore) Set(name string, value any) {
	if vs.Variables == nil {
		vs.Variables = make(map[string]any)
	}
	name = strings.TrimSpace(name)
	if name == "" {
		return
	}
	if existing, ok := vs.findKey(name); ok {
		delete(vs.Variables, existing)
	}
	vs.Variables[name] = value
}

// Get retrieves a variable value (case-insensitive name match).
func (vs *VariableStore) Get(name string) (any, bool) {
	if vs.Variables == nil {
		return nil, false
	}
	if val, ok := vs.Variables[name]; ok {
		return val, true
	}
	lower := strings.ToLower(name)
	for k, v := range vs.Variables {
		if strings.ToLower(k) == lower {
			return v, true
		}
	}
	return nil, false
}

// Clear removes all variables
func (vs *VariableStore) Clear() {
	vs.Variables = make(map[string]any)
}

// Has checks if a variable exists (case-insensitive).
func (vs *VariableStore) Has(name string) bool {
	_, ok := vs.Get(name)
	return ok
}

// Delete removes a variable by name (case-insensitive).
func (vs *VariableStore) Delete(name string) {
	if vs.Variables == nil {
		return
	}
	if existing, ok := vs.findKey(name); ok {
		delete(vs.Variables, existing)
	}
}

// GetAll returns all variable names
func (vs *VariableStore) GetAll() []string {
	if vs.Variables == nil {
		return []string{}
	}
	names := make([]string, 0, len(vs.Variables))
	for name := range vs.Variables {
		names = append(names, name)
	}
	return names
}
