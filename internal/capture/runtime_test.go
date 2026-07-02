package capture

import (
	"image"
	"testing"
)

func TestEnsureMonitorSizeReturnsExpectedDimensions(t *testing.T) {
	src := image.NewRGBA(image.Rect(0, 0, 640, 480))
	_, err := ensureMonitorSize(src, image.Rect(0, 0, 320, 240))
	if err == nil {
		t.Fatal("expected geometry mismatch error")
	}
}

func TestEnsureMonitorSizeKeepsMatchingSize(t *testing.T) {
	src := image.NewRGBA(image.Rect(0, 0, 800, 600))
	got, err := ensureMonitorSize(src, image.Rect(0, 0, 800, 600))
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if got.Bounds().Dx() != 800 || got.Bounds().Dy() != 600 {
		t.Fatalf("unexpected size %dx%d", got.Bounds().Dx(), got.Bounds().Dy())
	}
}
