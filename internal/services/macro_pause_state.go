package services

import (
	"strings"
	"sync/atomic"
	"time"
)

const macroEscapeSuppressGrace = 250 * time.Millisecond

var (
	macroPauseWaiting       atomic.Bool
	macroPauseContinueEsc   atomic.Bool
	macroKeyActionEscape    atomic.Bool
	macroEscSuppressUntil   atomic.Int64
)

// IsEscapeKey reports whether key is the Escape key in hook or robotgo form.
func IsEscapeKey(key string) bool {
	switch strings.ToLower(strings.TrimSpace(key)) {
	case "esc", "escape":
		return true
	default:
		return false
	}
}

// BeginMacroPauseWait marks the running macro as waiting for a continue chord.
// When continueKeyIsEscape is true, a lone Escape press resumes instead of stopping.
func BeginMacroPauseWait(continueKeyIsEscape bool) {
	macroPauseWaiting.Store(true)
	macroPauseContinueEsc.Store(continueKeyIsEscape)
}

// EndMacroPauseWait clears the pause-wait state after the continue chord or stop.
func EndMacroPauseWait() {
	macroPauseWaiting.Store(false)
	macroPauseContinueEsc.Store(false)
}

// BeginMacroKeyActionEscape suppresses macro stop on Escape while a Key action sends it.
func BeginMacroKeyActionEscape() {
	macroKeyActionEscape.Store(true)
}

// EndMacroKeyActionEscape clears Key-action Escape suppression.
func EndMacroKeyActionEscape() {
	macroKeyActionEscape.Store(false)
}

// extendMacroEscapeSuppress ignores macro-stop Escape briefly after a Key action
// sends it. gohook may deliver the synthetic KeyDown after robotgo returns.
func extendMacroEscapeSuppress(d time.Duration) {
	if d <= 0 {
		return
	}
	until := time.Now().Add(d).UnixNano()
	for {
		old := macroEscSuppressUntil.Load()
		if until <= old {
			return
		}
		if macroEscSuppressUntil.CompareAndSwap(old, until) {
			return
		}
	}
}

func macroEscapeSuppressActive() bool {
	return time.Now().UnixNano() < macroEscSuppressUntil.Load()
}

// ShouldEscapeStopMacro reports whether Escape should request a macro stop right now.
func ShouldEscapeStopMacro() bool {
	if !IsMacroRunning() {
		return false
	}
	if macroPauseWaiting.Load() && macroPauseContinueEsc.Load() {
		return false
	}
	if macroKeyActionEscape.Load() || macroEscapeSuppressActive() {
		return false
	}
	return true
}
