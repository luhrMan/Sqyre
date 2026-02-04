package actions

import (
	"fmt"

	"Squire/internal/assets"

	"fyne.io/fyne/v2"
)

type Move struct {
	*BaseAction `yaml:",inline" mapstructure:",squash"`
	Point       Point
}

func NewMove(p Point) *Move {
	return &Move{
		BaseAction: newBaseAction("move"),
		Point:      p,
	}
}

func (a *Move) String() string {
	return fmt.Sprintf("%v (%v, %v)", a.Point.Name, a.Point.X, a.Point.Y)
}

func (a *Move) Icon() fyne.Resource {
	return assets.MouseIcon
}
