//go:build !detector_onnx

package detector

func onnxDetectorAvailable() bool { return false }

func tryNewONNXDetector() (Detector, bool) { return nil, false }
