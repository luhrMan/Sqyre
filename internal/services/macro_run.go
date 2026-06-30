package services

import (
	"sync"
	"sync/atomic"
)

var (
	macroRunMu         sync.Mutex
	macroRunning       atomic.Bool
	macroStopRequested atomic.Bool
	runningMacroName   string
)

// tryStartMacroRun claims the single macro execution slot. Returns false when another
// macro is already running.
func tryStartMacroRun(name string) bool {
	macroRunMu.Lock()
	defer macroRunMu.Unlock()
	if macroRunning.Load() {
		return false
	}
	macroRunning.Store(true)
	macroStopRequested.Store(false)
	runningMacroName = name
	return true
}

func endMacroRun() {
	macroRunMu.Lock()
	runningMacroName = ""
	macroRunMu.Unlock()
	macroRunning.Store(false)
	macroStopRequested.Store(false)
}

// RequestMacroStop asks the currently running macro to stop at the next checkpoint.
func RequestMacroStop() {
	if macroRunning.Load() {
		macroStopRequested.Store(true)
	}
}

// MacroStopPending reports whether the user has requested the running macro to stop.
func MacroStopPending() bool {
	return macroStopRequested.Load()
}

// IsMacroRunning reports whether a macro is currently executing.
func IsMacroRunning() bool {
	return macroRunning.Load()
}

// RunningMacroName returns the name of the macro currently executing, or "" if none.
func RunningMacroName() string {
	macroRunMu.Lock()
	defer macroRunMu.Unlock()
	return runningMacroName
}
