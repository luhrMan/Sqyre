package macro

import (
	"Sqyre/internal/models"
	"Sqyre/internal/models/actions"
	"testing"

	"fyne.io/fyne/v2/test"
)

func newTestMacro() *models.Macro {
	root := actions.NewLoop(1, "root", nil)
	root.AddSubAction(actions.NewWait(100))
	root.AddSubAction(actions.NewClick(false, true))
	return &models.Macro{Name: "test", Root: root}
}

func TestNewMacroTree(t *testing.T) {
	test.NewApp()
	m := newTestMacro()
	tree := NewMacroTree(m)
	if tree == nil {
		t.Fatal("NewMacroTree returned nil")
	}
	if tree.Macro != m {
		t.Error("tree.Macro should be the macro we passed")
	}
}

func TestMacroTree_ChildUIDs(t *testing.T) {
	test.NewApp()
	m := newTestMacro()
	tree := NewMacroTree(m)
	children := tree.ChildUIDs("")
	if len(children) != 2 {
		t.Errorf("root should have 2 children, got %d", len(children))
	}
}

func TestMacroTree_IsBranch(t *testing.T) {
	test.NewApp()
	m := newTestMacro()
	tree := NewMacroTree(m)
	if !tree.IsBranch("") {
		t.Error("root should be a branch")
	}
	// children of root are Wait and Click — neither are branches
	children := tree.ChildUIDs("")
	for _, uid := range children {
		if tree.IsBranch(uid) {
			t.Errorf("child %s should not be a branch", uid)
		}
	}
}

func TestMacroTree_PasteNode_NilClipboard(t *testing.T) {
	test.NewApp()
	m := newTestMacro()
	tree := NewMacroTree(m)
	if tree.PasteNode(nil) {
		t.Error("PasteNode(nil) should return false")
	}
}

func TestNewMacroTabs(t *testing.T) {
	test.NewApp()
	tabs := NewMacroTabs()
	if tabs == nil {
		t.Fatal("NewMacroTabs returned nil")
	}
	if tabs.MacroNameEntry == nil {
		t.Error("MacroNameEntry should not be nil")
	}
	if tabs.HotkeyTriggerRadio == nil {
		t.Error("HotkeyTriggerRadio should not be nil")
	}
	if tabs.HotkeyTriggerRadio.Selected != "On press" {
		t.Errorf("default hotkey trigger = %q, want 'On press'", tabs.HotkeyTriggerRadio.Selected)
	}
}

func TestMacroTabs_SelectedTab_NoTabs(t *testing.T) {
	test.NewApp()
	tabs := NewMacroTabs()
	if tabs.SelectedTab() != nil {
		t.Error("SelectedTab() with no tabs should be nil")
	}
}
