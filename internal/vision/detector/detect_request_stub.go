//go:build !detector_onnx

package detector

import "fmt"

// ExecuteWorkerRequest is unavailable without detector_onnx.
func ExecuteWorkerRequest(req WorkerRequest) (WorkerResponse, error) {
	return WorkerResponse{}, fmt.Errorf("detector_onnx build required")
}

// PreloadModels is a no-op when ONNX is not linked.
func PreloadModels() error { return nil }
