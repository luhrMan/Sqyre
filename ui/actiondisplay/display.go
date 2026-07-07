package actiondisplay

import (
	"image/color"

	"Sqyre/internal/models/actions"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/container"
)

func Display(action actions.ActionInterface, known map[string]bool) fyne.CanvasObject {
	line, _, _ := DisplayFromParams(action.Params(), known)
	return line
}

// DisplayFromParams builds inline summary pills and returns extra params for tooltips.
func DisplayFromParams(params []actions.Param, known map[string]bool) (line fyne.CanvasObject, extra []actions.Param, actionType string) {
	summary, extra := actions.DisplayParams(params)
	actionType = actions.ActionTypeFromParams(params)
	box := container.NewHBox()
	for _, p := range summary {
		if text := actions.FormatParamMinimal(p); text != "" {
			box.Add(NewDisplayValuePill(text, actionType, known))
		}
	}
	return box, extra, actionType
}

// NewDisplayPill renders a rounded label chip using the pastel color for actionType.
func NewDisplayPill(text string, actionType string) fyne.CanvasObject {
	return PillChrome(NewPillText(text), actionType)
}

func actionPillColor(actionType string) color.NRGBA {
	return ActionPastelColorForApp(actionType)
}
