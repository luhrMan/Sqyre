package actiondialog

import (
	"Sqyre/internal/models/actions"
	"testing"

	"fyne.io/fyne/v2"
)

func TestActionDialogSizeComplexUsesMostOfParent(t *testing.T) {
	parent := fyne.NewSize(1200, 900)
	action := &actions.ImageSearch{}

	got := actionDialogSize(parent, action, fyne.NewSize(400, 300))
	wantW := parent.Width * (1 - 2*dialogEdgeGapFraction)
	wantH := parent.Height * (1 - 2*dialogEdgeGapFraction)
	if got.Width != wantW || got.Height != wantH {
		t.Fatalf("complex dialog size = %v, want %v x %v", got, wantW, wantH)
	}
}

func TestActionDialogSizeSimpleShrinksToContent(t *testing.T) {
	parent := fyne.NewSize(1200, 900)
	action := &actions.Click{}
	want := fyne.NewSize(360, dialogMinH)

	got := actionDialogSize(parent, action, want)
	if got.Width != want.Width || got.Height != want.Height {
		t.Fatalf("simple dialog size = %v, want %v", got, want)
	}
}
