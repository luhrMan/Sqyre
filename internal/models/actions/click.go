package actions

import (
	"fmt"

	"Squire/internal/assets"

	"fyne.io/fyne/v2"
)

type Click struct {
	*BaseAction `yaml:",inline" mapstructure:",squash"`
	Button      bool `yaml:"button" mapstructure:"button"`
	Hold        bool `yaml:"hold" mapstructure:"hold"`
}

func NewClick(button bool, hold bool) *Click {
	return &Click{
		BaseAction: newBaseAction("click"),
		Button:     button,
		Hold:       hold,
	}
}

func (a *Click) String() string {
	if a.Hold {
		return fmt.Sprintf("%s click (hold)", LeftOrRight(a.Button))
	}
	return fmt.Sprintf("%s click", LeftOrRight(a.Button))
}

func LeftOrRight(b bool) string {
	if b {
		return "right"
	}
	return "left"
}

func (a *Click) Icon() fyne.Resource {
	if a.Hold {
		return assets.MouseClickFilledIcon
	}
	return assets.MouseClickIcon
}
