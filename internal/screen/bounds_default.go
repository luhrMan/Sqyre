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
	if r, ok := headlessVirtualBounds(); ok {
		return r
	}
	n := screenshot.NumActiveDisplays()
	if n <= 0 {
		w, h := robotgoScreenSizeFallback()
		return image.Rect(0, 0, w, h)
	}
	var u image.Rectangle
	for i := 0; i < n; i++ {
		u = u.Union(DisplayBoundsAbs(i))
	}
	if u.Empty() {
		w, h := robotgoScreenSizeFallback()
		return image.Rect(0, 0, w, h)
	}
	return u
}
