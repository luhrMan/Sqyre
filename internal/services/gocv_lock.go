package services

import "sync"

// openCVMu serializes OpenCV C calls. gocv is not safe for concurrent reads of
// the same Mat (e.g. one blurred capture shared by parallel template matchers).
var openCVMu sync.Mutex
