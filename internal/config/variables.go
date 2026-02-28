package config

import (
	"strconv"

	"github.com/go-vgo/robotgo"
)

var (
	MainMonitorSize       = robotgo.GetDisplayRect(0)
	MonitorWidth          = MainMonitorSize.W
	MonitorHeight         = MainMonitorSize.H
	MainMonitorSizeString = strconv.Itoa(MonitorWidth) + "x" + strconv.Itoa(MonitorHeight)
)
