package custom_widgets

import (
	"context"
	"image"
	"testing"
	"time"

	"fyne.io/fyne/v2/canvas"
)

func TestAcquirePreviewCaptureSlot_respectsCancel(t *testing.T) {
	t.Helper()
	ReleasePreviewCaptureSlot()

	ctx, cancel := context.WithCancel(context.Background())
	if !AcquirePreviewCaptureSlot(ctx) {
		t.Fatal("expected first acquire to succeed")
	}
	defer ReleasePreviewCaptureSlot()

	ctx2, cancel2 := context.WithCancel(context.Background())
	cancel2()
	if AcquirePreviewCaptureSlot(ctx2) {
		t.Fatal("expected cancelled acquire to fail")
	}

	cancel()
	done := make(chan struct{})
	go func() {
		if AcquirePreviewCaptureSlot(ctx) {
			ReleasePreviewCaptureSlot()
		}
		close(done)
	}()
	select {
	case <-done:
	case <-time.After(time.Second):
		t.Fatal("acquire did not unblock after context cancel")
	}
}

func TestRevokeActivePreviewCapture_unblocksWaiter(t *testing.T) {
	t.Helper()
	ReleasePreviewCaptureSlot()

	if !AcquirePreviewCaptureSlot(context.Background()) {
		t.Fatal("expected first acquire to succeed")
	}

	done := make(chan struct{})
	go func() {
		ctx, cancel := context.WithTimeout(context.Background(), 2*time.Second)
		defer cancel()
		if AcquirePreviewCaptureSlot(ctx) {
			ReleasePreviewCaptureSlot()
		}
		close(done)
	}()

	time.Sleep(20 * time.Millisecond)
	RevokeActivePreviewCapture()

	select {
	case <-done:
	case <-time.After(time.Second):
		t.Fatal("revoke did not unblock waiting acquire")
	}
}

func TestPreviewTooltipPanel_clearPreviewDropsImage(t *testing.T) {
	t.Helper()
	panel := newPreviewTooltipPanel(nil)
	panel.img = canvas.NewImageFromImage(nil)
	panel.setImage(image.NewRGBA(image.Rect(0, 0, 1, 1)), "caption")
	panel.clearPreview()
	if panel.img.Image != nil {
		t.Fatal("clearPreview should drop image reference")
	}
	if panel.showImage {
		t.Fatal("clearPreview should reset showImage")
	}
}
