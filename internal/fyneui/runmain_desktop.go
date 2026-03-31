//go:build !js || !wasm

package fyneui

// RunOnMain runs fn immediately. Desktop widget callbacks already run on the Fyne main thread;
// avoid fyne.Do here so virtualized widgets (GridWrap/List) stay synchronous and fast.
func RunOnMain(fn func()) {
	fn()
}
