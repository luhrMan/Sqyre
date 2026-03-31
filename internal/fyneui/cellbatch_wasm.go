//go:build js && wasm

package fyneui

import (
	"sync"

	"fyne.io/fyne/v2"
)

// RunCellUpdatesBatched queues fn and runs all queued cell updates inside a single fyne.Do.
// Without batching, each GridWrap cell schedules its own fyne.Do (catastrophic for large grids).
var (
	cellMu      sync.Mutex
	cellQueue   []func()
	cellFlushScheduled bool
)

func RunCellUpdatesBatched(fn func()) {
	cellMu.Lock()
	cellQueue = append(cellQueue, fn)
	if cellFlushScheduled {
		cellMu.Unlock()
		return
	}
	cellFlushScheduled = true
	cellMu.Unlock()

	fyne.Do(flushCellUpdateQueue)
}

func flushCellUpdateQueue() {
	for {
		cellMu.Lock()
		if len(cellQueue) == 0 {
			cellFlushScheduled = false
			cellMu.Unlock()
			return
		}
		batch := cellQueue
		cellQueue = nil
		cellMu.Unlock()

		for _, f := range batch {
			if f != nil {
				f()
			}
		}
	}
}
