package dialogs

import (
	"testing"

	"fyne.io/fyne/v2/test"
	"fyne.io/fyne/v2/widget"
)

func TestWrapModalPopup_OnClosedRunsOnHide(t *testing.T) {
	w := test.NewWindow(widget.NewLabel("parent"))
	t.Cleanup(w.Close)

	pop := widget.NewPopUp(widget.NewLabel("popup"), w.Canvas())
	d := WrapModalPopup(pop)

	closed := false
	d.SetOnClosed(func() { closed = true })
	d.Hide()

	if !closed {
		t.Fatal("expected onClosed to run when dialog Hide is called")
	}
}
