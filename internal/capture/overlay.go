package capture

import (
	"Sqyre/internal/macro"
	"image"
	"fmt"

	"github.com/vcaesar/screenshot"
)

// OverlayMonitorImage captures one monitor for the recording overlay. Screenshot
// per-monitor capture is preferred when desktop mapping is available because it
// avoids virtual-desktop stitch/crop issues on asymmetric X11 layouts.
func OverlayMonitorImage(plan SessionPlan, session Session, displayIndex int) (image.Image, string, error) {
	mon, ok := monitorByIndex(plan.Monitors, displayIndex)
	if !ok {
		return nil, "", fmt.Errorf("overlay capture: unknown display index %d", displayIndex)
	}
	if img, err := overlayScreenshotCapture(mon); err == nil && img != nil {
		return img, "screenshot-display-overlay", nil
	}
	img, err := session.CaptureMonitor(displayIndex)
	return img, string(plan.Backend), err
}

func overlayScreenshotCapture(mon MonitorPlan) (image.Image, error) {
	ssIndex := mon.BackendDisplayIndex
	if ssIndex < 0 {
		ssIndex = mon.DisplayIndex
	}
	img, err := screenshot.CaptureDisplay(ssIndex)
	if err != nil || img == nil {
		return nil, err
	}
	return ensureMonitorSize(macro.CaptureToRGBA(img), mon.DesktopBounds)
}

func applyScreenshotMappingToPlan(plan *SessionPlan, specs []monitorSpec) {
	mapping, err := resolveScreenshotMapping(specs)
	if err != nil {
		return
	}
	applyScreenshotMapping(plan, mapping)
}
