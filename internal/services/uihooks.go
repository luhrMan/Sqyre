package services

import "sync"

var (
	runOnUIThread        = func(fn func()) { fn() }
	runOnUIThreadAndWait = func(fn func()) { fn() }

	macroIndicatorMu sync.RWMutex
	macroIndicator   macroIndicatorHooks
)

type macroIndicatorHooks struct {
	show  func()
	hide  func()
	start func()
	stop  func()
}

// MacroIndicatorUI wires the macro-running activity widget from the UI layer.
type MacroIndicatorUI struct {
	Show  func()
	Hide  func()
	Start func()
	Stop  func()
}

// SetRunOnUIThread registers the main-thread dispatcher (e.g. fyne.Do). Tests may omit this.
func SetRunOnUIThread(fn func(func())) {
	if fn == nil {
		runOnUIThread = func(f func()) { f() }
		return
	}
	runOnUIThread = fn
}

// SetRunOnUIThreadAndWait registers a blocking main-thread dispatcher (e.g. fyne.DoAndWait).
func SetRunOnUIThreadAndWait(fn func(func())) {
	if fn == nil {
		runOnUIThreadAndWait = func(f func()) { f() }
		return
	}
	runOnUIThreadAndWait = fn
}

func onUIThreadAndWait(fn func()) {
	runOnUIThreadAndWait(fn)
}

// SetMacroIndicatorUI registers show/hide/start/stop for the macro activity indicator.
func SetMacroIndicatorUI(ui MacroIndicatorUI) {
	macroIndicatorMu.Lock()
	defer macroIndicatorMu.Unlock()
	macroIndicator = macroIndicatorHooks{
		show:  ui.Show,
		hide:  ui.Hide,
		start: ui.Start,
		stop:  ui.Stop,
	}
}

// ResetMacroIndicatorUIForTesting clears indicator hooks.
func ResetMacroIndicatorUIForTesting() {
	SetMacroIndicatorUI(MacroIndicatorUI{})
}

func onUIThread(fn func()) {
	runOnUIThread(fn)
}

func showMacroIndicator() {
	macroIndicatorMu.RLock()
	show := macroIndicator.show
	macroIndicatorMu.RUnlock()
	if show != nil {
		onUIThread(show)
	}
}

func hideMacroIndicator() {
	macroIndicatorMu.RLock()
	hide := macroIndicator.hide
	macroIndicatorMu.RUnlock()
	if hide != nil {
		onUIThread(hide)
	}
}

func startMacroIndicator() {
	macroIndicatorMu.RLock()
	start := macroIndicator.start
	macroIndicatorMu.RUnlock()
	if start != nil {
		onUIThread(start)
	}
}

func stopMacroIndicator() {
	macroIndicatorMu.RLock()
	stop := macroIndicator.stop
	macroIndicatorMu.RUnlock()
	if stop != nil {
		onUIThread(stop)
	}
}
