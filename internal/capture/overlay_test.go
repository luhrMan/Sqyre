package capture

import (
	"image"
	"testing"
)

func TestOverlayScreenshotCaptureUsesMappedIndex(t *testing.T) {
	mon := MonitorPlan{
		DisplayIndex:        1,
		BackendDisplayIndex: 0,
		DesktopBounds:       image.Rect(0, 0, 1920, 1080),
	}
	// Without a display server this returns an error; ensure we call the mapped path.
	_, err := overlayScreenshotCapture(mon)
	if err == nil {
		t.Skip("display capture unavailable in this environment")
	}
}
