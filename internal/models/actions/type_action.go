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
	return stringifyParams(a.parameters())
}

func (a *Type) Display() fyne.CanvasObject {
	return displayFromParams(a.parameters())
}

func (a *Type) parameters() []actionParam {
	params := []actionParam{
		newParam("Type", a.GetType()),
		newParam("Text", fmt.Sprintf("%q", a.Text)),
	}
	if a.DelayMs > 0 {
		params = append(params, newParam("Delay", fmt.Sprintf("%d ms", a.DelayMs)))
	}
	return params
}

func (a *Type) Icon() fyne.Resource {
	return theme.DocumentIcon()
}

