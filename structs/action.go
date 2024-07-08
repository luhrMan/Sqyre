package structs

import (
	"Dark-And-Darker/utils"
	"fmt"
	"log"

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
	Target    string
}

func (a *ImageSearchAction) Execute(context *Context) error {
	log.Printf("Image Search | %s in X1:%d Y1:%d X2:%d Y2:%d", a.Target, a.SearchBox.SearchArea.LeftX, a.SearchBox.SearchArea.TopY, a.SearchBox.SearchArea.RightX, a.SearchBox.SearchArea.BottomY)

	ip := "./images/icons/" + a.Target + ".png"
	capture := robotgo.CaptureScreen(a.SearchBox.SearchArea.LeftX, a.SearchBox.SearchArea.TopY, a.SearchBox.SearchArea.RightX, a.SearchBox.SearchArea.BottomY)
	defer robotgo.FreeBitmap(capture)
	err := robotgo.SaveJpeg(robotgo.ToImage(capture), "./images/wholeScreen.jpeg")
	if err != nil {
		return nil
	}

	predefinedImage, err := robotgo.OpenImg(ip)
	if err != nil {
		log.Printf("robotgo.OpenImg failed:%d\n", err)
		return err
	}
	predefinedBitmap := robotgo.ByteToCBitmap(predefinedImage)
	//defer robotgo.FreeBitmap(predefinedBitmap)

	context.Variables["ImageSearchResults"] = bitmap.FindAll(predefinedBitmap, capture, 0.2) // Example results
	//context.Variables["ImageSearchResults"] = []fyne.Position{{X: 100, Y: 100}, {X: 200, Y: 200}} // Example results
	return nil
}

func (a *ImageSearchAction) GetType() ActionType {
	return ImageSearchType
}

func (a *ImageSearchAction) String() string {
	return fmt.Sprintf("%s Image Search for %s", utils.GetEmoji("Image Search"), a.Target)
}

// ImageSearch searchBox[x, y, w, h], imagePath "./images/test.png"
func ImageSearch(sbc SearchBox, itemName string) []robotgo.Point {
	ip := "./images/icons/" + itemName + ".png"
	capture := robotgo.CaptureScreen(sbc.SearchArea.LeftX, sbc.SearchArea.TopY, sbc.SearchArea.RightX, sbc.SearchArea.BottomY)
	defer robotgo.FreeBitmap(capture)
	err := robotgo.SaveJpeg(robotgo.ToImage(capture), "./images/wholeScreen.jpeg")
	if err != nil {
		return nil
	}

	predefinedImage, err := robotgo.OpenImg(ip)
	if err != nil {
		log.Printf("robotgo.OpenImg failed:%d\n", err)
		return []robotgo.Point{}
	}
	predefinedBitmap := robotgo.ByteToCBitmap(predefinedImage)
	//defer robotgo.FreeBitmap(predefinedBitmap)
	return bitmap.FindAll(predefinedBitmap, capture, 0.2)
}

// ***************************************************************************************OCR

type OcrAction struct {
	X1, Y1, X2, Y2 int
	Target         string
}

func (a *OcrAction) Execute(context *Context) error {
	log.Printf("OCR search | %s in X1:%d Y1:%d X2:%d Y2:%d", a.Target, a.X1, a.Y1, a.X2, a.Y2)
	return nil
}

func (a *OcrAction) GetType() ActionType {
	return OcrType
}

func (a *OcrAction) String() string {
	return fmt.Sprintf("%s OCR search for %s", utils.GetEmoji("OCR"), a.Target)
}
