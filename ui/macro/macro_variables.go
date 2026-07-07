package macro

import (
	"Sqyre/internal/models"
	"Sqyre/ui/macrocxt"
)

func macroVariableDefs() []models.VariableDef {
	return macrocxt.VariableDefs(activeWire.MacroContext, activeWire.MacroVariableDefs)
}
