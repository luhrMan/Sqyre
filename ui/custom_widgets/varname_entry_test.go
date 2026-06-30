package custom_widgets

import (
	"Sqyre/internal/models"
	"testing"

	"fyne.io/fyne/v2/test"
)

func TestVarNameEntry_pickVariable(t *testing.T) {
	test.NewApp()
	defs := []models.VariableDef{
		{Name: "count", Type: models.VariableTypeNumber, Source: models.VariableSource{ActionName: "Set Variable"}},
		{Name: "result"},
	}
	e := NewVarNameEntryWithDefs(func() []models.VariableDef { return defs })
	e.pickVariable("count")
	if e.Text != "count" {
		t.Fatalf("Text = %q, want count", e.Text)
	}
}

