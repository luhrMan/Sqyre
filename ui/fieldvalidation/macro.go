package fieldvalidation

import (
	macrologic "Sqyre/internal/macro"
	"Sqyre/internal/models"
)

// MacroContext supplies macro-scoped validation inputs from UI wiring.
type MacroContext struct {
	CurrentMacro    func() *models.Macro
	VariableDefs    func() []models.VariableDef
}

func (c MacroContext) currentMacro() *models.Macro {
	if c.CurrentMacro != nil {
		return c.CurrentMacro()
	}
	return nil
}

func (c MacroContext) ValidateNumericExpression(text string) macrologic.EntryValidation {
	return macrologic.ValidateNumericExpression(text, c.currentMacro())
}

func (c MacroContext) ValidateSetVariableValue(text string) macrologic.EntryValidation {
	return macrologic.ValidateSetVariableValue(text, c.currentMacro())
}

func (c MacroContext) ValidateVariableReferences(text string) macrologic.EntryValidation {
	return macrologic.ValidateVariableReferences(text, c.currentMacro())
}

func (c MacroContext) ResolveVariablePreview(text string) string {
	resolved, err := macrologic.ResolveVariables(text, c.currentMacro())
	if err != nil || resolved == text {
		return ""
	}
	return "→ " + resolved
}
