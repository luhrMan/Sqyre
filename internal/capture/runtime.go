package capture

import (
	"fmt"
	"image"

	"github.com/vcaesar/screenshot"
)

type Session interface {
	CaptureMonitor(displayIndex int) (image.Image, error)
}

type session struct {
	plan SessionPlan
}

func NewSession(plan SessionPlan) (Session, error) {
	if len(plan.Monitors) == 0 {
		return nil, fmt.Errorf("capture session plan has no monitors")
	}
	return &session{plan: plan}, nil
}

func (s *session) CaptureVirtual() (image.Image, image.Rectangle, error) {
	img, vb, err := CaptureVirtualDesktop()
	if err != nil {
		return nil, image.Rectangle{}, err
	}
	return CaptureToRGBA(img), vb, nil
}

func (s *session) CaptureMonitor(displayIndex int) (image.Image, error) {
	mon, ok := monitorByIndex(s.plan.Monitors, displayIndex)
	if !ok {
		return nil, fmt.Errorf("capture session: unknown display index %d", displayIndex)
	}
	switch s.plan.Backend {
	case BackendRobotgoMonitorRect:
		img, err := CaptureRect(mon.DesktopBounds.Min.X, mon.DesktopBounds.Min.Y, mon.DesktopBounds.Dx(), mon.DesktopBounds.Dy())
		if err != nil {
			return nil, err
		}
		return CaptureToRGBA(img), nil
	case BackendRobotgoVirtual:
		full, vb, err := s.CaptureVirtual()
		if err != nil {
			return nil, err
		}
		img := cropVirtualCapture(full, vb, mon.DesktopBounds)
		if img == nil {
			return nil, fmt.Errorf("capture session: virtual crop failed for display %d", displayIndex)
		}
		return img, nil
	case BackendScreenshotDisplay:
		ssIndex := mon.BackendDisplayIndex
		if ssIndex < 0 {
			ssIndex = displayIndex
		}
		img, err := screenshot.CaptureDisplay(ssIndex)
		if err != nil {
			return nil, err
		}
		return ensureMonitorSize(CaptureToRGBA(img), mon.DesktopBounds)
	case BackendScreenshotRect:
		img, err := screenshot.CaptureRect(mon.BackendBounds)
		if err != nil {
			return nil, err
		}
		return ensureMonitorSize(CaptureToRGBA(img), mon.DesktopBounds)
	default:
		return nil, fmt.Errorf("capture session: unsupported backend %s", s.plan.Backend)
	}
}

func ensureMonitorSize(img image.Image, expected image.Rectangle) (image.Image, error) {
	if img == nil {
		return nil, fmt.Errorf("capture session: monitor capture is nil")
	}
	b := img.Bounds()
	if b.Dx() == expected.Dx() && b.Dy() == expected.Dy() {
		return img, nil
	}
	return nil, fmt.Errorf(
		"capture session: geometry mismatch expected=%dx%d got=%dx%d",
		expected.Dx(), expected.Dy(), b.Dx(), b.Dy(),
	)
}
