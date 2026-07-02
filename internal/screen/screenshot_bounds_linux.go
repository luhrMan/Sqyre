//go:build linux

package screen

import (
	"image"

	"github.com/vcaesar/screenshot"
)

// ScreenshotDisplayBoundsAbs returns screenshot monitor bounds translated into the
// same absolute desktop coordinate space as DisplayBoundsAbs.
func ScreenshotDisplayBoundsAbs(displayIndex int) image.Rectangle {
	rel := screenshot.GetDisplayBounds(displayIndex)
	if rel.Empty() {
		return image.Rectangle{}
	}
	if ox, oy, ok := xineramaPrimaryOrigin(); ok {
		return rel.Add(image.Pt(ox, oy))
	}
	return rel
}
