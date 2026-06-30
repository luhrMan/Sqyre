package editor

import (
	"Sqyre/internal/models"
	"Sqyre/internal/services"
	"Sqyre/ui/custom_widgets"
	"Sqyre/ui/macrocxt"
)

func currentMacro() *models.Macro {
	if activeWire.MacroContext.CurrentMacro != nil {
		return activeWire.MacroContext.CurrentMacro()
	}
	return nil
}

func macroVarNames() []string {
	return macrocxt.VariableNames(activeWire.MacroContext, activeWire.MacroVariables)
}

func macroVariableDefs() []models.VariableDef {
	return macrocxt.VariableDefs(activeWire.MacroContext, activeWire.MacroVariableDefs)
}

func validateNumericExpression(text string) services.EntryValidation {
	return services.ValidateNumericExpression(text, currentMacro())
}

func newValidatedCoordEntry() *custom_widgets.VarEntryField {
	return custom_widgets.NewVarEntryFieldWithDefs(macroVariableDefs, validateNumericExpression)
}

func tabValidatedFields(tab *EditorTab) []*custom_widgets.VarEntryField {
	if tab == nil || tab.Widgets == nil {
		return nil
	}
	out := make([]*custom_widgets.VarEntryField, 0, 8)
	for _, w := range tab.Widgets {
		if f, ok := w.(*custom_widgets.VarEntryField); ok {
			out = append(out, f)
		}
	}
	return out
}

func allTabFieldsValid(tab *EditorTab) bool {
	for _, f := range tabValidatedFields(tab) {
		if !f.Valid() {
			return false
		}
	}
	return true
}

