//go:build js

package ui

import (
	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/dialog"
)

// AddDialogEscapeClose wires the parent window canvas so Escape dismisses the dialog (no global hook in WASM).
func AddDialogEscapeClose(d dialog.Dialog, parent fyne.Window) {
	if d == nil || parent == nil {
		return
	}
	c := parent.Canvas()
	if c == nil {
		return
	}
	prev := c.OnTypedKey()
	esc := func(e *fyne.KeyEvent) {
		if e.Name == fyne.KeyEscape {
			d.Hide()
			return
		}
		if prev != nil {
			prev(e)
		}
	}
	c.SetOnTypedKey(esc)
	d.SetOnClosed(func() {
		c.SetOnTypedKey(prev)
	})
}
