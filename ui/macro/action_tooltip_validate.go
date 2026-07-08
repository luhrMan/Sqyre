package macro

import (
	"Sqyre/internal/models"
	"Sqyre/internal/models/actions"
	"Sqyre/internal/validation"
)

func validateTooltipAction(node actions.ActionInterface) error {
	var macro *models.Macro
	if activeWire.MacroContext.CurrentMacro != nil {
		macro = activeWire.MacroContext.CurrentMacro()
	}
	return validation.ValidateAction(node, macro)
}
