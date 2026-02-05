package models

import "strings"

// VariableStore manages variables for a macro
type VariableStore struct {
	Variables map[string]any `yaml:"variables"`
}

// NewVariableStore creates a new VariableStore
func NewVariableStore() *VariableStore {
	return &VariableStore{
		Variables: make(map[string]any),
	}
}

// Set sets a variable value (name is trimmed so "foundX" and " foundX " match)
func (vs *VariableStore) Set(name string, value any) {
	if vs.Variables == nil {
		vs.Variables = make(map[string]any)
	}
	name = strings.TrimSpace(name)
	if name == "" {
		return
	}
	vs.Variables[name] = value
}

// Get retrieves a variable value
func (vs *VariableStore) Get(name string) (any, bool) {
	if vs.Variables == nil {
		return nil, false
	}
	val, ok := vs.Variables[name]
	return val, ok
}

// Clear removes all variables
func (vs *VariableStore) Clear() {
	vs.Variables = make(map[string]any)
}

// Has checks if a variable exists
func (vs *VariableStore) Has(name string) bool {
	if vs.Variables == nil {
		return false
	}
	_, ok := vs.Variables[name]
	return ok
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
