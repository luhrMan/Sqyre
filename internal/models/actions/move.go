package actions

import (
	"fmt"

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
	if a.Smooth {
		return fmt.Sprintf("%v (%v, %v) smooth", a.Point.Name, a.Point.X, a.Point.Y)
	}
	return fmt.Sprintf("%v (%v, %v)", a.Point.Name, a.Point.X, a.Point.Y)
}

func (a *Move) Icon() fyne.Resource {
	return assets.MouseIcon
}
