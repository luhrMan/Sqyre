package capture

import (
	"Sqyre/internal/panicsafe"
	"Sqyre/internal/screen"
	"fmt"
	"image"
	"image/draw"
	"log"
	"sync"

	"github.com/go-vgo/robotgo"
)

// captureMu serializes robotgo screen capture. The native backend is not safe
// under concurrent capture calls (e.g. macro image search + UI preview).
var captureMu sync.Mutex

// CaptureToRGBA returns img as an *image.RGBA, reusing it directly when possible
// so callers can scan the raw pixel buffer instead of using per-pixel At() calls.
func CaptureToRGBA(img image.Image) *image.RGBA {
	if rgba, ok := img.(*image.RGBA); ok {
		return rgba
	}
	b := img.Bounds()
	out := image.NewRGBA(image.Rect(0, 0, b.Dx(), b.Dy()))
	draw.Draw(out, out.Bounds(), img, b.Min, draw.Src)
	return out
}

// ValidateSearchAreaBounds crops out-of-bounds edges to enabled displays and reports
// whether the resulting rectangle has positive size.
func ValidateSearchAreaBounds(leftX, topY, rightX, bottomY int) (lx, ty, rx, by, w, h int, err error) {
	return screen.ValidateSearchAreaRect(leftX, topY, rightX, bottomY)
}

// CaptureRect captures a screen rectangle via robotgo. Calls are serialized and
// panics from the native backend are recovered and returned as errors.
func CaptureRect(x, y, w, h int) (image.Image, error) {
	var img image.Image
	var capErr error
	captureMu.Lock()
	func() {
		defer func() {
			if r := recover(); r != nil {
				panicsafe.LogPanicToFile(r, "CaptureRect")
				capErr = fmt.Errorf("screen capture panic: %v", r)
			}
		}()
		img, capErr = robotgo.CaptureImg(x, y, w, h)
	}()
	captureMu.Unlock()
	if capErr != nil {
		return nil, capErr
	}
	if img == nil {
		return nil, fmt.Errorf("screen capture returned nil image")
	}
	return img, nil
}

// CaptureVirtualDesktop captures the union of enabled displays in absolute
// virtual-desktop coordinates (same space as robotgo.Location).
func CaptureVirtualDesktop() (image.Image, image.Rectangle, error) {
	vb := screen.VirtualBounds()
	if vb.Empty() || vb.Dx() <= 0 || vb.Dy() <= 0 {
		return nil, vb, fmt.Errorf("invalid virtual bounds %v", vb)
	}
	img, err := CaptureRect(vb.Min.X, vb.Min.Y, vb.Dx(), vb.Dy())
	if err != nil {
		return nil, vb, err
	}
	return CaptureToRGBA(img), vb, nil
}

// CaptureSearchArea captures the given search rectangle after validating bounds.
// Uses robotgo.CaptureImg because search areas are stored in absolute virtual-desktop
// coordinates (same space as robotgo.Location). screenshot.Capture expects coords
// relative to the primary monitor origin and breaks multi-monitor macros.
// Panics from the native capture backend are recovered and returned as errors.
func CaptureSearchArea(leftX, topY, rightX, bottomY int) (image.Image, int, int, int, int, error) {
	lx, ty, rx, by, w, h, err := ValidateSearchAreaBounds(leftX, topY, rightX, bottomY)
	if err != nil {
		return nil, lx, ty, rx, by, err
	}

	var img image.Image
	var capErr error
	img, capErr = CaptureRect(lx, ty, w, h)
	if capErr != nil {
		log.Printf("CaptureSearchArea: capture failed: %v", capErr)
		return nil, lx, ty, rx, by, capErr
	}
	return img, lx, ty, rx, by, nil
}
