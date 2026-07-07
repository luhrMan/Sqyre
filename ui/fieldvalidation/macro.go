package fieldvalidation

import (
	"Sqyre/internal/models"
	"Sqyre/internal/services"
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

func (c MacroContext) ValidateNumericExpression(text string) services.EntryValidation {
	return services.ValidateNumericExpression(text, c.currentMacro())
}

func (c MacroContext) ValidateCalculateExpression(text string) services.EntryValidation {
	return services.ValidateCalculateExpression(text, c.currentMacro())
}

func (c MacroContext) ValidateSetVariableValue(text string) services.EntryValidation {
	return services.ValidateSetVariableValue(text, c.currentMacro())
}

func (c MacroContext) ValidateVariableReferences(text string) services.EntryValidation {
	return services.ValidateVariableReferences(text, c.currentMacro())
}

func (c MacroContext) ResolveVariablePreview(text string) string {
	resolved, err := services.ResolveVariables(text, c.currentMacro())
	if err != nil || resolved == text {
		return ""
	}
	return "→ " + resolved
}
