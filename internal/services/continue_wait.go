package services

import "errors"

// ContinueWaitOptions configures blocking until a user-configured continue chord is pressed.
type ContinueWaitOptions struct {
	Keys        []string
	PassThrough bool
	OnMatch     func()
}

var continueKeyWaitFn func(ContinueWaitOptions) error

// SetContinueKeyWaitFunc registers the platform hook implementation (from macrohotkey at init).
func SetContinueKeyWaitFunc(fn func(ContinueWaitOptions) error) {
	continueKeyWaitFn = fn
}

// WaitForContinueKey blocks until the user presses the configured continue chord.
func WaitForContinueKey(opts ContinueWaitOptions) error {
	if continueKeyWaitFn == nil {
		return errors.New("pause: continue key wait is not available in this build")
	}
	return continueKeyWaitFn(opts)
}

var suspendMacroHotkeysFn func()
var resumeMacroHotkeysFn func()

// SetMacroHotkeySuspendFuncs registers macro hotkey suspend/resume (from macrohotkey at init).
func SetMacroHotkeySuspendFuncs(suspend, resume func()) {
	suspendMacroHotkeysFn = suspend
	resumeMacroHotkeysFn = resume
}

func suspendMacroHotkeysForPause() {
	if suspendMacroHotkeysFn != nil {
		suspendMacroHotkeysFn()
	}
}

func resumeMacroHotkeysForPause() {
	if resumeMacroHotkeysFn != nil {
		resumeMacroHotkeysFn()
	}
}