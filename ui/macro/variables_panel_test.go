package macro

import (
	"testing"

	"fyne.io/fyne/v2/test"
	"fyne.io/fyne/v2/widget"
)

func TestVariableListRowFrom_findsButtonsAfterHBoxGrew(t *testing.T) {
	test.NewApp()
	root, row := newVariableListRow()
	got, ok := variableListRowFrom(root)
	if !ok {
		t.Fatal("variableListRowFrom returned false")
	}
	if got.iconBg == nil || got.iconBtn == nil || got.pillsBox == nil {
		t.Fatal("expected icon and pills widgets")
	}
	if got.detailsBtn == nil {
		t.Fatal("detailsBtn not found")
	}
	if got.editBtn == nil {
		t.Fatal("editBtn not found")
	}
	if got.removeBtn == nil {
		t.Fatal("removeBtn not found")
	}
	if row.detailsBtn != got.detailsBtn || row.editBtn != got.editBtn || row.removeBtn != got.removeBtn {
		t.Fatal("parsed buttons do not match created row")
	}
	if got.removeBtn.Importance != widget.DangerImportance {
		t.Fatalf("removeBtn importance = %v, want DangerImportance", got.removeBtn.Importance)
	}
}
