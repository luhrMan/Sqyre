package macrocxt

import (
	"Sqyre/internal/models"
)

// Provider supplies macro-scoped variable metadata to UI widgets.
type Provider struct {
	CurrentMacro func() *models.Macro
}

func (p Provider) macro() *models.Macro {
	if p.CurrentMacro == nil {
		return nil
	}
	return p.CurrentMacro()
}

// VariableNames returns sorted variable names for completion menus.
func (p Provider) VariableNames() []string {
	m := p.macro()
	if m == nil {
		return nil
	}
	return m.CollectDefinedVariables()
}

// VariableDefs returns variable definitions with source metadata.
func (p Provider) VariableDefs() []models.VariableDef {
	m := p.macro()
	if m == nil {
		return nil
	}
	return m.CollectVariableDefs()
}
