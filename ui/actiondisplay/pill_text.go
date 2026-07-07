package actiondisplay

import (
	"Sqyre/ui/custom_widgets"

	"fyne.io/fyne/v2/canvas"
	"fyne.io/fyne/v2/theme"
)

// PillTextSize matches the caption size used by display pills.
func PillTextSize() float32 {
	return custom_widgets.PillTextSize()
}

// NewPillText creates foreground text at display-pill size.
func NewPillText(text string) *canvas.Text {
	t := canvas.NewText(text, theme.Color(theme.ColorNameForeground))
	t.TextSize = PillTextSize()
	return t
}

func setPillTextEnabled(t *canvas.Text, enabled bool) {
	if enabled {
		t.Color = theme.Color(theme.ColorNameForeground)
	} else {
		t.Color = theme.Color(theme.ColorNameDisabled)
	}
	t.Refresh()
}
