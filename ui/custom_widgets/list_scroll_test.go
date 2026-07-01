package custom_widgets

import (
	"testing"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/test"
	"fyne.io/fyne/v2/widget"
)

func TestRefreshListPreservingScroll(t *testing.T) {
	test.NewApp()
	w := test.NewWindow(nil)
	defer w.Close()

	items := []string{"a", "b", "c", "d", "e", "f", "g", "h", "i", "j"}
	list := widget.NewList(
		func() int { return len(items) },
		func() fyne.CanvasObject { return widget.NewLabel("") },
		func(id widget.ListItemID, co fyne.CanvasObject) {
			co.(*widget.Label).SetText(items[id])
		},
	)
	list.Resize(fyne.NewSize(200, 80))
	w.SetContent(list)
	list.ScrollToOffset(40)
	want := list.GetScrollOffset()

	items = items[:5]
	RefreshListPreservingScroll(list)

	if got := list.GetScrollOffset(); got != want {
		t.Fatalf("scroll offset = %v, want %v", got, want)
	}
}
