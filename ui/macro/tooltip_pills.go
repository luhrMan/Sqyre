package macro

import (
	"strings"

	"Sqyre/ui/actiondisplay"
	"Sqyre/ui/custom_widgets"
)

func macroKnownVariables() map[string]bool {
	return custom_widgets.KnownVariableSet(macroVariableDefs())
}

func addDisplayPill(row *pillRow, label, value, actionType string) {
	row.add(actiondisplay.NewDisplayLabeledPill(label, value, actionType, macroKnownVariables()))
}

func addInlineDisplayPill(row *pillRow, label, value, actionType string) {
	row.add(actiondisplay.NewDisplayLabeledPill(label, strings.TrimSpace(value), actionType, macroKnownVariables()))
}
