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
