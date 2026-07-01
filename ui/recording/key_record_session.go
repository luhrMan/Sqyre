package recording

import "sync/atomic"

var keyRecordSessionActive atomic.Bool

func setKeyRecordSessionActive(active bool) {
	keyRecordSessionActive.Store(active)
}

// KeyRecordSessionActive reports whether the single-key recorder dialog is open.
func KeyRecordSessionActive() bool {
	return keyRecordSessionActive.Load()
}
