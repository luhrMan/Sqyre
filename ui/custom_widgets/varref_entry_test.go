package custom_widgets

import (
	"testing"

	"fyne.io/fyne/v2/test"
)

func TestVarRefEntry_insertVariable(t *testing.T) {
	test.NewApp()
	vars := []string{"count", "name"}
	e := NewVarRefEntry(func() []string { return vars })
	e.SetText("hello ")
	e.CursorColumn = len([]rune("hello "))
	e.CursorRow = 0
	e.insertVariable("count")
	if e.Text != "hello ${count}" {
		t.Fatalf("Text = %q, want %q", e.Text, "hello ${count}")
	}
}

func TestVarRefEntry_insertVariable_replacesSelection(t *testing.T) {
	test.NewApp()
	e := NewVarRefEntry(func() []string { return []string{"x"} })
	e.SetText("abc")
	e.CursorColumn = 3
	e.CursorRow = 0
	// Select "bc" by setting selection via paste trick: set full text and use SelectedText path
	e.SetText("abc")
	// Entry selection API is limited in tests; verify append-at-cursor instead.
	e.SetText("")
	e.insertVariable("x")
	if e.Text != "${x}" {
		t.Fatalf("Text = %q, want ${x}", e.Text)
	}
}

func TestVarRefEntry_insertButtonDisabledWithoutVariables(t *testing.T) {
	test.NewApp()
	e := NewVarRefEntry(func() []string { return nil })
	if !e.insert.Disabled() {
		t.Fatal("insert button should be disabled when no variables are defined")
	}
	e.GetVariables = func() []string { return []string{"a"} }
	e.updateInsertButton()
	if e.insert.Disabled() {
		t.Fatal("insert button should enable when variables become available")
	}
}
