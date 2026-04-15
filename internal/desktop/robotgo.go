//go:build !sqyre_no_desktop_native

package desktop

import (
	"image"
	"os"
	"runtime"
	"strings"

	"github.com/go-vgo/robotgo"
)

func init() {
	if runtime.GOOS == "linux" && os.Getenv("WAYLAND_DISPLAY") != "" {
		Default = waylandBridge{}
		return
	}
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

func (robotgoBridge) Move(x, y int) { robotgo.Move(x, y) }

func (robotgoBridge) MoveSmooth(x, y int, low, high float64) {
	robotgo.MoveSmooth(x, y, low, high)
}

func (robotgoBridge) MouseToggle(btn string, downUp ...string) {
	if len(downUp) > 0 {
		robotgo.Toggle(btn, downUp[0])
	} else {
		robotgo.Toggle(btn)
	}
}

func (robotgoBridge) KeyDown(key string) error { return robotgo.KeyDown(key) }

func (robotgoBridge) KeyUp(key string) error { return robotgo.KeyUp(key) }

func (robotgoBridge) TypeChar(s string) { robotgo.Type(s) }

func (robotgoBridge) ClipboardWrite(text string) { robotgo.WriteAll(text) }

func (robotgoBridge) FindWindowNames() ([]string, error) { return robotgo.FindNames() }

func (robotgoBridge) ActiveWindowByName(name string) error { return robotgo.ActiveName(name) }
