//go:build android

package config

import "strconv"

type displayRect struct{ W, H int }

var (
	MainMonitorSize       = displayRect{W: 1080, H: 1920}
	MonitorWidth          = MainMonitorSize.W
	MonitorHeight         = MainMonitorSize.H
	MainMonitorSizeString = strconv.Itoa(MonitorWidth) + "x" + strconv.Itoa(MonitorHeight)
)
