//go:build linux

package screen

import "github.com/go-vgo/robotgo"

func robotgoGetScreenSize() (int, int) {
	return robotgo.GetScreenSize()
}
