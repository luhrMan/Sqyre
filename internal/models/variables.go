package models

import (
	"strings"

	"gopkg.in/yaml.v3"
)

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

// VariableStoreFromYAMLBytes extracts macro variables from YAML, preserving key casing.
// Viper/mapstructure lowercases map keys during macro decode; use this on raw YAML bytes instead.
func VariableStoreFromYAMLBytes(macroYAML []byte) *VariableStore {
	var raw map[string]any
	if err := yaml.Unmarshal(macroYAML, &raw); err != nil {
		return nil
	}
	return VariableStoreFromMap(raw["variables"])
}

// VariableStoreFromMap builds a store from decoded YAML (nested or flat variables block).
func VariableStoreFromMap(data any) *VariableStore {
	if data == nil {
		return nil
	}
	m, ok := data.(map[string]any)
	if !ok {
		return NewVariableStore()
	}
	if inner, ok := m["variables"].(map[string]any); ok {
		m = inner
	}
	vs := NewVariableStore()
	for k, v := range m {
		vs.Variables[k] = v
	}
	return vs
}

// NormalizeKeys collapses case-insensitive duplicate names, keeping the preferred spelling.
func (vs *VariableStore) NormalizeKeys() {
	if vs == nil || len(vs.Variables) < 2 {
		return
	}
	out := make(map[string]any, len(vs.Variables))
	canonical := make(map[string]string)
	for name, val := range vs.Variables {
		lower := strings.ToLower(name)
		if prev, ok := canonical[lower]; ok {
			chosen := preferVariableName(prev, name)
			canonical[lower] = chosen
			out[chosen] = val
			if chosen != prev {
				delete(out, prev)
			}
			continue
		}
		canonical[lower] = name
		out[name] = val
	}
	vs.Variables = out
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
