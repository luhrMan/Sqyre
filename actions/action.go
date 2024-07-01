package actions

import (
	"Dark-And-Darker/utils"
	"log"

	"github.com/go-vgo/robotgo"
)

type Action interface {
	ActionType() string
	PrintParams() string
}

func PerformActions(actions []Action) {
	for _, action := range actions {
		robotgo.Sleep(1)
		switch action := action.(type) {
		case MouseMove:
			//log.Printf("Mouse Move to %s at X: %d, Y: %d", action.Coordinates.SpotName, action.Coordinates.X, action.Coordinates.Y)
			log.Println(action.PrintParams())
			robotgo.Move(action.Coordinates.X, action.Coordinates.Y)
		case Click:
			//log.Printf("Click %d times", action.Amount)
			log.Println(action.PrintParams())
			robotgo.Click()
		case Search:
			// log.Printf("Search %s for %d %s", action.SearchBox.AreaName, action.Amount, action.Item)
			log.Println(action.PrintParams())
			utils.ImageSearch(action.SearchBox, action.Item.Name)
		case OCR:
		default:
			log.Printf("Unsupported action type: %s", action.ActionType())
		}
	}
}
