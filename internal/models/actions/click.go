package actions

import (
	"fmt"

	"Sqyre/internal/assets"

	"fyne.io/fyne/v2"
)

type Click struct {
	*BaseAction `yaml:",inline" mapstructure:",squash"`
	Button      bool `yaml:"button" mapstructure:"button"`
	State       bool `yaml:"state" mapstructure:"state"`
}

func NewClick(button bool, state bool) *Click {
	return &Click{
		BaseAction: newBaseAction("click"),
		Button:     button,
		State:      state,
	}
}

func (a *Click) String() string {
	return fmt.Sprintf("%s click %s", LeftOrRight(a.Button), UpOrDown(a.State))
}

func LeftOrRight(b bool) string {
	if b {
		return "right"
	}
	return "left"
}

func (a *Click) Icon() fyne.Resource {
	if a.State {
		return assets.MouseClickFilledIcon
	}
	return assets.MouseClickIcon
}
