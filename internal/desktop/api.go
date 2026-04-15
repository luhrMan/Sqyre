// Package desktop abstracts OS-level automation (screen capture, mouse position, etc.)
// so the ui package does not import robotgo or gocv. For cross-compilation or CGO-free
// checks, build with -tags sqyre_no_desktop_native (stub implementation).
package desktop

import (
	"errors"
	"image"
)

// ErrUnavailable is returned by stub implementations when native desktop access is disabled.
var ErrUnavailable = errors.New("desktop automation unavailable")

// Bridge is the automation surface used by the Fyne UI layer.
type Bridge interface {
	Location() (x, y int)
	MilliSleep(ms int)
	CaptureImg(x, y, w, h int) (image.Image, error)
	SavePng(img image.Image, path string) error
	// PixelColorHex returns RGB hex without "#", typically 6 hex digits.
	PixelColorHex(x, y int) string
	ProcessID() int
	WindowBounds(pid int) (x, y, width, height int)
	SetMouseSleep(ms int)
	SetKeySleep(ms int)

	// Input automation
	Move(x, y int)
	MoveSmooth(x, y int, low, high float64)
	MouseToggle(btn string, downUp ...string)
	KeyDown(key string) error
	KeyUp(key string) error
	TypeChar(s string)
	ClipboardWrite(text string)

	// Window management
	FindWindowNames() ([]string, error)
	ActiveWindowByName(name string) error
}

// Default is the process-wide bridge. The native build sets this in init.
var Default Bridge = noopBridge{}

type noopBridge struct{}

func (noopBridge) Location() (int, int) { return 0, 0 }

func (noopBridge) MilliSleep(int) {}

func (noopBridge) CaptureImg(int, int, int, int) (image.Image, error) {
	return nil, ErrUnavailable
}

func (noopBridge) SavePng(image.Image, string) error { return ErrUnavailable }

func (noopBridge) PixelColorHex(int, int) string { return "808080" }

func (noopBridge) ProcessID() int { return 0 }

func (noopBridge) WindowBounds(int) (int, int, int, int) { return 0, 0, 0, 0 }

func (noopBridge) SetMouseSleep(int) {}

func (noopBridge) SetKeySleep(int) {}

func (noopBridge) Move(int, int) {}

func (noopBridge) MoveSmooth(int, int, float64, float64) {}

func (noopBridge) MouseToggle(string, ...string) {}

func (noopBridge) KeyDown(string) error { return ErrUnavailable }

func (noopBridge) KeyUp(string) error { return ErrUnavailable }

func (noopBridge) TypeChar(string) {}

func (noopBridge) ClipboardWrite(string) {}

func (noopBridge) FindWindowNames() ([]string, error) { return nil, ErrUnavailable }

func (noopBridge) ActiveWindowByName(string) error { return ErrUnavailable }
