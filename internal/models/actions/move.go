package actions

import (
	"Sqyre/internal/assets"

	"fyne.io/fyne/v2"
)

type Move struct {
	*BaseAction `yaml:",inline" mapstructure:",squash"`
	Point       Point
	Smooth      bool `yaml:"smooth" mapstructure:"smooth"`
}

func NewMove(p Point, smooth bool) *Move {
	return &Move{
		BaseAction: newBaseAction("move"),
		Point:      p,
		Smooth:     smooth,
	}
}

func (a *Move) String() string {
	return stringifyParams(a.parameters())
}

func (a *Move) Display() fyne.CanvasObject {
	return displayFromParams(a.parameters())
}

func (a *Move) parameters() []actionParam {
	params := []actionParam{
		newParam("Type", a.GetType()),
		newParam("Point", a.Point.Name),
		newParam("X", a.Point.X),
		newParam("Y", a.Point.Y),
	}
	if a.Smooth {
		params = append(params, newParam("Smooth", true))
	}
	return params
}

func (a *Move) Icon() fyne.Resource {
	return assets.MouseIcon
}
