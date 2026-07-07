package actiondisplay

import (
	"strings"

	"Sqyre/ui/custom_widgets"

	"fyne.io/fyne/v2"
)

// NewDisplayValuePill renders a compact action pill whose value may contain nested variable pills.
func NewDisplayValuePill(value, actionType string, known map[string]bool) fyne.CanvasObject {
	value = strings.TrimSpace(value)
	return PillChrome(custom_widgets.BuildVarRefPillContent(value, known), actionType)
}

// NewDisplayLabeledPill renders an outer action pill with a label and value that may
// contain compact nested variable reference pills.
func NewDisplayLabeledPill(label, value, actionType string, known map[string]bool) fyne.CanvasObject {
	value = strings.TrimSpace(value)
	valuePart := custom_widgets.BuildVarRefPillContent(value, known)
	if label == "" {
		return PillChrome(valuePart, actionType)
	}
	return PillChrome(NewPillInlineField(label+": ", valuePart), actionType)
}
