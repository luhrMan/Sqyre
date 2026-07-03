package vision

import (
	"Sqyre/internal/config"
	"image"
	"testing"
)

func TestPreviewCaptureBoundsForPoint(t *testing.T) {
	t.Helper()
	vb := image.Rect(0, 0, 1920, 1080)
	half := config.EditorPreviewMinCaptureSize / 2

	got := shiftRectIntoVirtualBounds(image.Rect(500-half, 400-half, 500+half, 400+half), vb)
	want := image.Rect(500-half, 400-half, 500+half, 400+half)
	if got != want {
		t.Fatalf("centered point: got %v want %v", got, want)
	}

	got = shiftRectIntoVirtualBounds(image.Rect(10-half, 10-half, 10+half, 10+half), vb)
	if got.Min.X != 0 || got.Min.Y != 0 {
		t.Fatalf("corner point should shift to origin, got %v", got)
	}
	if got.Dx() != config.EditorPreviewMinCaptureSize || got.Dy() != config.EditorPreviewMinCaptureSize {
		t.Fatalf("corner point should keep size, got %v", got)
	}
}

func TestPreviewCaptureBoundsForSearchArea(t *testing.T) {
	t.Helper()
	vb := image.Rect(0, 0, 1920, 1080)

	got := shiftRectIntoVirtualBounds(expandRectToMinSize(image.Rect(80, 80, 120, 120), config.EditorPreviewMinCaptureSize, config.EditorPreviewMinCaptureSize), vb)
	if got.Dx() < config.EditorPreviewMinCaptureSize || got.Dy() < config.EditorPreviewMinCaptureSize {
		t.Fatalf("small area should expand to min capture size, got %v", got)
	}
	if !got.In(vb) {
		t.Fatalf("capture bounds %v should stay inside virtual bounds %v", got, vb)
	}

	expanded := expandRectToMinSize(image.Rect(150, 100, 100, 250), config.EditorPreviewMinCaptureSize, config.EditorPreviewMinCaptureSize)
	if expanded.Min.X > expanded.Max.X || expanded.Min.Y > expanded.Max.Y {
		t.Fatal("inverted coords should be normalized before expand")
	}
}

func TestExpandRectToMinSize(t *testing.T) {
	t.Helper()
	r := image.Rect(100, 100, 120, 130)
	got := expandRectToMinSize(r, 200, 200)
	if got.Dx() < 200 || got.Dy() < 200 {
		t.Fatalf("got %v", got)
	}
	cx := (got.Min.X + got.Max.X) / 2
	cy := (got.Min.Y + got.Max.Y) / 2
	if cx != 110 || cy != 115 {
		t.Fatalf("expected center preserved, got center (%d,%d)", cx, cy)
	}
}

func TestShiftRectIntoVirtualBounds(t *testing.T) {
	t.Helper()
	vb := image.Rect(0, 0, 100, 100)
	desired := image.Rect(80, 80, 180, 180)
	got := shiftRectIntoVirtualBounds(desired, vb)
	if got.Max.X > vb.Max.X || got.Max.Y > vb.Max.Y {
		t.Fatalf("shifted rect %v exceeds bounds %v", got, vb)
	}
	if got.Dx() != 100 || got.Dy() != 100 {
		t.Fatalf("should clamp to virtual bounds size, got %v", got)
	}
}
