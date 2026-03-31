//go:build js

package config

import "strconv"

// Fixed logical resolution for the WASM shell (no robotgo / native screen APIs).
const (
	MonitorWidth  = 1920
	MonitorHeight = 1080
)

// MainMonitorSizeString matches program coordinate keys (e.g. Points per resolution).
var MainMonitorSizeString = strconv.Itoa(MonitorWidth) + "x" + strconv.Itoa(MonitorHeight)

// MainMonitorSize is a neutral placeholder; desktop builds use robotgo.Rect.
var MainMonitorSize = struct {
	Point struct{ X, Y int }
	Size  struct{ W, H int }
}{
	Point: struct{ X, Y int }{X: 0, Y: 0},
	Size:  struct{ W, H int }{W: MonitorWidth, H: MonitorHeight},
}
