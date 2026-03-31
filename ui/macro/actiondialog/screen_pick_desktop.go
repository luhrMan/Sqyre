//go:build !js

package actiondialog

import "github.com/go-vgo/robotgo"

func screenPointerXY() (x, y int) {
	return robotgo.Location()
}

func pixelColorHexAt(x, y int) string {
	return robotgo.GetPixelColor(x, y)
}
