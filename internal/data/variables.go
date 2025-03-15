package data

import (
	"github.com/go-vgo/robotgo"
)

var (
	MainMonitorSize       = robotgo.GetDisplayRect(0)
	MonitorWidth          = MainMonitorSize.W
	MonitorHeight         = MainMonitorSize.H
	XOffset, YOffset      = findOffsets()
	RootPath              = "./"
	InternalPath          = RootPath + "internal/"
	DataPath              = InternalPath + "data/"
	ResourcePath          = DataPath + "resources/"
	ImagesPath            = ResourcePath + "images/"
	MaskImagesPath        = ImagesPath + "masks/"
	CalibrationImagesPath = ImagesPath + "calibration/"
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
