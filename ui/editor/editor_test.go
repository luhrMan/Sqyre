package editor

import (
	"testing"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/container"
	"fyne.io/fyne/v2/test"
	"fyne.io/fyne/v2/widget"
)

func TestValidateCreateName(t *testing.T) {
	if err := validateCreateName(""); err == nil {
		t.Error("empty name should fail validation")
	}
	if err := validateCreateName("valid"); err != nil {
		t.Errorf("valid name should pass: %v", err)
	}
}

func TestValidateCreateProgramName(t *testing.T) {
	if err := validateCreateProgramName(""); err == nil {
		t.Error("empty program name should fail validation")
	}
	if err := validateCreateProgramName("prog"); err != nil {
		t.Errorf("valid program name should pass: %v", err)
	}
}

func TestEnsureNameAvailable(t *testing.T) {
	getter := func(name string) (any, error) {
		if name == "exists" {
			return "found", nil
		}
		return nil, &notFoundErr{}
	}
	if err := ensureNameAvailable("exists", "thing", getter); err == nil {
		t.Error("existing name should fail")
	}
	if err := ensureNameAvailable("new", "thing", getter); err != nil {
		t.Errorf("new name should pass: %v", err)
	}
}

type notFoundErr struct{}

func (e *notFoundErr) Error() string { return "not found" }

func TestAccordionRowIndexForProgram(t *testing.T) {
	test.NewApp()
	acc := &widget.Accordion{}
	acc.Append(widget.NewAccordionItem("Alpha (3)", nil))
	acc.Append(widget.NewAccordionItem("Beta (1)", nil))

	if idx := accordionRowIndexForProgram(acc, "Alpha"); idx != 0 {
		t.Errorf("Alpha index = %d, want 0", idx)
	}
	if idx := accordionRowIndexForProgram(acc, "Beta"); idx != 1 {
		t.Errorf("Beta index = %d, want 1", idx)
	}
	if idx := accordionRowIndexForProgram(acc, "Gamma"); idx != -1 {
		t.Errorf("Gamma index = %d, want -1", idx)
	}
}

func TestSortKeysByRepoDisplayName(t *testing.T) {
	keys := []string{"c", "a", "b"}
	sortKeysByRepoDisplayName(keys, func(k string) string { return k })
	if keys[0] != "a" || keys[1] != "b" || keys[2] != "c" {
		t.Errorf("sorted = %v", keys)
	}
}

func TestFilterKeysByFuzzy(t *testing.T) {
	keys := []string{"apple", "banana", "apricot", "cherry"}
	got := filterKeysByFuzzy("ap", keys)
	if len(got) != 2 {
		t.Errorf("expected 2 matches, got %d: %v", len(got), got)
	}
	all := filterKeysByFuzzy("", keys)
	if len(all) != 4 {
		t.Errorf("empty filter should return all, got %d", len(all))
	}
}

func TestSkipProgramAccordionRow(t *testing.T) {
	if skipProgramAccordionRow("", "prog", nil) {
		t.Error("empty filter should never skip")
	}
	if !skipProgramAccordionRow("xyz", "prog", nil) {
		t.Error("non-matching filter with no entities should skip")
	}
	if skipProgramAccordionRow("xyz", "prog", []string{"xyz-item"}) {
		t.Error("matching entity should not skip")
	}
	if skipProgramAccordionRow("pro", "prog", nil) {
		t.Error("matching program name should not skip")
	}
}

func TestNewEditorTab(t *testing.T) {
	test.NewApp()
	tab := NewEditorTab("TestTab", nil, nil)
	if tab == nil {
		t.Fatal("NewEditorTab returned nil")
	}
	if tab.Text != "TestTab" {
		t.Errorf("tab text = %q, want TestTab", tab.Text)
	}
}

func TestLabeledProgramSelector(t *testing.T) {
	test.NewApp()
	sel := widget.NewSelect([]string{"a", "b"}, nil)
	c := LabeledProgramSelector(sel)
	if c == nil {
		t.Fatal("LabeledProgramSelector returned nil")
	}
}

func TestEditorUi_ActiveProgramName_WithTabs(t *testing.T) {
	test.NewApp()
	eu := &EditorUi{}
	sel := widget.NewSelect([]string{"prog1"}, nil)
	sel.SetSelected("prog1")
	tab := &EditorTab{
		TabItem:         container.NewTabItem("Programs", widget.NewLabel("")),
		ProgramSelector: sel,
		Widgets:         make(map[string]fyne.CanvasObject),
	}
	eu.EditorTabs.AppTabs = container.NewAppTabs(tab.TabItem)
	eu.EditorTabs.ProgramsTab = tab
	if got := eu.ActiveProgramName(); got != "prog1" {
		t.Errorf("expected prog1, got %q", got)
	}
}
