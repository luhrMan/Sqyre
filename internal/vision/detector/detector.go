// Package detector provides open-vocabulary object detection for macro screen search.
// Production inference uses YOLO-World ONNX (build tag detector_onnx); CI uses a stub.
package detector

import (
	"context"
	"image"
)

// Detection is a single localized match for a text prompt class.
type Detection struct {
	Label      string
	Confidence float32
	// Bounds in source image pixel coordinates (same space as the captured frame).
	Bounds image.Rectangle
}

// Center returns the integer center of Bounds.
func (d Detection) Center() (x, y int) {
	return d.Bounds.Min.X + d.Bounds.Dx()/2, d.Bounds.Min.Y + d.Bounds.Dy()/2
}

// Options tune inference for a single Detect call.
type Options struct {
	// Prompts are the class labels to search for (e.g. "healing potion", "metal armor").
	// Each prompt becomes one row in the text-feature tensor for YOLO-World.
	Prompts []string

	// ConfidenceThreshold drops detections below this score (0–1). Default 0.25.
	ConfidenceThreshold float32

	// IoUThreshold is NMS overlap threshold (0–1). Default 0.45.
	IoUThreshold float32

	// MaxMatches caps returned detections after NMS. 0 = unlimited.
	MaxMatches int

	// InputSize is the square side length fed to the detector (typically 640).
	InputSize int
}

func (o *Options) applyDefaults() {
	if o.ConfidenceThreshold <= 0 {
		o.ConfidenceThreshold = 0.25
	}
	if o.IoUThreshold <= 0 {
		o.IoUThreshold = 0.45
	}
	if o.InputSize <= 0 {
		o.InputSize = 640
	}
}

// Detector finds objects described by text prompts in a screenshot region.
type Detector interface {
	Detect(ctx context.Context, frame image.Image, opts Options) ([]Detection, error)
}

// Available reports whether semantic vision can run (worker or in-process ONNX).
func Available() bool {
	if ResolveWorkerPath() != "" {
		return true
	}
	return onnxDetectorAvailable()
}
