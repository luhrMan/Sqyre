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
	err := validateScreenshotAlignment(specs, mapping)
	if err == nil {
		t.Fatal("expected alignment error when screenshot bounds do not match desktop bounds")
	}
}
