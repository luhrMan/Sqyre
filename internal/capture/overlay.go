package capture

import (
	"Sqyre/internal/macro"
	"image"
	"fmt"

	"github.com/vcaesar/screenshot"
)

// OverlayMonitorImage captures one monitor for the recording overlay. Robotgo is
// preferred because search areas and mouse positions use the same absolute
// virtual-desktop space as macro.CaptureSearchArea / OCR capture. Screenshot is
// only a fallback when robotgo capture fails.
func OverlayMonitorImage(plan SessionPlan, session Session, displayIndex int) (image.Image, string, error) {
	mon, ok := monitorByIndex(plan.Monitors, displayIndex)
	if !ok {
		return nil, "", fmt.Errorf("overlay capture: unknown display index %d", displayIndex)
	}
	if img, err := overlayRobotgoCapture(mon); err == nil && img != nil {
		return img, "robotgo-overlay", nil
	}
	if img, err := overlayScreenshotCapture(mon); err == nil && img != nil {
		return img, "screenshot-display-overlay", nil
	}
	img, err := session.CaptureMonitor(displayIndex)
	return img, string(plan.Backend), err
}

func overlayRobotgoCapture(mon MonitorPlan) (image.Image, error) {
	b := mon.DesktopBounds
	img, err := macro.CaptureRect(b.Min.X, b.Min.Y, b.Dx(), b.Dy())
	if err != nil || img == nil {
		return nil, err
	}
	return macro.CaptureToRGBA(img), nil
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
