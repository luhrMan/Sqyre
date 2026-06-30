package screen

import (
	"image"
	"testing"
)

func TestRectFullyOnDisplaysRejectsMonitorGap(t *testing.T) {
	displays := []image.Rectangle{
		image.Rect(0, 0, 1920, 1080),
		image.Rect(2560, 0, 4480, 1080),
	}
	gapRect := image.Rect(2000, 0, 2100, 100)
	if rectFullyOnDisplays(gapRect, displays) {
		t.Fatal("expected gap rect to be rejected")
	}
	onDisplay := image.Rect(100, 50, 200, 150)
	if !rectFullyOnDisplays(onDisplay, displays) {
		t.Fatal("expected on-display rect to be accepted")
	}
	spanning := image.Rect(1900, 0, 1920, 100)
	if !rectFullyOnDisplays(spanning, displays) {
		t.Fatal("expected rect on single monitor edge to be accepted")
	}
	pastEdge := image.Rect(1900, 0, 2000, 100)
	if rectFullyOnDisplays(pastEdge, displays) {
		t.Fatal("expected rect extending past monitor edge to be rejected")
	}
	partialGap := image.Rect(1900, 0, 2100, 100)
	if rectFullyOnDisplays(partialGap, displays) {
		t.Fatal("expected partially-in-gap rect to be rejected")
	}
}

func TestValidateSearchAreaOnDisplaysRejectsNonPositiveDimensions(t *testing.T) {
	displays := []image.Rectangle{image.Rect(0, 0, 1920, 1080)}
	_, _, _, _, _, _, err := validateSearchAreaOnDisplays(10, 20, 10, 50, displays)
	if err == nil {
		t.Fatal("expected zero-width error")
	}
}

func TestValidateSearchAreaOnDisplaysNormalizesSwappedCoords(t *testing.T) {
	displays := []image.Rectangle{image.Rect(0, 0, 1920, 1080)}
	lx, ty, rx, by, w, h, err := validateSearchAreaOnDisplays(100, 200, 50, 150, displays)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if lx != 50 || ty != 150 || rx != 100 || by != 200 || w != 50 || h != 50 {
		t.Fatalf("got (%d,%d,%d,%d) %dx%d", lx, ty, rx, by, w, h)
	}
}

func TestValidateSearchAreaOnDisplaysCropsPastMonitorEdges(t *testing.T) {
	displays := []image.Rectangle{image.Rect(0, 0, 1920, 1080)}

	lx, ty, rx, by, w, h, err := validateSearchAreaOnDisplays(-50, -20, 100, 100, displays)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if lx != 0 || ty != 0 || rx != 100 || by != 100 || w != 100 || h != 100 {
		t.Fatalf("got (%d,%d,%d,%d) %dx%d", lx, ty, rx, by, w, h)
	}

	lx, ty, rx, by, w, h, err = validateSearchAreaOnDisplays(1800, 900, 2000, 1100, displays)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if lx != 1800 || ty != 900 || rx != 1920 || by != 1080 || w != 120 || h != 180 {
		t.Fatalf("got (%d,%d,%d,%d) %dx%d", lx, ty, rx, by, w, h)
	}
}

func TestValidateSearchAreaOnDisplaysCropsPastSingleMonitorEdge(t *testing.T) {
	displays := []image.Rectangle{
		image.Rect(0, 0, 1920, 1080),
		image.Rect(2560, 0, 4480, 1080),
	}
	lx, ty, rx, by, w, h, err := validateSearchAreaOnDisplays(1900, 0, 2000, 100, displays)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if lx != 1900 || ty != 0 || rx != 1920 || by != 100 || w != 20 || h != 100 {
		t.Fatalf("got (%d,%d,%d,%d) %dx%d", lx, ty, rx, by, w, h)
	}
}
