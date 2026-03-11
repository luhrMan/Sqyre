//go:build !android

package desktop

import (
	"image"
	"image/color"
	"image/png"
	"os"

	"github.com/go-vgo/robotgo"
	hook "github.com/luhrMan/gohook"
	"gocv.io/x/gocv"
)

func MousePosition() (x, y int) {
	return robotgo.Location()
}

func MilliSleep(ms int) {
	robotgo.MilliSleep(ms)
}

func ScreenSize() (w, h int) {
	return robotgo.GetScreenSize()
}

func DisplayCount() int {
	return robotgo.DisplaysNum()
}

func DisplayBounds(displayIndex int) (x, y, w, h int) {
	x, y, w, h = robotgo.GetDisplayBounds(displayIndex)
	return x, y, w, h
}

func CaptureRegion(x, y, w, h int) (image.Image, error) {
	bm, err := robotgo.CaptureImg(x, y, w, h)
	if err != nil || bm == nil {
		return nil, err
	}
	mat, err := gocv.ImageToMatRGB(bm)
	if err != nil {
		return nil, err
	}
	defer mat.Close()
	return mat.ToImage()
}

func SavePNG(img image.Image, path string) error {
	f, err := os.Create(path)
	if err != nil {
		return err
	}
	defer f.Close()
	return png.Encode(f, img)
}

func GetPixelColor(x, y int) string {
	return robotgo.GetPixelColor(x, y)
}

func DrawCrosshairOnImage(img image.Image, px, py int) (image.Image, error) {
	mat, err := gocv.ImageToMatRGB(img)
	if err != nil {
		return nil, err
	}
	defer mat.Close()
	red := color.RGBA{R: 255, A: 255}
	center := image.Point{X: px, Y: py}
	gocv.Circle(&mat, center, 8, red, 2)
	gocv.Line(&mat, image.Point{X: px - 15, Y: py}, image.Point{X: px + 15, Y: py}, red, 2)
	gocv.Line(&mat, image.Point{X: px, Y: py - 15}, image.Point{X: px, Y: py + 15}, red, 2)
	return mat.ToImage()
}

func DrawRectOnImage(img image.Image, r image.Rectangle) (image.Image, error) {
	mat, err := gocv.ImageToMatRGB(img)
	if err != nil {
		return nil, err
	}
	defer mat.Close()
	red := color.RGBA{R: 255, A: 255}
	gocv.Rectangle(&mat, r, red, 2)
	return mat.ToImage()
}

// RegisterMouseDown registers a callback for mouse down; returns an unregister function.
func RegisterMouseDown(cb func(x, y, button int)) func() {
	hook.Register(hook.MouseDown, []string{}, func(e hook.Event) {
		x, y := robotgo.Location()
		btn := 0
		if e.Button == hook.MouseMap["left"] {
			btn = MouseButtonLeft
		} else if e.Button == hook.MouseMap["right"] {
			btn = MouseButtonRight
		} else {
			btn = MouseButtonMiddle
		}
		cb(x, y, btn)
	})
	return func() {
		go hook.Unregister(hook.MouseDown, []string{})
	}
}

// SetMouseKeySleep sets robotgo sleep values (used by binders/macro).
func SetMouseKeySleep(mouseMs, keyMs int) {
	robotgo.MouseSleep = mouseMs
	robotgo.KeySleep = keyMs
}
