package actiondialog

import (
	"testing"

	"fyne.io/fyne/v2/test"
	"fyne.io/fyne/v2/widget"
)

func TestIsNameFieldLabel(t *testing.T) {
	if !isNameFieldLabel("Name:") {
		t.Error("Name: should be a name field")
	}
	if !isNameFieldLabel("Variable Name:") {
		t.Error("Variable Name: should be a name field")
	}
	if isNameFieldLabel("Other:") {
		t.Error("Other: should not be a name field")
	}
}

func TestApplyFieldTooltip_EmptyHint(t *testing.T) {
	test.NewApp()
	w := widget.NewLabel("test")
	got := applyFieldTooltip("Label:", w, "")
	if got != w {
		t.Error("empty hint should return original widget")
	}
}

func TestApplyFieldTooltip_NameFieldSkipsIcon(t *testing.T) {
	test.NewApp()
	w := widget.NewEntry()
	got := applyFieldTooltip("Name:", w, "some hint")
	if got != w {
		t.Error("name field should return original widget even with hint")
	}
}

func TestFormHint_ReturnsFormItem(t *testing.T) {
	test.NewApp()
	w := widget.NewEntry()
	item := formHint("Label", w, "hint text")
	if item == nil {
		t.Fatal("formHint returned nil")
	}
	if item.Text != "Label" {
		t.Errorf("label = %q, want Label", item.Text)
	}
}

func TestMacroVarNames_NilFunc(t *testing.T) {
	old := active
	defer func() { active = old }()
	active.MacroVariables = nil
	if got := macroVarNames(); got != nil {
		t.Errorf("expected nil, got %v", got)
	}
}

func TestMacroVarNames_WithFunc(t *testing.T) {
	old := active
	defer func() { active = old }()
	active.MacroVariables = func() []string { return []string{"x", "y"} }
	got := macroVarNames()
	if len(got) != 2 || got[0] != "x" || got[1] != "y" {
		t.Errorf("got %v", got)
	}
}

func TestCurrentMacroName_NilFunc(t *testing.T) {
	old := active
	defer func() { active = old }()
	active.CurrentMacroName = nil
	if got := currentMacroName(); got != "" {
		t.Errorf("expected empty, got %q", got)
	}
}

func TestCurrentMacroName_WithFunc(t *testing.T) {
	old := active
	defer func() { active = old }()
	active.CurrentMacroName = func() string { return "myMacro" }
	if got := currentMacroName(); got != "myMacro" {
		t.Errorf("expected myMacro, got %q", got)
	}
}
