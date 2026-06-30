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

func formatParamValue(value any) string {
	switch v := value.(type) {
	case float32:
		return formatFloatUpTo2Decimals(float64(v))
	case float64:
		return formatFloatUpTo2Decimals(v)
	default:
		return fmt.Sprintf("%v", value)
	}
}

func formatFloatUpTo2Decimals(f float64) string {
	s := fmt.Sprintf("%.2f", f)
	s = strings.TrimRight(s, "0")
	s = strings.TrimRight(s, ".")
	return s
}

func stringifyParams(params []actionParam) string {
	parts := make([]string, 0, len(params))
	for _, p := range params {
		value := strings.TrimSpace(formatParamValue(p.Value))
		if value == "" {
			continue
		}
		parts = append(parts, fmt.Sprintf("%s: %s", p.Label, value))
	}
	return strings.Join(parts, " "+config.DescriptionDelimiter+" ")
}

func displayFromParams(params []actionParam) fyne.CanvasObject {
	line := container.NewHBox()
	actionType := actionTypeFromParams(params)
	for _, p := range params {
		if strings.EqualFold(p.Label, "Type") {
			continue
		}
		value := strings.TrimSpace(formatParamValue(p.Value))
		if value == "" {
			continue
		}
		line.Add(NewDisplayPill(fmt.Sprintf("%s: %s", p.Label, value), actionType))
	}
	return line
}

// NewDisplayPill renders a rounded label chip using the pastel color for actionType.
func NewDisplayPill(text string, actionType string) fyne.CanvasObject {
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
