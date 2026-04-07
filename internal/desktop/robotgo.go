//go:build !sqyre_no_desktop_native

package desktop

import (
	"image"
	"strings"

	"github.com/go-vgo/robotgo"
)

func init() {
	Default = robotgoBridge{}
}

type robotgoBridge struct{}

func (robotgoBridge) Location() (int, int) { return robotgo.Location() }

func (robotgoBridge) MilliSleep(ms int) { robotgo.MilliSleep(ms) }

func (robotgoBridge) CaptureImg(x, y, w, h int) (image.Image, error) {
	return robotgo.CaptureImg(x, y, w, h)
}

func (robotgoBridge) SavePng(img image.Image, path string) error {
	return robotgo.SavePng(img, path)
}

func (robotgoBridge) PixelColorHex(x, y int) string {
	hex := robotgo.GetPixelColor(x, y)
	hex = strings.TrimPrefix(strings.ToLower(hex), "#")
	if len(hex) == 8 {
		hex = hex[2:]
	}
	return hex
}

func (robotgoBridge) ProcessID() int { return robotgo.GetPid() }

func (robotgoBridge) WindowBounds(pid int) (x, y, width, height int) {
	return robotgo.GetBounds(pid)
}

func (robotgoBridge) SetMouseSleep(ms int) { robotgo.MouseSleep = ms }

func (robotgoBridge) SetKeySleep(ms int) { robotgo.KeySleep = ms }
