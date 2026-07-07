package actiondisplay

import (
	"testing"

	"fyne.io/fyne/v2/test"
)

func TestPillSelect_CyclesOptions(t *testing.T) {
	t.Helper()
	test.NewApp()
	sel := NewPillSelect("Button", []string{"left", "right", "center"}, "left", nil)
	if sel.Value != "left" {
		t.Fatalf("Value = %q, want left", sel.Value)
	}
	sel.cycle(1)
	if sel.Value != "right" {
		t.Fatalf("after next Value = %q, want right", sel.Value)
	}
	sel.cycle(-1)
	if sel.Value != "left" {
		t.Fatalf("after prev Value = %q, want left", sel.Value)
	}
	sel.cycle(-1)
	if sel.Value != "center" {
		t.Fatalf("wrap prev Value = %q, want center", sel.Value)
	}
}

func TestPillSelect_NormalizesUnknownValue(t *testing.T) {
	t.Helper()
	test.NewApp()
	sel := NewPillSelect("Op", []string{"==", "!="}, "missing", nil)
	if sel.Value != "==" {
		t.Fatalf("Value = %q, want ==", sel.Value)
	}
}

func TestPillSelect_FormatDisplay(t *testing.T) {
	t.Helper()
	test.NewApp()
	sel := NewPillSelect("Button", []string{"left"}, "left", func(v string) string {
		if v == "left" {
			return "Left"
		}
		return v
	})
	if sel.valueText.Text != "Left" {
		t.Fatalf("display = %q, want Left", sel.valueText.Text)
	}
}
