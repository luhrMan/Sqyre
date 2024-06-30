package actions

import (
	"Dark-And-Darker/utils"
	"log"
	"reflect"

	"github.com/go-vgo/robotgo"
)

type Action interface {
	ActionType() string
	PrintParams() string
}

func PerformActions(actions []Action) {
	for a, action := range actions {
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

		case Repeater:
			if !action.Starter {
				continue
			} else if action.Starter {
				end := func() int {
					for b, findEnd := range actions[a:] { // find end of repeater
						if reflect.TypeOf(findEnd) == reflect.TypeOf(action) {
							findEnd := findEnd.(Repeater)
							if !findEnd.Starter {
								return b
							}
						}
					}
					return 0
				}()
				for i := 0; i < action.Amount; i++ {
					PerformActions(actions[a+1 : end+a])
				}
			}
		default:
			log.Printf("Unsupported action type: %s", action.ActionType())
		}
	}
}
