package custom_widgets

import (
	"testing"

	"Sqyre/internal/models"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/test"
)

func TestBuildVariableNamePillContent(t *testing.T) {
	t.Helper()
	known := map[string]bool{"count": true}
	content := BuildVariableNamePillContent("count", known)
	if content.MinSize().Height > PillLineHeight()+2 {
		t.Fatalf("pill height %v exceeds line height %v", content.MinSize().Height, PillLineHeight())
	}
	unknown := BuildVariableNamePillContent("missing", known)
	if unknown.MinSize().Width <= 0 {
		t.Fatal("expected non-zero width for unknown variable pill")
	}
}

func TestVarNameEntry_showsPillOverlayWhenUnfocused(t *testing.T) {
	t.Helper()
	test.NewApp()

	e := NewVarNameEntryWithDefs(func() []models.VariableDef {
		return []models.VariableDef{{Name: "count"}}
	})
	e.SetText("count")
	e.hasFocus = false
	e.syncPillDisplay()
	if !e.hideTextForPills {
		t.Fatal("expected pill overlay when unfocused with variable name")
	}
}

func TestBorderlessVarNameEntry_pillOverlayTapFocuses(t *testing.T) {
	t.Helper()
	test.NewApp()
	w := test.NewWindow(nil)
	defer w.Close()

	e := NewBorderlessVarNameEntry(func() []models.VariableDef {
		return []models.VariableDef{{Name: "count"}}
	})
	e.SetText("count")
	e.hasFocus = false
	w.SetContent(e)
	e.Refresh()

	host := e.overlay.object(&e.VarNameEntry)
	if tap, ok := host.(interface{ Tapped(*fyne.PointEvent) }); ok {
		tap.Tapped(&fyne.PointEvent{})
	} else {
		t.Fatal("expected pill overlay host to handle taps")
	}
}
