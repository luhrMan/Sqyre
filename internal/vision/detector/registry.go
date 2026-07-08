package detector

import (
	"context"
	"fmt"
	"image"
	"sync"
)

var (
	detectorMu          sync.RWMutex
	activeDetector      Detector
	forceInProcessONNX  bool
	cachedInProcess     Detector
	cachedInProcessMu   sync.Mutex
	remoteDetectorMu    sync.Mutex
	remoteDetector      Detector
	remoteDetectorPath  string
)

// SetForceInProcessONNX makes GetDetector use local ONNX only (sqyre-vision worker).
func SetForceInProcessONNX(v bool) {
	detectorMu.Lock()
	forceInProcessONNX = v
	detectorMu.Unlock()
}

// GetDetector returns the process-wide detector (injected in tests, else default).
func GetDetector() Detector {
	detectorMu.RLock()
	d := activeDetector
	force := forceInProcessONNX
	detectorMu.RUnlock()
	if d != nil {
		return d
	}
	if force {
		cachedInProcessMu.Lock()
		defer cachedInProcessMu.Unlock()
		if cachedInProcess != nil {
			return cachedInProcess
		}
		cachedInProcess = defaultDetector()
		return cachedInProcess
	}
	return defaultDetector()
}

// SetDetectorForTesting replaces the global detector. Pass nil to restore default.
func SetDetectorForTesting(d Detector) {
	detectorMu.Lock()
	activeDetector = d
	detectorMu.Unlock()
	cachedInProcessMu.Lock()
	cachedInProcess = nil
	cachedInProcessMu.Unlock()
}

func defaultDetector() Detector {
	detectorMu.RLock()
	inProcess := forceInProcessONNX
	detectorMu.RUnlock()
	if !inProcess {
		if path := ResolveWorkerPath(); path != "" {
			return getOrCreateRemoteDetector(path)
		}
	}
	if d, ok := tryNewONNXDetector(); ok {
		return d
	}
	if inProcess {
		return unavailableDetector{}
	}
	return StubDetector{}
}

// unavailableDetector is used when sqyre-vision is forced in-process but ONNX cannot load.
type unavailableDetector struct{}

func (unavailableDetector) Detect(_ context.Context, _ image.Image, _ Options) ([]Detection, error) {
	return nil, fmt.Errorf("vision ONNX runtime unavailable (run make vision-models and ensure libonnxruntime matches go.mod)")
}

func getOrCreateRemoteDetector(path string) Detector {
	remoteDetectorMu.Lock()
	defer remoteDetectorMu.Unlock()
	if remoteDetector != nil && remoteDetectorPath == path {
		return remoteDetector
	}
	remoteDetector = NewRemoteDetector(path)
	remoteDetectorPath = path
	return remoteDetector
}

func resetCachedRemoteDetector() {
	remoteDetectorMu.Lock()
	remoteDetector = nil
	remoteDetectorPath = ""
	remoteDetectorMu.Unlock()
}
