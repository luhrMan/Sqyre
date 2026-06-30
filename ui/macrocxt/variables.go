package macrocxt

import "Sqyre/internal/models"

// VariableNames resolves the variable name list for entry widgets.
// override, when non-nil, takes precedence over the provider.
func VariableNames(p Provider, override func() []string) []string {
	if override != nil {
		return override()
	}
	return p.VariableNames()
}

// VariableDefs resolves variable definitions for rich pickers and validation.
func VariableDefs(p Provider, override func() []models.VariableDef) []models.VariableDef {
	if override != nil {
		return override()
	}
	return p.VariableDefs()
}
