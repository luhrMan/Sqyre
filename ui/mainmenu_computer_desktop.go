//go:build !js

package ui

import (
	"strconv"

	"github.com/go-vgo/robotgo"
)

func computerInfoText() string {
	var str string
	w, h := robotgo.GetScreenSize()
	str = str + "Total Screen Size: " + strconv.Itoa(w) + "x" + strconv.Itoa(h) + "\n"
	for d := range robotgo.DisplaysNum() {
		_, _, mh, mw := robotgo.GetDisplayBounds(d)
		str = str + "Monitor " + strconv.Itoa(d+1) + " Size: " + strconv.Itoa(mh) + "x" + strconv.Itoa(mw) + "\n"
	}
	return str
}
