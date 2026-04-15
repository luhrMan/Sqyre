package config

import (
	"strconv"

	"Sqyre/internal/screen"
)

var (
	primaryAbs            = screen.DisplayBoundsAbs(0)
	MonitorWidth          = primaryAbs.Dx()
	MonitorHeight         = primaryAbs.Dy()
	MainMonitorSizeString = strconv.Itoa(MonitorWidth) + "x" + strconv.Itoa(MonitorHeight)
)
