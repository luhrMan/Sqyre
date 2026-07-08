package actiondisplay

import (
	"strings"

	"Sqyre/ui/custom_widgets"

	"fyne.io/fyne/v2"
)

// NewDisplayVariablePill renders a labeled pill whose value is a variable name chip.
func NewDisplayVariablePill(label, varName, actionType string, known map[string]bool) fyne.CanvasObject {
	varName = strings.TrimSpace(varName)
	valuePart := custom_widgets.BuildVariableNamePillContent(varName, known)
	if label == "" {
		return PillChrome(valuePart, actionType)
	}
	return PillChrome(NewPillInlineField(label+": ", valuePart), actionType)
}
