package services

import (
	"Sqyre/internal/screen"
	"fmt"
	"image"
	"log"

	"github.com/go-vgo/robotgo"
)

// ValidateSearchAreaBounds crops out-of-bounds edges to enabled displays and reports
// whether the resulting rectangle has positive size.
func ValidateSearchAreaBounds(leftX, topY, rightX, bottomY int) (lx, ty, rx, by, w, h int, err error) {
	return screen.ValidateSearchAreaRect(leftX, topY, rightX, bottomY)
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
	func() {
		defer func() {
			if r := recover(); r != nil {
				LogPanicToFile(r, "CaptureSearchArea")
				capErr = fmt.Errorf("screen capture panic: %v", r)
			}
		}()
		img, capErr = robotgo.CaptureImg(lx, ty, w, h)
	}()
	if capErr != nil {
		log.Printf("CaptureSearchArea: capture failed: %v", capErr)
		return nil, lx, ty, rx, by, capErr
	}
	if img == nil {
		err := fmt.Errorf("screen capture returned nil image")
		log.Printf("CaptureSearchArea: %v", err)
		return nil, lx, ty, rx, by, err
	}
	return img, lx, ty, rx, by, nil
}
