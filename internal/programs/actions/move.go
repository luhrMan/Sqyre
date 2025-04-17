package actions

import (
	"Squire/internal/config"
	"Squire/internal/programs/coordinates"
	"fmt"
	"log"

	"github.com/go-vgo/robotgo"
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

func (a *Move) Execute(ctx any) error {
	//if (a.X == -1) && (a.Y == -1) {
	// if c, ok := ctx.(robotgo.Point); ok {
	// 	log.Printf("Moving mouse to ctx (%d, %d)", c.X, c.Y)
	// 	robotgo.Move(c.X+config.XOffset+25, c.Y+config.YOffset+25)
	// } else {
	log.Printf("Moving mouse to %v", a.Point)
	robotgo.Move(a.Point.X+config.XOffset, a.Point.Y+config.YOffset)
	// }
	return nil
}

func (a *Move) String() string {
	// for _, s := range config.JsonPointMap() {
	// 	if (s.X == a.X) && (s.Y == a.Y) {
	// 		return fmt.Sprintf("%s Move mouse to %s", config.GetEmoji("Move"), s.Name)
	// 	}
	// }
	return fmt.Sprintf("%s Move mouse to %v (%d, %d)", config.GetEmoji("Move"), a.Point.Name, a.Point.X, a.Point.Y)
}
