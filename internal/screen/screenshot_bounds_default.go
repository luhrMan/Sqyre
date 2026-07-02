//go:build !linux

package screen

import (
	"image"

	"github.com/vcaesar/screenshot"
)

// ScreenshotDisplayBoundsAbs returns screenshot monitor bounds in absolute desktop coordinates.
func ScreenshotDisplayBoundsAbs(displayIndex int) image.Rectangle {
	return screenshot.GetDisplayBounds(displayIndex)
}
