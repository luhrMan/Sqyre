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

func TestVarNameEntry_insertButtonDisabledWithoutVariables(t *testing.T) {
	test.NewApp()
	e := NewVarNameEntryWithDefs(func() []models.VariableDef { return nil })
	if !e.insert.Disabled() {
		t.Fatal("insert button should be disabled when no variables exist")
	}
	e.GetVariableDefs = func() []models.VariableDef {
		return []models.VariableDef{{Name: "a"}}
	}
	e.InvalidateVariableCache()
	e.UpdateInsertButton()
	if e.insert.Disabled() {
		t.Fatal("insert button should enable when variables become available")
	}
}
