//go:build (!linux || wayland) && !windows

package recording

import (
	"image"

	"fyne.io/fyne/v2"
)

func positionFyneOverlayWindow(win fyne.Window, absBounds image.Rectangle) {
	win.SetFullScreen(true)
}
