package actiondisplay

import (
	"image/color"
	"strings"

	"Sqyre/internal/models/actions"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/canvas"
	"fyne.io/fyne/v2/container"
	"fyne.io/fyne/v2/theme"
)

func Display(action actions.ActionInterface) fyne.CanvasObject {
	return DisplayFromParams(action.Params())
}

func DisplayFromParams(params []actions.Param) fyne.CanvasObject {
	line := container.NewHBox()
	actionType := actions.ActionTypeFromParams(params)
	for _, p := range params {
		if strings.EqualFold(p.Label, "Type") {
			continue
		}
		if entry := actions.FormatParamEntry(p); entry != "" {
			line.Add(NewDisplayPill(entry, actionType))
		}
	}
	return line
}

// NewDisplayPill renders a rounded label chip using the pastel color for actionType.
func NewDisplayPill(text string, actionType string) fyne.CanvasObject {
	label := canvas.NewText(text, theme.Color(theme.ColorNameForeground))
	label.TextSize = theme.CaptionTextSize()
	fill := actionPillColor(actionType)
	bg := canvas.NewRectangle(fill)
	bg.StrokeColor = theme.Color(theme.ColorNameSeparator)
	bg.StrokeWidth = 0.5
	bg.CornerRadius = 6
	leftPad := canvas.NewRectangle(color.Transparent)
	leftPad.SetMinSize(fyne.NewSize(4, 0))
	rightPad := canvas.NewRectangle(color.Transparent)
	rightPad.SetMinSize(fyne.NewSize(4, 0))
	topPad := canvas.NewRectangle(color.Transparent)
	topPad.SetMinSize(fyne.NewSize(0, 2))
	bottomPad := canvas.NewRectangle(color.Transparent)
	bottomPad.SetMinSize(fyne.NewSize(0, 2))
	content := container.NewBorder(topPad, bottomPad, leftPad, rightPad, label)
	return container.NewStack(bg, content)
}

func actionPillColor(actionType string) color.NRGBA {
	return ActionPastelColorForApp(actionType)
}
