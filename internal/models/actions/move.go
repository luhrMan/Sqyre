package actions

import (
	"fmt"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/theme"
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
	return fmt.Sprintf("%v (%d, %d)", a.Point.Name, a.Point.X, a.Point.Y)
}

func (a *Move) Icon() fyne.Resource {
	return theme.MailForwardIcon()
}
