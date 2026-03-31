//go:build js

package screen

import (
	"image"

	"Sqyre/internal/config"
)

// NumDisplays returns 1 for the browser demo.
func NumDisplays() int { return 1 }

// DisplayBoundsAbs returns a single full logical monitor in absolute coordinates.
func DisplayBoundsAbs(displayIndex int) image.Rectangle {
	if displayIndex != 0 {
		return image.Rectangle{}
	}
	return image.Rect(0, 0, config.MonitorWidth, config.MonitorHeight)
}

// VirtualBounds matches the single logical display.
func VirtualBounds() image.Rectangle {
	return DisplayBoundsAbs(0)
}

// MonitorIndexAt returns 0 when the point lies in the logical desktop, else 0.
func MonitorIndexAt(absX, absY int) int {
	b := DisplayBoundsAbs(0)
	p := image.Pt(absX, absY)
	if b.Empty() || p.In(b) {
		return 0
	}
	return 0
}

// MonitorIndexForOverlay always uses display 0 in WASM.
func MonitorIndexForOverlay() int { return 0 }
