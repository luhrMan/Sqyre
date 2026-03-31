//go:build js

package services

import (
	"Sqyre/internal/models"
	"log"

	"fyne.io/fyne/v2"
)

var (
	showMacroLogPopupFunc func(macroName string)
	macroRunningCallback  func(running bool)
)

// SetShowMacroLogPopupFunc sets the callback to show the macro log popup (browser: optional UI).
func SetShowMacroLogPopupFunc(fn func(macroName string)) {
	showMacroLogPopupFunc = fn
}

// SetMacroRunningCallback sets the callback when macro execution starts or stops.
func SetMacroRunningCallback(fn func(running bool)) {
	macroRunningCallback = fn
}

// ExecuteMacroWithLogging is a no-op for WASM except optional log UI bookkeeping.
func ExecuteMacroWithLogging(m *models.Macro) {
	if m == nil {
		return
	}
	defer func() {
		fyne.Do(func() {
			if macroRunningCallback != nil {
				macroRunningCallback(false)
			}
		})
		if r := recover(); r != nil {
			LogPanicToFile(r, "Macro "+m.Name)
		}
	}()
	if showMacroLogPopupFunc != nil {
		fyne.DoAndWait(func() {
			showMacroLogPopupFunc(m.Name)
			if macroRunningCallback != nil {
				macroRunningCallback(true)
			}
		})
		defer StopMacroLogCapture()
	} else if macroRunningCallback != nil {
		fyne.DoAndWait(func() { macroRunningCallback(true) })
	}
	log.Printf("WASM shell: macro %q — desktop automation is not available in the browser", m.Name)
}

// SuspendMacroHotkeys is a no-op on WASM.
func SuspendMacroHotkeys() {}

// ResumeMacroHotkeys is a no-op on WASM.
func ResumeMacroHotkeys() {}

// RegisterMacroHotkey is a no-op on WASM.
func RegisterMacroHotkey(m *models.Macro) {}

// UnregisterMacroHotkey is a no-op on WASM.
func UnregisterMacroHotkey(m *models.Macro) {}

// UnregisterHotkeyKeys is a no-op on WASM.
func UnregisterHotkeyKeys(hk []string, trigger string) {}

// ActiveWindowNames returns an empty list in the browser build.
func ActiveWindowNames() ([]string, error) {
	return nil, nil
}

// GoSafe runs fn in a goroutine; panic recovery matches desktop (see paniclog.go).
// Re-declared here only if paniclog's GoSafe is not visible — actually paniclog has GoSafe for all builds.
// Remove duplicate — GoSafe is in paniclog.go untagged.
