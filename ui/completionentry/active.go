package completionentry

import (
	"sync/atomic"
	"time"
)

var activeCompletions atomic.Int32

// suppressEnterUntilUnixNano is set when completion consumes Enter so a global
// Enter hook (e.g. tooltip save) does not fire after the popup hides.
var suppressEnterUntilUnixNano atomic.Int64

const enterSuppressDuration = 100 * time.Millisecond

func registerCompletionShown() {
	activeCompletions.Add(1)
}

func registerCompletionHidden() {
	for {
		cur := activeCompletions.Load()
		if cur <= 0 {
			return
		}
		if activeCompletions.CompareAndSwap(cur, cur-1) {
			return
		}
	}
}

// IsCompletionActive reports whether a completion popup is currently visible.
func IsCompletionActive() bool {
	return activeCompletions.Load() > 0
}

func suppressTooltipEnter() {
	until := time.Now().Add(enterSuppressDuration).UnixNano()
	for {
		cur := suppressEnterUntilUnixNano.Load()
		if cur >= until {
			return
		}
		if suppressEnterUntilUnixNano.CompareAndSwap(cur, until) {
			return
		}
	}
}

// IsTooltipEnterSuppressed reports whether a recent completion Enter should
// block a global tooltip save handler.
func IsTooltipEnterSuppressed() bool {
	return time.Now().UnixNano() < suppressEnterUntilUnixNano.Load()
}
