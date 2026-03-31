//go:build !js

package editor

import "github.com/go-vgo/robotgo"

func screenPointerAbs() (x, y int) {
	return robotgo.Location()
}
