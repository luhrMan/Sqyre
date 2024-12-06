package structs

import (
	"Dark-And-Darker/internal/utils"
	"fmt"
	"fyne.io/fyne/v2/data/binding"
	"log"

	"github.com/go-vgo/robotgo"
)

type ActionInterface interface {
	Execute(ctx interface{}) error

	GetUID() string
	SetUID(string)

	GetParent() AdvancedActionInterface
	SetParent(AdvancedActionInterface)

	String() string

	UpdateBaseAction(uid string, parent AdvancedActionInterface)
}

func (a *AdvancedAction) GetName() string                      { return a.Name }
func (a *AdvancedAction) SetName(name string)                  { a.Name = name }
func (a *baseAction) GetUID() string                           { return a.UID }
func (a *baseAction) SetUID(uid string)                        { a.UID = uid }
func (a *baseAction) GetParent() AdvancedActionInterface       { return a.Parent }
func (a *baseAction) SetParent(action AdvancedActionInterface) { a.Parent = action }
func (a *baseAction) Execute(ctx interface{}) error            { return nil }
func (a *baseAction) String() string                           { return "This is a baseAction" }

//***************************************************************************************Wait

type WaitAction struct {
	baseAction     //`json:"baseaction"`
	Time       int `json:"waittime"`
}

func NewWaitAction(time int) *WaitAction {
	return &WaitAction{
		baseAction: newBaseAction(),
		Time:       time,
	}
}

func (a *WaitAction) Execute(ctx interface{}) error {
	log.Printf("Waiting for %d milliseconds", a.Time)
	robotgo.MilliSleep(a.Time)
	return nil
}

func (a *WaitAction) String() string {
	return fmt.Sprintf("%s Wait for %d ms", utils.GetEmoji("Wait"), a.Time)
}

func (a *WaitAction) GetBoundTime() binding.ExternalInt {
	return binding.BindInt(&a.Time)
}

// ***************************************************************************************Click

type ClickAction struct {
	baseAction        //`json:"baseaction"`
	Button     string `json:"button"`
}

func NewClickAction(button string) *ClickAction {
	return &ClickAction{
		baseAction: newBaseAction(),
		Button:     button,
	}
}

func (a *ClickAction) Execute(ctx interface{}) error {
	log.Printf("%s click", a.Button)
	robotgo.Click(a.Button)
	return nil
}

func (a *ClickAction) String() string {
	return fmt.Sprintf("%s %s click", utils.GetEmoji("Click"), a.Button)
}

// ***************************************************************************************Move

type MoveAction struct {
	baseAction //`json:"baseaction"`
	X, Y       int
}

func NewMoveAction(x, y int) *MoveAction {
	return &MoveAction{
		baseAction: newBaseAction(),
		X:          x,
		Y:          y,
	}
}

func (a *MoveAction) Execute(ctx interface{}) error {
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

func (a *MoveAction) String() string {
	for _, s := range *GetSpotMap() {
		if (s.X == a.X) && (s.Y == a.Y) {
			return fmt.Sprintf("%s Move mouse to %s", utils.GetEmoji("Move"), s.Name)
		}
	}
	return fmt.Sprintf("%s Move mouse to (%d, %d)", utils.GetEmoji("Move"), a.X, a.Y)
}

// ***************************************************************************************Key

type KeyAction struct {
	baseAction        //`json:"baseaction"`
	Key        string `json:"key"`
	State      string `json:"state"`
}

func NewKeyAction(key, state string) *KeyAction {
	return &KeyAction{
		baseAction: newBaseAction(),
		Key:        key,
		State:      state,
	}
}

func (a *KeyAction) Execute(ctx interface{}) error {
	log.Printf("Key: %s %s", a.Key, a.State)
	switch a.State {
	case "Up":
		err := robotgo.KeyUp(a.Key)
		if err != nil {
			return err
		}
	case "Down":
		err := robotgo.KeyDown(a.Key)
		if err != nil {
			return err
		}
	}
	return nil
}

func (a *KeyAction) String() string {
	return fmt.Sprintf("%s Key: %s %s ", utils.GetEmoji("Key"), a.Key, a.State)
}
