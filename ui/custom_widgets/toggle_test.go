package custom_widgets

import (
	"testing"

	"fyne.io/fyne/v2/test"
)

func TestCompactToggleFitsPillLineHeight(t *testing.T) {
	test.NewApp()
	toggle := NewCompactToggle(nil)
	lineH := PillLineHeight()
	min := toggle.MinSize()
	if min.Height > lineH+0.5 {
		t.Fatalf("compact toggle height %v exceeds pill line height %v", min.Height, lineH)
	}
	if min.Width <= min.Height {
		t.Fatalf("compact toggle width %v should exceed height %v", min.Width, min.Height)
	}
}
