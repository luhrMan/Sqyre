package actions

import (
	"Squire/internal/data"
	"fmt"
	"log"

	"github.com/go-vgo/robotgo"
)

type Move struct {
	baseAction //`json:"baseaction"`
	X, Y       int
}

func NewMove(x, y int) *Move {
	return &Move{
		baseAction: newBaseAction(),
		X:          x,
		Y:          y,
	}
}

func (a *Move) Execute(ctx interface{}) error {
	//if (a.X == -1) && (a.Y == -1) {
	if c, ok := ctx.(robotgo.Point); ok {
		log.Printf("Moving mouse to ctx (%d, %d)", c.X, c.Y)
		robotgo.Move(c.X+data.XOffset+25, c.Y+data.YOffset+25)
	} else {
		log.Printf("Moving mouse to (%d, %d)", a.X, a.Y)
		robotgo.Move(a.X+data.XOffset, a.Y+data.YOffset)
	}
	return nil
}

func (a *Move) String() string {
	for _, s := range *data.GetPointMap() {
		if (s.X == a.X) && (s.Y == a.Y) {
			return fmt.Sprintf("%s Move mouse to %s", data.GetEmoji("Move"), s.Name)
		}
	}
	return fmt.Sprintf("%s Move mouse to (%d, %d)", data.GetEmoji("Move"), a.X, a.Y)
}
