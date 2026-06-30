package vision

import (
	"sync"

	"gocv.io/x/gocv"
)

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

// CloseMat releases an OpenCV matrix when it holds native memory. Safe on zero
// or already-released Mats (avoids double-free crashes from gocv).
func CloseMat(m *gocv.Mat) {
	if m == nil || m.Ptr() == nil {
		return
	}
	m.Close()
	*m = gocv.Mat{}
}
