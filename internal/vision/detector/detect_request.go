//go:build detector_onnx

package detector

import (
	"context"
	"fmt"
	"image"
	_ "image/png"
	"os"
)

// ExecuteWorkerRequest runs one detect call in-process (sqyre-vision worker).
func ExecuteWorkerRequest(req WorkerRequest) (WorkerResponse, error) {
	f, err := os.Open(req.ImagePath)
	if err != nil {
		return WorkerResponse{}, fmt.Errorf("open image: %w", err)
	}
	defer f.Close()
	img, _, err := image.Decode(f)
	if err != nil {
		return WorkerResponse{}, fmt.Errorf("decode image: %w", err)
	}
	opts := Options{
		Prompts:             req.Prompts,
		ConfidenceThreshold: req.ConfidenceThreshold,
		IoUThreshold:        req.IoUThreshold,
		MaxMatches:          req.MaxMatches,
		InputSize:           req.InputSize,
	}
	dets, err := GetDetector().Detect(context.Background(), img, opts)
	if err != nil {
		return WorkerResponse{}, err
	}
	return WorkerResponse{Detections: localDetectionsToWorker(dets)}, nil
}

func localDetectionsToWorker(dets []Detection) []WorkerDetection {
	out := make([]WorkerDetection, len(dets))
	for i, d := range dets {
		out[i] = WorkerDetection{
			Label:      d.Label,
			Confidence: d.Confidence,
			Bounds: WorkerBounds{
				MinX: d.Bounds.Min.X,
				MinY: d.Bounds.Min.Y,
				MaxX: d.Bounds.Max.X,
				MaxY: d.Bounds.Max.Y,
			},
		}
	}
	return out
}
