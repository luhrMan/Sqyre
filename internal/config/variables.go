package config

import (
	"github.com/go-vgo/robotgo"
	"github.com/spf13/viper"
)

var (
	ViperConfig      = viper.New()
	MainMonitorSize  = robotgo.GetDisplayRect(0)
	MonitorWidth     = MainMonitorSize.W
	MonitorHeight    = MainMonitorSize.H
	XOffset, YOffset = findOffsets()
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
