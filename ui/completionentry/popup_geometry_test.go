package completionentry

import (
	"testing"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/test"
	"fyne.io/fyne/v2/widget"
)

func TestPopupWidthFor_usesLabelContent(t *testing.T) {
	t.Helper()
	test.NewApp()
	host := widget.NewEntry()
	host.Resize(fyne.NewSize(40, 32))

	narrow := popupWidthFor(host, host.Size().Width, []string{"short"})
	wide := popupWidthFor(host, host.Size().Width, []string{"foundX · output from image search"})

	if wide <= narrow {
		t.Fatalf("wide label width %v should exceed narrow label width %v", wide, narrow)
	}
	if wide < minCompletionPopupWidth {
		t.Fatalf("popup width %v below minimum %v", wide, minCompletionPopupWidth)
	}
}

func TestPopupWidthFor_respectsHostWhenWider(t *testing.T) {
	t.Helper()
	test.NewApp()
	host := widget.NewEntry()
	host.Resize(fyne.NewSize(360, 32))

	got := popupWidthFor(host, host.Size().Width, []string{"x"})
	if got != 360 {
		t.Fatalf("popupWidthFor() = %v, want host width 360", got)
	}
}

func TestClampPopupX_keepsPopupOnCanvas(t *testing.T) {
	t.Helper()
	got := clampPopupX(900, 220, 1000, 8)
	if got != 772 {
		t.Fatalf("clampPopupX() = %v, want 772", got)
	}
}
