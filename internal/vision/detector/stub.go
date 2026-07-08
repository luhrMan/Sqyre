package detector

import (
	"context"
	"image"
)

// StubDetector returns no detections. Used in CI and when ONNX models are unavailable.
type StubDetector struct{}

func (StubDetector) Detect(_ context.Context, _ image.Image, _ Options) ([]Detection, error) {
	return nil, nil
}
