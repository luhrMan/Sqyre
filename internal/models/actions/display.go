package actions

import (
	"fmt"
	"image/color"
	"strings"

	"Sqyre/internal/config"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/canvas"
	"fyne.io/fyne/v2/container"
	"fyne.io/fyne/v2/theme"
)

type actionParam struct {
	Label string
	Value any
}

func newParam(label string, value any) actionParam {
	return actionParam{Label: label, Value: value}
}

func stringifyParams(params []actionParam) string {
	parts := make([]string, 0, len(params))
	for _, p := range params {
		value := strings.TrimSpace(fmt.Sprintf("%v", p.Value))
		if value == "" {
			continue
		}
		parts = append(parts, fmt.Sprintf("%s: %s", p.Label, value))
	}
	return strings.Join(parts, " "+config.DescriptionDelimiter+" ")
}

func formatSearchAreaLabel(area SearchArea) string {
	name := strings.TrimSpace(area.Name)
	coordinates := fmt.Sprintf(
		"TopY: %v, LeftX: %v, BottomY: %v, RightX: %v",
		area.TopY,
		area.LeftX,
		area.BottomY,
		area.RightX,
	)

	if name == "" {
		return coordinates
	}

	return fmt.Sprintf("%s (%s)", name, coordinates)
}

func displayFromParams(params []actionParam) fyne.CanvasObject {
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

func actionTypeFromParams(params []actionParam) string {
	for _, p := range params {
		if strings.EqualFold(p.Label, "Type") {
			return strings.TrimSpace(fmt.Sprintf("%v", p.Value))
		}
	}
	return ""
}

func actionPillColor(actionType string) color.NRGBA {
	return ActionPastelColor(actionType)
}
