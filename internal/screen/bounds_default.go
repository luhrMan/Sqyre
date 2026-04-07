//go:build !linux

package screen

import (
	"image"

	"github.com/vcaesar/screenshot"
)

func displayBoundsAbsImpl(displayIndex int) image.Rectangle {
	r := screenshot.GetDisplayBounds(displayIndex)
	if r.Empty() {
		return image.Rectangle{}
	}
	return r
}

func virtualBoundsImpl() image.Rectangle {
	n := screenshot.NumActiveDisplays()
	if n <= 0 {
		r := screenshot.GetDisplayBounds(0)
		if !r.Empty() {
			return r
		}
		return image.Rect(0, 0, 1920, 1080)
	}
	var u image.Rectangle
	for i := 0; i < n; i++ {
		u = u.Union(DisplayBoundsAbs(i))
	}
	if u.Empty() {
		r := screenshot.GetDisplayBounds(0)
		if !r.Empty() {
			return r
		}
		return image.Rect(0, 0, 1920, 1080)
	}
	return u
}
