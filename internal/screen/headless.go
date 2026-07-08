package screen

import (
	"image"
	"os"
)

const headlessWidth = 1920
const headlessHeight = 1080

// headlessVirtualBounds reports a stable desktop size when real display queries are
// unavailable or would invoke robotgo without a working X display (headless CI/tests).
func headlessVirtualBounds() (image.Rectangle, bool) {
	if os.Getenv("SQYRE_UI_TEST") == "1" || os.Getenv("DISPLAY") == "" {
		return image.Rect(0, 0, headlessWidth, headlessHeight), true
	}
	return image.Rectangle{}, false
}

func robotgoScreenSizeFallback() (w, h int) {
	if r, ok := headlessVirtualBounds(); ok {
		return r.Dx(), r.Dy()
	}
	w, h = robotgoGetScreenSize()
	return w, h
}
