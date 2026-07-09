package custom_widgets

import (
	"testing"

	"fyne.io/fyne/v2"
)

func TestImageContentRectFitsAtZoomOne(t *testing.T) {
	t.Helper()
	viewport := fyne.NewSize(400, 300)
	imageSize := fyne.NewSize(800, 600)
	x, y, w, h := ImageContentRect(viewport, imageSize, ResetImageViewTransform())
	if w != 400 || h != 300 {
		t.Fatalf("fit size = (%v,%v), want (400,300)", w, h)
	}
	if x != 0 || y != 0 {
		t.Fatalf("fit position = (%v,%v), want (0,0)", x, y)
	}
}

func TestZoomImageAtCursorKeepsPointUnderCursor(t *testing.T) {
	t.Helper()
	viewport := fyne.NewSize(400, 300)
	imageSize := fyne.NewSize(400, 300)
	cursor := fyne.NewPos(300, 150)
	beforeX, beforeY, beforeW, beforeH := ImageContentRect(viewport, imageSize, ResetImageViewTransform())
	u := (cursor.X - beforeX) / beforeW
	v := (cursor.Y - beforeY) / beforeH

	tform := ZoomImageAtCursor(viewport, imageSize, ResetImageViewTransform(), cursor, 2)
	afterX, afterY, afterW, afterH := ImageContentRect(viewport, imageSize, tform)
	gotX := afterX + u*afterW
	gotY := afterY + v*afterH
	const eps = 0.01
	if diff(gotX, cursor.X) > eps || diff(gotY, cursor.Y) > eps {
		t.Fatalf("cursor anchor moved: got (%v,%v), want (%v,%v)", gotX, gotY, cursor.X, cursor.Y)
	}
	if tform.Zoom <= 1 {
		t.Fatalf("expected zoom > 1, got %v", tform.Zoom)
	}
}

func TestClampImagePanCentersWhenSmallerThanViewport(t *testing.T) {
	t.Helper()
	viewport := fyne.NewSize(400, 300)
	imageSize := fyne.NewSize(200, 150)
	tform := ClampImagePan(viewport, imageSize, ImageViewTransform{Zoom: 1, PanX: 50, PanY: -30})
	if tform.PanX != 0 || tform.PanY != 0 {
		t.Fatalf("pan = (%v,%v), want (0,0)", tform.PanX, tform.PanY)
	}
}

func TestScrollZoomFactorIgnoresMagnitude(t *testing.T) {
	t.Helper()
	small := ScrollZoomFactor(1)
	large := ScrollZoomFactor(100)
	if small != large {
		t.Fatalf("expected same step for any positive delta, got %v vs %v", small, large)
	}
	if small <= 1 {
		t.Fatal("positive delta should zoom in")
	}
	if ScrollZoomFactor(-100) >= 1 {
		t.Fatal("negative delta should zoom out")
	}
}

func TestClampImagePanAllowsEdgePadding(t *testing.T) {
	t.Helper()
	viewport := fyne.NewSize(400, 300)
	imageSize := fyne.NewSize(400, 300)
	tform := ImageViewTransform{Zoom: 3}
	tform.PanX = 500
	clamped := ClampImagePan(viewport, imageSize, tform)
	x, _, _, _ := ImageContentRect(viewport, imageSize, clamped)
	if x > imagePanEdgePadding {
		t.Fatalf("left-edge pan should allow up to %v padding, got x=%v", imagePanEdgePadding, x)
	}
}

func diff(a, b float32) float32 {
	if a > b {
		return a - b
	}
	return b - a
}
