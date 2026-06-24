package actiondialog

import (
	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/widget"
	ttwidget "github.com/dweymouth/fyne-tooltip/widget"
)

func createBreakDialogContent() (fyne.CanvasObject, func()) {
	label := ttwidget.NewLabel("Exits the innermost enclosing loop (Loop, For each row, or Image Search match iteration).")
	label.Wrapping = fyne.TextWrapWord
	return widget.NewForm(formHint("Break", label, "Place inside a loop container. Has no effect when not inside a loop.")), func() {}
}

func createContinueDialogContent() (fyne.CanvasObject, func()) {
	label := ttwidget.NewLabel("Skips the rest of the current loop iteration and continues with the next iteration.")
	label.Wrapping = fyne.TextWrapWord
	return widget.NewForm(formHint("Continue", label, "Place inside a loop container. Has no effect when not inside a loop.")), func() {}
}
