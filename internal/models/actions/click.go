package actions

import (
	"fmt"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/theme"
)

type Click struct {
	*BaseAction `yaml:",inline" mapstructure:",squash"`
	Button      bool
}

func NewClick(button bool) *Click {
	return &Click{
		BaseAction: newBaseAction("click"),
		Button:     button,
	}
}

func (a *Click) String() string {
	return fmt.Sprintf("%s click", LeftOrRight(a.Button))
}

func LeftOrRight(b bool) string {
	if b {
		return "right"
	}
	return "left"
}

func (a *Click) Icon() fyne.Resource {
	return theme.MenuDropDownIcon()
}
