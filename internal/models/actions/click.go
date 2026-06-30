package actions

import (
	"Sqyre/internal/assets"

	"fyne.io/fyne/v2"
)

const (
	ClickButtonLeft   = "left"
	ClickButtonRight  = "right"
	ClickButtonCenter = "center"
	ClickButtonScroll = "scroll"
)

var ClickButtons = []string{ClickButtonLeft, ClickButtonRight, ClickButtonCenter, ClickButtonScroll}

type Click struct {
	*BaseAction `yaml:",inline" mapstructure:",squash"`
	Button      string `yaml:"button" mapstructure:"button"`
	State       bool   `yaml:"state" mapstructure:"state"`
}

func NewClick(button string, state bool) *Click {
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
		newParam("Button", ClickButtonLabel(a.Button)),
		newParam("State", UpOrDown(a.State)),
	}
}

func ClickButtonLabel(button string) string {
	switch button {
	case ClickButtonRight:
		return "right"
	case ClickButtonCenter:
		return "center"
	case ClickButtonScroll:
		return "scroll"
	default:
		return "left"
	}
}

func (a *Click) Icon() fyne.Resource {
	if a.State {
		return assets.MouseClickFilledIcon
	}
	return assets.MouseClickIcon
}
