package actions

import (
	"Squire/internal/config"
	"Squire/internal/models/coordinates"
	"fmt"
)

type Move struct {
	*BaseAction `yaml:",inline" mapstructure:",squash"`
	Point       coordinates.Point
	// X, Y        int
}

func NewMove(p coordinates.Point) *Move {
	return &Move{
		BaseAction: newBaseAction("move"),
		Point:      p,
		// X:          x,
		// Y:          y,
	}
}

func (a *Move) String() string {
	// for _, s := range config.JsonPointMap() {
	// 	if (s.X == a.X) && (s.Y == a.Y) {
	// 		return fmt.Sprintf("%s Move mouse to %s", config.GetEmoji("Move"), s.Name)
	// 	}
	// }
	return fmt.Sprintf("%s Move mouse to %v (%d, %d)", config.GetEmoji("Move"), a.Point.Name, a.Point.X, a.Point.Y)
}
