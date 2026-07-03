package vision

import (
	"Sqyre/internal/models"
	"testing"
)

func TestPointPreviewCaption(t *testing.T) {
	t.Helper()
	got := PointPreviewCaption(&models.Point{X: 100, Y: 200})
	want := "X: 100, Y: 200"
	if got != want {
		t.Fatalf("PointPreviewCaption() = %q, want %q", got, want)
	}
}

func TestSearchAreaPreviewCaption(t *testing.T) {
	t.Helper()
	got := SearchAreaPreviewCaption(&models.SearchArea{LeftX: 10, TopY: 20, RightX: 110, BottomY: 80})
	want := "Left: 10, Top: 20, Right: 110, Bottom: 80"
	if got != want {
		t.Fatalf("SearchAreaPreviewCaption() = %q, want %q", got, want)
	}
}
