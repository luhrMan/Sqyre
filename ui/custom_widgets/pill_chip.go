package custom_widgets

import (
	"image/color"

	"Sqyre/internal/models/actions"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/canvas"
	"fyne.io/fyne/v2/container"
	"fyne.io/fyne/v2/theme"
)

func newPillText(text string) *canvas.Text {
	t := canvas.NewText(text, theme.Color(theme.ColorNameForeground))
	t.TextSize = PillTextSize()
	return t
}

func pillChrome(content fyne.CanvasObject, actionType string) fyne.CanvasObject {
	fill := actionPastelColor(actionType)
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
	padded := container.NewBorder(topPad, bottomPad, leftPad, rightPad, content)
	return container.NewStack(bg, padded)
}

func actionPastelColor(actionType string) color.NRGBA {
	dark := false
	if app := fyne.CurrentApp(); app != nil {
		dark = app.Settings().ThemeVariant() == theme.VariantDark
	}
	return actions.ActionPastelColor(actionType, dark)
}

func newDisplayPillChip(text string, actionType string) fyne.CanvasObject {
	return pillChrome(newPillText(text), actionType)
}

// NewNestedVariableRefPill renders a compact variable chip for use inside another pill.
func NewNestedVariableRefPill(name string, unknown bool) fyne.CanvasObject {
	actionType := "setvariable"
	if unknown {
		actionType = "warning"
	}
	return nestedVarRefChip(name, actionType)
}

func nestedVarRefColor(actionType string) color.NRGBA {
	if actionType != "setvariable" {
		return actionPastelColor(actionType)
	}
	dark := false
	if app := fyne.CurrentApp(); app != nil {
		dark = app.Settings().ThemeVariant() == theme.VariantDark
	}
	return actions.DefaultNestedVarRefColor(dark)
}

func nestedVarRefChip(text, actionType string) fyne.CanvasObject {
	fill := nestedVarRefColor(actionType)
	bg := canvas.NewRectangle(fill)
	bg.CornerRadius = 4
	leftPad := canvas.NewRectangle(color.Transparent)
	leftPad.SetMinSize(fyne.NewSize(3, 0))
	rightPad := canvas.NewRectangle(color.Transparent)
	rightPad.SetMinSize(fyne.NewSize(3, 0))
	row := container.NewHBox(leftPad, newPillText(text), rightPad)
	rowH := PillLineHeight()
	vGuide := canvas.NewRectangle(color.Transparent)
	vGuide.SetMinSize(fyne.NewSize(0, rowH))
	return container.NewStack(bg, container.NewStack(vGuide, row))
}
