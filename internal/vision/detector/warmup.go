package detector

import (
	"log"
	"sync"
	"time"
)

var (
	warmUpMu   sync.Mutex
	warmUpPath string
)

// WarmUpOnce starts the persistent worker and loads ONNX models (idempotent per path).
func WarmUpOnce(workerPath string) {
	warmUpMu.Lock()
	defer warmUpMu.Unlock()
	if warmUpPath == workerPath {
		return
	}
	start := time.Now()
	if err := StartWorker(workerPath); err != nil {
		log.Printf("Vision warmup: %v", err)
		return
	}
	warmUpPath = workerPath
	log.Printf("Vision worker ready in %s", time.Since(start).Round(time.Millisecond))
}

// ResetWarmUpForTesting clears warmup state.
func ResetWarmUpForTesting() {
	warmUpMu.Lock()
	warmUpPath = ""
	warmUpMu.Unlock()
	resetRemoteWorkers()
}
