package custom_widgets

import (
	"context"
	"sync/atomic"
)

// One in-flight screen capture keeps tooltip hovers from piling up large preview
// bitmaps when the pointer moves quickly across many rows.
var previewCaptureSlot = make(chan struct{}, 1)

var previewCaptureHeld atomic.Bool

// AcquirePreviewCaptureSlot blocks until the capture slot is free or ctx is cancelled.
func AcquirePreviewCaptureSlot(ctx context.Context) bool {
	select {
	case previewCaptureSlot <- struct{}{}:
		previewCaptureHeld.Store(true)
		return true
	case <-ctx.Done():
		return false
	}
}

// ReleasePreviewCaptureSlot drops the capture slot when the holder finishes normally.
func ReleasePreviewCaptureSlot() {
	if previewCaptureHeld.CompareAndSwap(true, false) {
		select {
		case <-previewCaptureSlot:
		default:
		}
	}
}

// RevokeActivePreviewCapture releases the slot for a cancelled in-flight capture so
// the next hover does not wait for a stale screen grab to finish.
func RevokeActivePreviewCapture() {
	ReleasePreviewCaptureSlot()
}
