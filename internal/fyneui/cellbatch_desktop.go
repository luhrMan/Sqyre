//go:build !js || !wasm

package fyneui

// RunCellUpdatesBatched runs fn immediately. Desktop GridWrap updates are already on the Fyne main thread.
func RunCellUpdatesBatched(fn func()) {
	fn()
}
