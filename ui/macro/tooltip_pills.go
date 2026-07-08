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

func newColorSwatchRect() *canvas.Rectangle {
	swatch := canvas.NewRectangle(color.Transparent)
	swatch.StrokeColor = theme.Color(theme.ColorNameSeparator)
	swatch.StrokeWidth = 1
	swatch.CornerRadius = 2
	size := actiondisplay.PillLineHeight()
	swatch.SetMinSize(fyne.NewSize(size, size))
	return swatch
}

func wrapSwatch(swatch *canvas.Rectangle, actionType string) fyne.CanvasObject {
	size := actiondisplay.PillLineHeight()
	return actiondisplay.PillChrome(container.NewGridWrap(fyne.NewSize(size, size), swatch), actionType)
}

// colorSwatchPill renders a small filled square showing the given hex color,
// wrapped in the standard pill chrome. Returns nil when hex is not a valid
// color (e.g. a variable reference), so callers can skip it.
func colorSwatchPill(hex, actionType string) fyne.CanvasObject {
	c, ok := parseHexColor(hex)
	if !ok {
		return nil
	}
	swatch := newColorSwatchRect()
	swatch.FillColor = c
	return wrapSwatch(swatch, actionType)
}

// editableColorSwatchPill returns a swatch pill plus an update func that recolors
// it from a hex string live while editing. The pill hides itself when the value
// is not a valid color (e.g. a variable reference).
func editableColorSwatchPill(hex, actionType string) (fyne.CanvasObject, func(hex string)) {
	swatch := newColorSwatchRect()
	pill := wrapSwatch(swatch, actionType)
	update := func(h string) {
		c, ok := parseHexColor(h)
		if !ok {
			pill.Hide()
			return
		}
		pill.Show()
		if swatch.FillColor != c {
			swatch.FillColor = c
			swatch.Refresh()
		}
	}
	update(hex)
	return pill, update
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
