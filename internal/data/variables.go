package data

import "github.com/go-vgo/robotgo"

var (
	MainMonitorSize  = robotgo.GetDisplayRect(0)
	MonitorWidth     = MainMonitorSize.W
	MonitorHeight    = MainMonitorSize.H
	XOffset, YOffset = findOffsets()
	ResPath          = "./internal/data/resources/"
	imagesPath       = ResPath + "images/"
	masksPath        = imagesPath + "masks/"
)

func findOffsets() (X, Y int) {
	for d := range robotgo.DisplaysNum() {
		x, y, _, _ := robotgo.GetDisplayBounds(d)
		if x < 0 {
			X = x * -1
		}
		if y < 0 {
			Y = y * -1
		}
	}
	return X, Y
}
