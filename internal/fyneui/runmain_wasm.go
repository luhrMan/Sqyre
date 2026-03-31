//go:build js && wasm

package fyneui

import "fyne.io/fyne/v2"

// RunOnMain schedules fn on the Fyne main thread when the caller may be on the browser event path.
func RunOnMain(fn func()) {
	fyne.Do(fn)
}
