package macro

import (
	"image/color"
	"strconv"
	"strings"

	"Sqyre/ui/actiondisplay"
	"Sqyre/ui/custom_widgets"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/canvas"
	"fyne.io/fyne/v2/container"
	"fyne.io/fyne/v2/theme"
)

func macroKnownVariables() map[string]bool {
	return custom_widgets.KnownVariableSet(macroVariableDefs())
}

// parseHexColor converts a 6- or 8-char hex string (optional leading '#') to an
// NRGBA color. Returns ok=false for variable references or malformed values.
func parseHexColor(hex string) (color.NRGBA, bool) {
	hex = strings.TrimPrefix(strings.ToLower(strings.TrimSpace(hex)), "#")
	if len(hex) == 8 {
		hex = hex[2:]
	}
	if len(hex) != 6 {
		return color.NRGBA{}, false
	}
	r, err1 := strconv.ParseUint(hex[0:2], 16, 8)
	g, err2 := strconv.ParseUint(hex[2:4], 16, 8)
	b, err3 := strconv.ParseUint(hex[4:6], 16, 8)
	if err1 != nil || err2 != nil || err3 != nil {
		return color.NRGBA{}, false
	}
	return color.NRGBA{R: uint8(r), G: uint8(g), B: uint8(b), A: 0xff}, true
}

// colorSwatchPill renders a small filled square showing the given hex color,
// wrapped in the standard pill chrome. Returns nil when hex is not a valid
// color (e.g. a variable reference), so callers can skip it.
func colorSwatchPill(hex, actionType string) fyne.CanvasObject {
	c, ok := parseHexColor(hex)
	if !ok {
		return nil
	}
	swatch := canvas.NewRectangle(c)
	swatch.StrokeColor = theme.Color(theme.ColorNameSeparator)
	swatch.StrokeWidth = 1
	swatch.CornerRadius = 2
	size := actiondisplay.PillLineHeight()
	swatch.SetMinSize(fyne.NewSize(size, size))
	return actiondisplay.PillChrome(container.NewGridWrap(fyne.NewSize(size, size), swatch), actionType)
}

func addDisplayPill(row *pillRow, label, value, actionType string) {
	row.add(actiondisplay.NewDisplayLabeledPill(label, value, actionType, macroKnownVariables()))
}

func addDisplayVariablePill(row *pillRow, label, varName, actionType string) {
	row.add(actiondisplay.NewDisplayVariablePill(label, varName, actionType, macroKnownVariables()))
}

func addInlineDisplayPill(row *pillRow, label, value, actionType string) {
	row.add(actiondisplay.NewDisplayLabeledPill(label, strings.TrimSpace(value), actionType, macroKnownVariables()))
}
