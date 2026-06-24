package recording

import (
	"image"
	"testing"
)

func TestClipRectToMonitor(t *testing.T) {
	monitor := image.Rect(1920, 0, 3840, 1080)

	lx, ty, rx, by, ok := clipRectToMonitor(2000, 100, 2100, 200, monitor)
	if !ok {
		t.Fatal("expected intersection")
	}
	if lx != 2000 || ty != 100 || rx != 2100 || by != 200 {
		t.Fatalf("got (%d,%d)-(%d,%d), want (2000,100)-(2100,200)", lx, ty, rx, by)
	}

	_, _, _, _, ok = clipRectToMonitor(0, 0, 100, 100, monitor)
	if ok {
		t.Fatal("expected no intersection with other monitor")
	}

	lx, ty, rx, by, ok = clipRectToMonitor(1800, 50, 2000, 150, monitor)
	if !ok {
		t.Fatal("expected partial intersection")
	}
	if lx != 1920 || ty != 50 || rx != 2000 || by != 150 {
		t.Fatalf("got clipped (%d,%d)-(%d,%d), want (1920,50)-(2000,150)", lx, ty, rx, by)
	}
}

func TestClipRectToMonitorNormalizesInvertedCoords(t *testing.T) {
	monitor := image.Rect(0, 0, 1920, 1080)
	lx, ty, rx, by, ok := clipRectToMonitor(300, 400, 100, 200, monitor)
	if !ok {
		t.Fatal("expected intersection after normalization")
	}
	if lx != 100 || ty != 200 || rx != 300 || by != 400 {
		t.Fatalf("got (%d,%d)-(%d,%d)", lx, ty, rx, by)
	}
}
