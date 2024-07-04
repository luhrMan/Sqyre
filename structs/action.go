package structs

import (
	//"Dark-And-Darker/main"
	"Dark-And-Darker/utils"
	"fmt"
	"log"

	"github.com/go-vgo/robotgo"
)

type Action interface {
	Execute()
	String() string
}

// ***************************************************************************************Sleep
type WaitAction struct {
	Time int
}

func (a *WaitAction) Execute() {
	log.Printf("Waiting for %d milliseconds", a.Time)
	robotgo.MilliSleep(a.Time)
}

func (a *WaitAction) String() string {
	return fmt.Sprintf("%s Wait for %d ms", utils.GetEmoji("Wait"), a.Time)
}

// ***************************************************************************************Click
type ClickAction struct {
	Button string
}

func (a *ClickAction) Execute() {
	log.Printf("%s Click", a.Button)
	robotgo.Click(a.Button)
}

func (a *ClickAction) String() string {
	return fmt.Sprintf("%s %s Click", utils.GetEmoji("Click"), a.Button)
}

// ***************************************************************************************Move
type MouseMoveAction struct {
	X, Y int
}

func (a *MouseMoveAction) Execute() {
	log.Printf("Moving mouse to (%d, %d)", a.X, a.Y)
	robotgo.Move(a.X, a.Y)
}

func (a *MouseMoveAction) String() string {
	return fmt.Sprintf("%s Move mouse to (%d, %d)", utils.GetEmoji("Move"), a.X, a.Y)
}

// ***************************************************************************************Key
type KeyAction struct {
	Key   string
	State string
}

func (a *KeyAction) Execute() {
	log.Printf("Key: %s %s", a.Key, a.State)
	switch a.State {
	case "Up":
		robotgo.KeyUp(a.Key)
	case "Down":
		robotgo.KeyDown(a.Key)
	}
}

func (a *KeyAction) String() string {
	return fmt.Sprintf("%s Key: %s %s ", utils.GetEmoji("Key"), a.Key, a.State)
}

// ***************************************************************************************ImageSearch
type ImageSearchAction struct {
	X1, Y1, X2, Y2 int
	Target         string
}

func (a *ImageSearchAction) Execute() {
	log.Printf("Image Search | %s in X1:%d Y1:%d X2:%d Y2:%d", a.Target, a.X1, a.Y1, a.X2, a.Y2)

}

func (a *ImageSearchAction) String() string {
	return fmt.Sprintf("Image Search for %s", a.Target)
}

// type Action interface {
// 	ActionType() string
// 	PrintParams() string
// }

// func PerformActions(actions []Action) {
// 	for _, action := range actions {
// 		robotgo.Sleep(1)
// 		switch action := action.(type) {
// 		case MouseMove:
// 			//log.Printf("Mouse Move to %s at X: %d, Y: %d", action.Coordinates.SpotName, action.Coordinates.X, action.Coordinates.Y)
// 			log.Println(action.PrintParams())
// 			robotgo.Move(action.Coordinates.X, action.Coordinates.Y)
// 		case Click:
// 			//log.Printf("Click %d times", action.Amount)
// 			log.Println(action.PrintParams())
// 			robotgo.Click()
// 		case Search:
// 			// log.Printf("Search %s for %d %s", action.SearchBox.AreaName, action.Amount, action.Item)
// 			log.Println(action.PrintParams())
// 			utils.ImageSearch(action.SearchBox, action.Item.Name)
// 		case OCR:
// 		default:
// 			log.Printf("Unsupported action type: %s", action.ActionType())
// 		}
// 	}
// }
