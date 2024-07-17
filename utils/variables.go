package utils

import "github.com/go-vgo/robotgo"

var (
	MonitorWidth, MonitorHeight = robotgo.GetScreenSize()
	_, _, XOffset, YOffset      = robotgo.GetDisplayBounds(1)
)
