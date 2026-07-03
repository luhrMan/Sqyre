package editor

import (
	"testing"

	"Sqyre/internal/models"
	"Sqyre/internal/vision"
)

func TestPointPreviewImageRejectsNil(t *testing.T) {
	t.Helper()
	if _, err := vision.PointPreview(nil); err == nil {
		t.Fatal("expected error for nil point")
	}
}

func TestSearchAreaPreviewImageRejectsNil(t *testing.T) {
	t.Helper()
	if _, err := vision.SearchAreaPreview(nil); err == nil {
		t.Fatal("expected error for nil search area")
	}
}

func TestPointPreviewImageRejectsOutOfBounds(t *testing.T) {
	t.Helper()
	_, err := vision.PointPreview(&models.Point{Name: "far", X: -99999, Y: -99999})
	if err == nil {
		t.Fatal("expected error for point outside virtual desktop")
	}
}
