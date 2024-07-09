package structs

import (
	"Dark-And-Darker/utils"
	"fmt"
	"log"
	"sync"

	"github.com/vcaesar/bitmap"

	"github.com/go-vgo/robotgo"
)

type ActionType int

const (
	WaitType ActionType = iota
	ClickType
	MouseMoveType
	KeyType
	ImageSearchType
	OcrType
)

type Action interface {
	Execute(context *Context) error
	GetType() ActionType
	String() string
}

type Context struct {
	Variables map[string]interface{}
}

//***************************************************************************************Wait

type WaitAction struct {
	Time int
}

func (a *WaitAction) Execute(context *Context) error {
	log.Printf("Waiting for %d milliseconds", a.Time)
	robotgo.MilliSleep(a.Time)
	return nil
}
func (a *WaitAction) GetType() ActionType {
	return WaitType
}

func (a *WaitAction) String() string {
	return fmt.Sprintf("%s Wait for %d ms", utils.GetEmoji("Wait"), a.Time)
}

// ***************************************************************************************Click

type ClickAction struct {
	Button string
}

func (a *ClickAction) Execute(context *Context) error {
	log.Printf("%s Click", a.Button)
	robotgo.Click(a.Button)
	log.Println(context)
	return nil
}

func (a *ClickAction) GetType() ActionType {
	return ClickType
}

func (a *ClickAction) String() string {
	return fmt.Sprintf("%s %s Click", utils.GetEmoji("Click"), a.Button)
}

// ***************************************************************************************Move

type MouseMoveAction struct {
	X, Y int
}

func (a *MouseMoveAction) Execute(context *Context) error {
	log.Printf("Moving mouse to (%d, %d)", a.X, a.Y)
	robotgo.Move(a.X+utils.XOffset, a.Y+utils.YOffset)
	return nil
}

func (a *MouseMoveAction) GetType() ActionType {
	return MouseMoveType
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
	Key   string
	State string
}

func (a *KeyAction) Execute(context *Context) error {
	log.Printf("Key: %s %s", a.Key, a.State)
	switch a.State {
	case "Up":
		robotgo.KeyUp(a.Key)
	case "Down":
		robotgo.KeyDown(a.Key)
	}
	return nil

}
func (a *KeyAction) GetType() ActionType {
	return KeyType
}

func (a *KeyAction) String() string {
	return fmt.Sprintf("%s Key: %s %s ", utils.GetEmoji("Key"), a.Key, a.State)
}

// ***************************************************************************************ImageSearch

type ImageSearchAction struct {
	SearchBox SearchBox
	Targets   []string
}

func (a *ImageSearchAction) Execute(context *Context) error {
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

	context.Variables["ImageSearchResults"] = results

	// for _, r := range results {
	// 	for _, i := range r {
	// 		robotgo.Move(i.X+utils.XOffset+5, i.Y+utils.YOffset+5)
	// 		robotgo.MilliSleep(500)
	// 	}
	// }
	return nil
}

func (a *ImageSearchAction) GetType() ActionType {
	return ImageSearchType
}

func (a *ImageSearchAction) String() string {
	return fmt.Sprintf("%s Image Search for `%s` in `%s`", utils.GetEmoji("Image Search"), a.Targets, a.SearchBox.Name)
}

// ***************************************************************************************OCR

type OcrAction struct {
	SearchBox SearchBox
	Target    string
}

func (a *OcrAction) Execute(context *Context) error {
	log.Printf("OCR search | %s in X1:%d Y1:%d X2:%d Y2:%d", a.Target, a.SearchBox.SearchArea.LeftX, a.SearchBox.SearchArea.TopY, a.SearchBox.SearchArea.RightX, a.SearchBox.SearchArea.BottomY)
	return nil
}

func (a *OcrAction) GetType() ActionType {
	return OcrType
}

func (a *OcrAction) String() string {
	return fmt.Sprintf("%s OCR search for `%s` in `%s`", utils.GetEmoji("OCR"), a.Target, a.SearchBox.Name)
}
