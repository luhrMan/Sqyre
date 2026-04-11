package actionrender

import (
	"fmt"
	"image/color"
	"strings"

	"Sqyre/internal/models/actions"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/canvas"
	"fyne.io/fyne/v2/container"
	"fyne.io/fyne/v2/theme"
)

// DisplayWidget builds a Fyne widget that shows the action's parameters as colored pills.
func DisplayWidget(a actions.ActionInterface) fyne.CanvasObject {
	params := a.Parameters()
	return displayFromParams(params)
}

func displayFromParams(params []actions.ActionParam) fyne.CanvasObject {
	line := container.NewHBox()
	actionType := actionTypeFromParams(params)
	for _, p := range params {
		if strings.EqualFold(p.Label, "Type") {
			continue
		}
		value := strings.TrimSpace(fmt.Sprintf("%v", p.Value))
		if value == "" {
			continue
		}
		line.Add(newParamPill(fmt.Sprintf("%s: %s", p.Label, value), actionType))
	}
	return line
}

func newParamPill(text string, actionType string) fyne.CanvasObject {
	label := canvas.NewText(text, theme.Color(theme.ColorNameForeground))
	label.TextSize = 11
	fill := ActionPastelColor(actionType)
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

func actionTypeFromParams(params []actions.ActionParam) string {
	for _, p := range params {
		if strings.EqualFold(p.Label, "Type") {
			return strings.TrimSpace(fmt.Sprintf("%v", p.Value))
		}
	}
	return ""
}
