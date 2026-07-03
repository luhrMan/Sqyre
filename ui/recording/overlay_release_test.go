package recording

import (
	"image"
	"testing"

	"fyne.io/fyne/v2/canvas"
)

func TestOverlayDropSnapshotClearsImage(t *testing.T) {
	t.Helper()
	img := image.NewRGBA(image.Rect(0, 0, 64, 64))
	bg := canvas.NewImageFromImage(img)
	o := &fyneDesktopOverlay{bgImage: bg}

	o.dropSnapshot()

	if bg.Image != nil {
		t.Fatal("expected canvas image to be cleared on dropSnapshot")
	}
	if o.bgImage != nil {
		t.Fatal("expected overlay bgImage reference to be cleared")
	}
}

func TestRepositionNoOpsAfterWindowCleared(t *testing.T) {
	t.Helper()
	o := &fyneDesktopOverlay{widthPx: 100, heightPx: 100, bgImage: canvas.NewImageFromImage(nil)}
	o.reposition() // must not panic with nil win
}

func TestStopPositionLoopIsIdempotent(t *testing.T) {
	t.Helper()
	o := &fyneDesktopOverlay{stopPosition: make(chan struct{})}
	o.stopPositionLoop()
	o.stopPositionLoop() // must not panic on second close
}
