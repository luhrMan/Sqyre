package actiondisplay

import (
	"testing"

	"fyne.io/fyne/v2/test"
)

func TestPillDropdown_SelectsOption(t *testing.T) {
	t.Helper()
	test.NewApp()
	drop := NewPillDropdown("op", []string{"==", "!=", ">"}, "==", nil)
	if drop.Value != "==" {
		t.Fatalf("Value = %q, want ==", drop.Value)
	}
	drop.setValue(">")
	if drop.Value != ">" {
		t.Fatalf("after select Value = %q, want >", drop.Value)
	}
	if drop.valueText.Text != ">" {
		t.Fatalf("display = %q, want >", drop.valueText.Text)
	}
}

func TestPillToggle_SetLabel(t *testing.T) {
	t.Helper()
	test.NewApp()
	toggle := NewPillToggle("Match any (OR)", true)
	toggle.SetLabel("Match all (AND)")
	if toggle.Label != "Match all (AND)" {
		t.Fatalf("Label = %q, want Match all (AND)", toggle.Label)
	}
	if toggle.labelText.Text != "Match all (AND)" {
		t.Fatalf("labelText = %q, want Match all (AND)", toggle.labelText.Text)
	}
}
