package actions

import (
	"fmt"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/theme"
)

// Type automates keyboard input by typing a string with a configurable
// delay between each key press.
type Type struct {
	*BaseAction `yaml:",inline" mapstructure:",squash"`
	Text        string
	DelayMs     int
}

func NewType(text string, delayMs int) *Type {
	return &Type{
		BaseAction: newBaseAction("type"),
		Text:       text,
		DelayMs:    delayMs,
	}
}

func (a *Type) String() string {
	if a.DelayMs > 0 {
		return fmt.Sprintf("%q (%d ms)", a.Text, a.DelayMs)
	}
	return fmt.Sprintf("%q", a.Text)
}

func (a *Type) Icon() fyne.Resource {
	return theme.DocumentIcon()
}

