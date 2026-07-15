package actiondisplay

import (
	"testing"

	"fyne.io/fyne/v2/test"
)

func TestNewDisplayVariablePill(t *testing.T) {
	t.Helper()
	test.NewApp()

	known := map[string]bool{"foundX": true}
	pill := NewDisplayVariablePill("Output X", "foundX", "imagesearch", known)
	if pill.MinSize().Width <= 0 || pill.MinSize().Height <= 0 {
		t.Fatalf("expected non-zero pill size, got %v", pill.MinSize())
	}
	unknown := NewDisplayVariablePill("Output", "newVar", "setvariable", known)
	if unknown.MinSize().Width <= 0 {
		t.Fatal("expected non-zero width for unknown variable pill")
	}
}
