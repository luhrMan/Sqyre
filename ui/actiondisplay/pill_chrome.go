package actiondisplay

import (
	"image/color"

	"Sqyre/ui/custom_widgets"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/canvas"
	"fyne.io/fyne/v2/container"
	"fyne.io/fyne/v2/theme"
)

// PillLineHeight is the text line height used inside display and editable pills.
func PillLineHeight() float32 {
	return custom_widgets.PillLineHeight()
}

// PillChrome wraps content in the same rounded chip background as display pills.
func PillChrome(content fyne.CanvasObject, actionType string) fyne.CanvasObject {
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
	padded := container.NewBorder(topPad, bottomPad, leftPad, rightPad, content)
	return container.NewStack(bg, padded)
}
