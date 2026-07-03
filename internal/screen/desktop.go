package screen

import (
	"image"
	"sync"

	"github.com/go-vgo/robotgo"
)

// DesktopBackend abstracts mouse position, pixel color, display geometry, and
// lightweight desktop I/O so UI code can be tested without robotgo.
type DesktopBackend interface {
	Location() (x, y int)
	GetPixelColor(x, y int) string
	GetScreenSize() (w, h int)
	NumDisplays() int
	GetDisplayBounds(display int) (x, y, w, h int)
	SavePNG(img image.Image, path string) error
	WriteClipboard(text string) error
	ProcessWindowBounds() (x, y, width, height int)
}

type robotgoDesktopBackend struct{}

func (robotgoDesktopBackend) Location() (int, int) {
	return robotgo.Location()
}

func (robotgoDesktopBackend) GetPixelColor(x, y int) string {
	return robotgo.GetPixelColor(x, y)
}

func (robotgoDesktopBackend) GetScreenSize() (int, int) {
	return robotgo.GetScreenSize()
}

func (robotgoDesktopBackend) NumDisplays() int {
	return numDisplaysImpl()
}

func (robotgoDesktopBackend) GetDisplayBounds(display int) (int, int, int, int) {
	return robotgo.GetDisplayBounds(display)
}

func (robotgoDesktopBackend) SavePNG(img image.Image, path string) error {
	return robotgo.SavePng(img, path)
}

func (robotgoDesktopBackend) WriteClipboard(text string) error {
	return robotgo.WriteAll(text)
}

func (robotgoDesktopBackend) ProcessWindowBounds() (int, int, int, int) {
	pid := robotgo.GetPid()
	return robotgo.GetBounds(pid)
}

var (
	desktopBackend     DesktopBackend = robotgoDesktopBackend{}
	desktopBackendMu   sync.RWMutex
)

func getDesktopBackend() DesktopBackend {
	desktopBackendMu.RLock()
	defer desktopBackendMu.RUnlock()
	return desktopBackend
}

// SetDesktopBackend replaces the global desktop backend (tests only).
func SetDesktopBackend(b DesktopBackend) {
	desktopBackendMu.Lock()
	desktopBackend = b
	desktopBackendMu.Unlock()
}

// ResetDesktopBackend restores the default robotgo desktop backend.
func ResetDesktopBackend() {
	SetDesktopBackend(robotgoDesktopBackend{})
}

func Location() (x, y int) {
	return getDesktopBackend().Location()
}

func GetPixelColor(x, y int) string {
	return getDesktopBackend().GetPixelColor(x, y)
}

func GetScreenSize() (w, h int) {
	return getDesktopBackend().GetScreenSize()
}

func GetDisplayBounds(display int) (x, y, w, h int) {
	return getDesktopBackend().GetDisplayBounds(display)
}

func SavePNG(img image.Image, path string) error {
	return getDesktopBackend().SavePNG(img, path)
}

func WriteClipboard(text string) error {
	return getDesktopBackend().WriteClipboard(text)
}

func ProcessWindowBounds() (x, y, width, height int) {
	return getDesktopBackend().ProcessWindowBounds()
}
