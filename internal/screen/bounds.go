//go:build !js

// Package screen provides desktop bounds in absolute coordinates: the same
// space as robotgo.GetMousePos / robotgo.CaptureImg (virtual framebuffer / root
// origin), not primary-monitor-relative.
package screen

import (
	"image"

	"github.com/go-vgo/robotgo"
	"github.com/vcaesar/screenshot"
)

// NumDisplays returns the number of active displays.
func NumDisplays() int {
	n := screenshot.NumActiveDisplays()
	if n > 0 {
		return n
	}
	return 1
}

// DisplayBoundsAbs returns the bounds of display i in absolute desktop coordinates
// (top-left of the virtual desktop / root window). Falls back to screenshot
// bounds (already absolute on Windows; primary-relative on some Unix setups).
func DisplayBoundsAbs(displayIndex int) image.Rectangle {
	return displayBoundsAbsImpl(displayIndex)
}

// VirtualBounds returns the union of enabled displays in absolute coordinates.
// When the user has not restricted monitors (empty preference), all displays are included.
func VirtualBounds() image.Rectangle {
	enabled := EnabledMonitorIndices()
	if enabled == nil {
		return virtualBoundsImpl()
	}
	var u image.Rectangle
	for _, i := range enabled {
		u = u.Union(DisplayBoundsAbs(i))
	}
	if u.Empty() {
		return virtualBoundsImpl()
	}
	return u
}

// MonitorIndexAt returns the display index containing the given absolute point,
// or 0 if none match.
func MonitorIndexAt(absX, absY int) int {
	p := image.Pt(absX, absY)
	n := NumDisplays()
	for i := 0; i < n; i++ {
		b := DisplayBoundsAbs(i)
		if !b.Empty() && p.In(b) {
			return i
		}
	}
	return 0
}

// MonitorIndexForOverlay chooses the display that should show the recording overlay:
// the monitor containing the mouse (where the user is about to click).
// If the mouse is on a disabled monitor, the first enabled monitor is used.
func MonitorIndexForOverlay() int {
	x, y := robotgo.Location()
	idx := MonitorIndexAt(x, y)
	if IsMonitorEnabled(idx) {
		return idx
	}
	for _, i := range EnabledMonitorIndices() {
		return i
	}
	return 0
}
