package structs

import (
	"Dark-And-Darker/utils"
	"fmt"
	"log"

	"github.com/go-vgo/robotgo"
)

type ActionInterface interface {
	Execute(context interface{}) error

	GetUID() string
	SetUID(string)

	GetParent() AdvancedActionInterface
	SetParent(AdvancedActionInterface)

	String() string

	updateBaseAction(uid string, parent AdvancedActionInterface)
}

func (a *AdvancedAction) GetName() string                      { return a.Name }
func (a *AdvancedAction) SetName(name string)                  { a.Name = name }
func (a *BaseAction) GetUID() string                           { return a.UID }
func (a *BaseAction) SetUID(uid string)                        { a.UID = uid }
func (a *BaseAction) GetParent() AdvancedActionInterface       { return a.Parent }
func (a *BaseAction) SetParent(action AdvancedActionInterface) { a.Parent = action }
func (a *BaseAction) Execute(context interface{}) error        { return nil }
func (a *BaseAction) String() string                           { return "This is a BaseAction" }

//***************************************************************************************Wait

type WaitAction struct {
	BaseAction
	Time int `json:"waittime"`
}

func (a *WaitAction) Execute(context interface{}) error {
	log.Printf("Waiting for %d milliseconds", a.Time)
	robotgo.MilliSleep(a.Time)
	return nil
}

func (a *WaitAction) String() string {
	return fmt.Sprintf("%s Wait for %d ms", utils.GetEmoji("Wait"), a.Time)
}

// ***************************************************************************************Click

type ClickAction struct {
	BaseAction
	Button string `json:"button"`
}

func (a *ClickAction) Execute(context interface{}) error {
	log.Printf("%s click", a.Button)
	robotgo.Click(a.Button)
	return nil
}

func (a *ClickAction) String() string {
	return fmt.Sprintf("%s %s click", utils.GetEmoji("Click"), a.Button)
}

// ***************************************************************************************Move

type MouseMoveAction struct {
	BaseAction
	X, Y int
}

func (a *MouseMoveAction) Execute(context interface{}) error {
	//if (a.X == -1) && (a.Y == -1) {
	if c, ok := context.(robotgo.Point); ok {
		log.Printf("Moving mouse to context (%d, %d)", c.X, c.Y)
		robotgo.Move(c.X+40+utils.XOffset, c.Y+40+utils.YOffset)
	} else {
		log.Printf("Moving mouse to (%d, %d)", a.X, a.Y)
		robotgo.Move(a.X+utils.XOffset, a.Y+utils.YOffset)
	}
	return nil
}

func (a *MouseMoveAction) String() string {
	for _, s := range *GetSpotMap() {
		if (s.X == a.X) && (s.Y == a.Y) {
			return fmt.Sprintf("%s Move mouse to %s", utils.GetEmoji("Move"), s.Name)
		}
	}
	return fmt.Sprintf("%s Move mouse to (%d, %d)", utils.GetEmoji("Move"), a.X, a.Y)
}

// ***************************************************************************************Key

type KeyAction struct {
	BaseAction
	Key   string `json:"key"`
	State string `json:"state"`
}

func (a *KeyAction) Execute(context interface{}) error {
	log.Printf("Key: %s %s", a.Key, a.State)
	switch a.State {
	case "Up":
		robotgo.KeyUp(a.Key)
	case "Down":
		robotgo.KeyDown(a.Key)
	}
	return nil
}

func (a *KeyAction) String() string {
	return fmt.Sprintf("%s Key: %s %s ", utils.GetEmoji("Key"), a.Key, a.State)
}
