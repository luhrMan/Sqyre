package vision

import "sync"

// openCVMu serializes OpenCV C calls. gocv/OpenCV are not safe for concurrent
// use from multiple goroutines, even on distinct Mats.
var openCVMu sync.Mutex

func OpenCVLock()   { openCVMu.Lock() }
func OpenCVUnlock() { openCVMu.Unlock() }

// WithOpenCV runs fn while holding the process-wide OpenCV lock.
func WithOpenCV(fn func()) {
	openCVMu.Lock()
	defer openCVMu.Unlock()
	fn()
}
