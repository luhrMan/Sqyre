package actions

import (
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
	return stringifyParams(a.parameters())
}

func (a *Click) Display() fyne.CanvasObject {
	return displayFromParams(a.parameters())
}

func (a *Click) parameters() []actionParam {
	return []actionParam{
		newParam("Type", a.GetType()),
		newParam("Button", LeftOrRight(a.Button)),
		newParam("State", UpOrDown(a.State)),
	}
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
