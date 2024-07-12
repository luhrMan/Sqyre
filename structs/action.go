package structs

import (
	"Dark-And-Darker/utils"
	"fmt"
	"log"
	"sync"

	"fyne.io/fyne/v2/widget"
	"github.com/go-vgo/robotgo"

	//"github.com/otiai10/gosseract"
	"github.com/vcaesar/bitmap"
)

type ActionInterface interface {
	Execute(context interface{}) error

	GetBaseAction() *BaseAction
	SetBaseAction(*BaseAction)

	GetName() string
	SetName(string)

	GetUID() string
	SetUID(string)

	GetParent() ActionWithSubActionsInterface
	SetParent(ActionWithSubActionsInterface)

	String() string

	updateBaseAction(uid string, name string, parent ActionWithSubActionsInterface)
}

type ActionWithSubActionsInterface interface {
	ActionInterface

	GetSubActions() []ActionInterface

	AddSubAction(ActionInterface, string)
	RemoveSubAction(ActionInterface, *widget.Tree)

	RenameActions(*widget.Tree)
}

type ActionWithSubActions struct {
	BaseAction
	SubActions []ActionInterface
}

func (a *ActionWithSubActions) GetSubActions() []ActionInterface {
	return a.SubActions
}

func (a *ActionWithSubActions) AddSubAction(action ActionInterface, name string) {
	actionNum := len(a.GetSubActions()) + 1
	uid := fmt.Sprintf("%s.%d", a.GetUID(), actionNum)
	action.updateBaseAction(uid, name, a)

	a.SubActions = append(a.SubActions, action)
	log.Printf("Added new action: %s", name)
}

func (a *ActionWithSubActions) RemoveSubAction(action ActionInterface, tree *widget.Tree) {
	for i, c := range a.SubActions {
		if c == action {
			a.SubActions = append(a.SubActions[:i], a.SubActions[i+1:]...)
			log.Printf("Removing %s", action.GetUID())
			//child.SetParent(nil)
			a.RenameActions(tree)
			return
		}
	}
}

func (a *ActionWithSubActions) RenameActions(tree *widget.Tree) {
	for i, child := range a.SubActions {
		open := tree.IsBranchOpen(child.GetUID())
		child.SetUID(fmt.Sprintf("%s.%d", a.UID, i+1))
		if open {
			tree.OpenBranch(child.GetUID())
		}
		if n, ok := child.(ActionWithSubActionsInterface); ok {
			n.RenameActions(tree)
		}
	}
}

type BaseAction struct {
	Name   string
	UID    string
	Parent ActionWithSubActionsInterface
}

func (a *BaseAction) updateBaseAction(uid string, name string, parent ActionWithSubActionsInterface) {
	a.SetUID(uid)
	a.SetParent(parent)
	a.SetName(name)
}

func (a *BaseAction) GetName() string                                { return a.Name }
func (a *BaseAction) SetName(name string)                            { a.Name = name }
func (a *BaseAction) GetUID() string                                 { return a.UID }
func (a *BaseAction) SetUID(uid string)                              { a.UID = uid }
func (a *BaseAction) GetParent() ActionWithSubActionsInterface       { return a.Parent }
func (a *BaseAction) SetParent(action ActionWithSubActionsInterface) { a.Parent = action }
func (a *BaseAction) Execute(context interface{}) error              { return nil }
func (a *BaseAction) String() string                                 { return "This is a BaseAction" }
func (a *BaseAction) GetBaseAction() *BaseAction                     { return a }
func (a *BaseAction) SetBaseAction(b *BaseAction)                    { a = b }

func (a *ActionWithSubActions) Execute(context interface{}) error { return nil }
func (a *ActionWithSubActions) String() string                    { return "This is a Action with SubActions" }

//***************************************************************************************Wait

type WaitAction struct {
	BaseAction
	Time int
}

func (a *WaitAction) Execute(context interface{}) error {
	log.Printf("Waiting for %d milliseconds", a.Time)
	robotgo.MilliSleep(a.Time)
	return nil
}

func (a *WaitAction) GetName() string {
	return a.Name
}

func (a *WaitAction) String() string {
	return fmt.Sprintf("%s Wait for %d ms", utils.GetEmoji("Wait"), a.Time)
}

// ***************************************************************************************Click

type ClickAction struct {
	BaseAction
	Button string
}

func (a *ClickAction) Execute(context interface{}) error {
	log.Printf("%s Click", a.Button)
	robotgo.Click(a.Button)
	return nil
}

func (a *ClickAction) String() string {
	return fmt.Sprintf("%s %s Click", utils.GetEmoji("Click"), a.Button)
}

// ***************************************************************************************Move

type MouseMoveAction struct {
	BaseAction
	X, Y int
}

func (a *MouseMoveAction) Execute(context interface{}) error {
	if c, ok := context.(robotgo.Point); ok {
		log.Printf("Moving mouse to (%d, %d)", c.X, c.Y)
		robotgo.Move(c.X+40+utils.XOffset, c.Y+40+utils.YOffset)
	} else {
		log.Printf("Moving mouse to (%d, %d)", a.X, a.Y)
		robotgo.Move(a.X+utils.XOffset, a.Y+utils.YOffset)
	}
	return nil
}

