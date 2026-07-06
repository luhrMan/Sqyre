package actiondisplay

import (
	"testing"

	"fyne.io/fyne/v2/test"
)

func TestNewDisplayLabeledPill_nestedVariableRef(t *testing.T) {
	t.Helper()
	test.NewApp()

	known := map[string]bool{"count": true}
	pill := NewDisplayLabeledPill("Value", "${count}", "setvariable", known)
	if pill.MinSize().Width <= 0 || pill.MinSize().Height <= 0 {
		t.Fatalf("expected non-zero pill size, got %v", pill.MinSize())
	}
	expr := NewDisplayLabeledPill("Expr", "1+${count}", "calculate", known)
	if expr.MinSize().Width <= pill.MinSize().Width {
		t.Fatalf("expression pill should be wider than a single ref pill")
	}
}
