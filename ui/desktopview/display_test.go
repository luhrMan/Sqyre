package desktopview

import (
	"image"
	"testing"
)

func TestClipAbsoluteRectToVirtualLocal(t *testing.T) {
	vb := image.Rect(1920, 0, 3840, 1080)

	local, ok := ClipAbsoluteRectToVirtualLocal(2000, 100, 2100, 200, vb)
	if !ok {
		t.Fatal("expected intersection on second monitor")
	}
	if local != image.Rect(80, 100, 180, 200) {
		t.Fatalf("got %v, want (80,100)-(180,200)", local)
	}

	_, ok = ClipAbsoluteRectToVirtualLocal(0, 0, 100, 100, vb)
	if ok {
		t.Fatal("expected no intersection with primary monitor")
	}

	local, ok = ClipAbsoluteRectToVirtualLocal(1800, 50, 2000, 150, vb)
	if !ok {
		t.Fatal("expected partial intersection")
	}
	if local != image.Rect(0, 50, 80, 150) {
		t.Fatalf("got clipped %v, want (0,50)-(80,150)", local)
	}
}

func TestClipAbsoluteRectToVirtualLocalNormalizesInvertedCoords(t *testing.T) {
	vb := image.Rect(0, 0, 1920, 1080)
	local, ok := ClipAbsoluteRectToVirtualLocal(300, 400, 100, 200, vb)
	if !ok {
		t.Fatal("expected intersection after normalization")
	}
	if local != image.Rect(100, 200, 300, 400) {
		t.Fatalf("got %v", local)
	}
}

func TestClipAbsoluteRectToVirtualLocalNegativeOrigin(t *testing.T) {
	vb := image.Rect(-1920, 0, 0, 1080)
	local, ok := ClipAbsoluteRectToVirtualLocal(-1800, 100, -1700, 200, vb)
	if !ok {
		t.Fatal("expected intersection on left monitor")
	}
	if local != image.Rect(120, 100, 220, 200) {
		t.Fatalf("got %v, want (120,100)-(220,200)", local)
	}
}
