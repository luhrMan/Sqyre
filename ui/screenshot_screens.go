package ui

import "fyne.io/fyne/v2"

// EditorScreenForScreenshot returns the data editor layout for docs/tests.
func EditorScreenForScreenshot(u *Ui) fyne.CanvasObject {
	EnsureDataEditor()
	return u.EditorUi.CanvasObject
}
