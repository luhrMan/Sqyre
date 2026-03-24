package actions

import (
	"fmt"

	"Sqyre/internal/assets"
	"Sqyre/internal/config"

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
	s := fmt.Sprintf("Point: %v %s Coordinates: (%v, %v)", a.Point.Name, config.DescriptionDelimiter, a.Point.X, a.Point.Y)
	if a.Smooth {
		return s + config.DescriptionDelimiter + "Smooth"
	}
	return s
}

func (a *Move) Icon() fyne.Resource {
	return assets.MouseIcon
}