func (a *MouseMoveAction) String() string {
	for _, s := range *GetSpotMap() {
		if (s.Coordinates.X == a.X) && (s.Coordinates.Y == a.Y) {
			return fmt.Sprintf("%s Move mouse to %s", utils.GetEmoji("Move"), s.Name)
		}
	}
	return fmt.Sprintf("%s Move mouse to (%d, %d)", utils.GetEmoji("Move"), a.X, a.Y)
}

// ***************************************************************************************Key

type KeyAction struct {
	BaseAction
	Key   string
	State string
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

// ***************************************************************************************ImageSearch

type ImageSearchAction struct {
	ActionWithSubActions
	SearchBox SearchBox
	Targets   []string
}

func (a *ImageSearchAction) Execute(context interface{}) error {
	log.Printf("Image Search | %v in X1:%d Y1:%d X2:%d Y2:%d", a.Targets, a.SearchBox.SearchArea.LeftX, a.SearchBox.SearchArea.TopY, a.SearchBox.SearchArea.RightX, a.SearchBox.SearchArea.BottomY)

	// Capture the screen once before processing targets
	capture := robotgo.CaptureScreen(a.SearchBox.SearchArea.LeftX, a.SearchBox.SearchArea.TopY, a.SearchBox.SearchArea.RightX, a.SearchBox.SearchArea.BottomY)
	defer robotgo.FreeBitmap(capture)

	err := robotgo.SaveJpeg(robotgo.ToImage(capture), "./images/wholeScreen.jpeg")
	if err != nil {
		return err
	}

	var wg sync.WaitGroup
	results := make(map[string][]robotgo.Point)
	resultsMutex := &sync.Mutex{}

	for _, target := range a.Targets {
		wg.Add(1)
		go func(target string) {
			defer wg.Done()

			ip := "./images/icons/" + target + ".png"
			predefinedImage, err := robotgo.OpenImg(ip)
			if err != nil {
				log.Printf("robotgo.OpenImg failed for %s: %v\n", target, err)
				return
			}
			predefinedBitmap := robotgo.ByteToCBitmap(predefinedImage)
			targetResults := bitmap.FindAll(predefinedBitmap, capture, 0.1)

			resultsMutex.Lock()
			results[target] = targetResults
			resultsMutex.Unlock()

			log.Printf("Results for %s: %v\n", target, targetResults)

		}(target)
	}

	wg.Wait()

	for _, pointArr := range results {
		for _, point := range pointArr {
			for _, d := range a.SubActions {
				d.Execute(point)
			}
		}
	}
	return nil
}
func (a *ImageSearchAction) String() string {
	return fmt.Sprintf("%s Image Search for `%s` in `%s`", utils.GetEmoji("Image Search"), a.Targets, a.SearchBox.Name)
}

// ***************************************************************************************OCR

type OcrAction struct {
	ActionWithSubActions
	SearchBox SearchBox
	Target    string
}

func (a *OcrAction) Execute(context interface{}) error {
	log.Printf("OCR search | %s in X1:%d Y1:%d X2:%d Y2:%d", a.Target, a.SearchBox.SearchArea.LeftX, a.SearchBox.SearchArea.TopY, a.SearchBox.SearchArea.RightX, a.SearchBox.SearchArea.BottomY)
	// client := gosseract.NewClient()
	// defer client.Close()
	// //img := robotgo.ToByteImg(robotgo.CaptureImg(sb[0], sb[1], sb[2], sb[3]))
	// //capture := robotgo.CaptureImg(sb[0], sb[1], sb[2], sb[3])
	// capture := robotgo.CaptureImg(a.SearchBox.SearchArea.LeftX, a.SearchBox.SearchArea.TopY, a.SearchBox.SearchArea.RightX, a.SearchBox.SearchArea.BottomY)
	// robotgo.SaveJpeg(capture, "./images/test1.jpeg")
	// client.SetImage("./images/test1.jpeg")
	// text, _ := client.Text()
	// log.Println(text)
	return nil
}

func (a *OcrAction) String() string {
	return fmt.Sprintf("%s OCR search for `%s` in `%s`", utils.GetEmoji("OCR"), a.Target, a.SearchBox.Name)
}

//******************************************************************************************Loop

type LoopAction struct {
	ActionWithSubActions
	Count int
}

func (a *LoopAction) Execute(context interface{}) error {
	for i := 0; i < a.Count; i++ {
		fmt.Printf("Loop iteration %d\n", i+1)
		for _, action := range a.GetSubActions() {
			if err := action.Execute(context); err != nil {
				return err
			}
		}
	}
	return nil
}

func (a *LoopAction) String() string {
	return fmt.Sprintf("Loop Action | %s%d", utils.GetEmoji("Container"), a.Count)
}
