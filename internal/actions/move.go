package actions

import (
        "Dark-And-Darker/internal/structs"
        "Dark-And-Darker/internal/utils"
        "fmt"
        "github.com/go-vgo/robotgo"
        "log"
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
                robotgo.Move(c.X+utils.XOffset+25, c.Y+utils.YOffset+25)
        } else {
                log.Printf("Moving mouse to (%d, %d)", a.X, a.Y)
                robotgo.Move(a.X+utils.XOffset, a.Y+utils.YOffset)
        }
        return nil
}

func (a *Move) String() string {
        for _, s := range *structs.GetSpotMap() {
                if (s.X == a.X) && (s.Y == a.Y) {
                        return fmt.Sprintf("%s Move mouse to %s", utils.GetEmoji("Move"), s.Name)
                }
        }
        return fmt.Sprintf("%s Move mouse to (%d, %d)", utils.GetEmoji("Move"), a.X, a.Y)
}
