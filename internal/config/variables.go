package config

import (
	"strconv"

	"Sqyre/internal/screen"

	"github.com/go-vgo/robotgo"
)

// Primary display size and resolution key use absolute display 0 bounds (same space as robotgo).
var (
	primaryAbs            = screen.DisplayBoundsAbs(0)
	MonitorWidth          = primaryAbs.Dx()
	MonitorHeight         = primaryAbs.Dy()
	MainMonitorSize       = robotgo.Rect{Point: robotgo.Point{X: primaryAbs.Min.X, Y: primaryAbs.Min.Y}, Size: robotgo.Size{W: MonitorWidth, H: MonitorHeight}}
	MainMonitorSizeString = strconv.Itoa(MonitorWidth) + "x" + strconv.Itoa(MonitorHeight)
)
