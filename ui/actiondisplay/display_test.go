package actiondisplay

import (
	"testing"

	"Sqyre/internal/models/actions"

	"fyne.io/fyne/v2/test"
)

func TestDisplayFromParams_nestedVariableRef(t *testing.T) {
	t.Helper()
	test.NewApp()

	params := actions.NewSetVariable("count", "${step}").Params()
	known := map[string]bool{"step": true}
	line, _, actionType := DisplayFromParams(params, known)
	if actionType != "setvariable" {
		t.Fatalf("actionType = %q, want setvariable", actionType)
	}
	if line.MinSize().Width <= 0 {
		t.Fatalf("expected inline display width, got %v", line.MinSize())
	}
}

func TestNewDisplayValuePill_nestedVariableRef(t *testing.T) {
	t.Helper()
	test.NewApp()

	known := map[string]bool{"count": true}
	pill := NewDisplayValuePill("${count}", "setvariable", known)
	if pill.MinSize().Width <= 0 {
		t.Fatalf("expected non-zero pill width, got %v", pill.MinSize())
	}
}
