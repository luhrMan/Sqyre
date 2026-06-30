package services

import (
	"sync/atomic"
)

// HighlightKind describes how an action should be highlighted during execution.
type HighlightKind int

const (
	// HighlightNone clears a highlight. When UID is empty it clears every highlight.
	HighlightNone HighlightKind = iota
	// HighlightSimple is the moving "cursor" highlight on the action currently running.
	HighlightSimple
	// HighlightFill is a horizontal progress fill used by container actions
	// (Image Search, Run Macro, For Each) that span multiple steps.
	HighlightFill
)

// HighlightEvent is sent to the UI to update the execution highlight of an action.
type HighlightEvent struct {
	MacroName string
	UID       string
	Kind      HighlightKind
	Fill      float64 // 0..1, only meaningful for HighlightFill
}

var (
	highlightEnabled  atomic.Bool
	highlightCallback func(HighlightEvent)
)

// SetHighlightCallback registers the UI handler invoked when the active-action
// highlight changes. The handler is responsible for marshalling onto the UI thread.
func SetHighlightCallback(fn func(HighlightEvent)) {
	highlightCallback = fn
}

// SetHighlightEnabled toggles the active-action highlight feature.
func SetHighlightEnabled(enabled bool) {
	highlightEnabled.Store(enabled)
}

// HighlightEnabled reports whether the feature is currently on.
func HighlightEnabled() bool {
	return highlightEnabled.Load()
}

// ClearHighlights removes all highlights regardless of the enabled flag (used on
// macro completion and when the feature is turned off mid-run).
func ClearHighlights() {
	if highlightCallback != nil {
		highlightCallback(HighlightEvent{Kind: HighlightNone, UID: ""})
	}
}

func highlightCursor(macroName, uid string) {
	if !highlightEnabled.Load() || highlightCallback == nil {
		return
	}
	highlightCallback(HighlightEvent{MacroName: macroName, UID: uid, Kind: HighlightSimple})
}

func highlightFill(macroName, uid string, fill float64) {
	if !highlightEnabled.Load() || highlightCallback == nil {
		return
	}
	if fill < 0 {
		fill = 0
	} else if fill > 1 {
		fill = 1
	}
	highlightCallback(HighlightEvent{MacroName: macroName, UID: uid, Kind: HighlightFill, Fill: fill})
}

func highlightClear(macroName, uid string) {
	if !highlightEnabled.Load() || highlightCallback == nil {
		return
	}
	highlightCallback(HighlightEvent{MacroName: macroName, UID: uid, Kind: HighlightNone})
}
