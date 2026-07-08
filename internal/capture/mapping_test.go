package capture

import (
	"image"
	"testing"
)

func TestValidateScreenshotAlignmentRejectsMismatchedBounds(t *testing.T) {
	specs := []monitorSpec{
		{displayIndex: 0, bounds: image.Rect(1920, 0, 4480, 1440)},
	}
	mapping := map[int]int{0: 0}
	boundsFor := func(int) image.Rectangle { return image.Rect(0, 0, 1920, 1080) }
	if err := validateScreenshotAlignment(specs, mapping, boundsFor); err == nil {
		t.Fatal("expected alignment error when screenshot bounds do not match desktop bounds")
	}
}

func TestValidateScreenshotAlignmentAcceptsMatchingBounds(t *testing.T) {
	specs := []monitorSpec{
		{displayIndex: 0, bounds: image.Rect(1920, 0, 4480, 1440)},
	}
	mapping := map[int]int{0: 0}
	boundsFor := func(int) image.Rectangle { return image.Rect(1920, 0, 4480, 1440) }
	if err := validateScreenshotAlignment(specs, mapping, boundsFor); err != nil {
		t.Fatalf("expected no error when bounds match, got %v", err)
	}
}
