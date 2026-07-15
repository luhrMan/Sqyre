package models

import (
	"testing"

	"Sqyre/internal/models/actions"
)

func TestRenameVariable_declAndReferences(t *testing.T) {
	m := NewMacro("t", 0, nil)
	m.UpsertVariable(VariableDecl{Name: "count", Type: VariableTypeNumber, InitialValue: "1", Description: "loop counter"})

	expr := actions.NewSetVariable("count", "${count} + 1")
	setv := actions.NewSetVariable("other", "${count}")
	m.Root = actions.NewLoop(1, "root", []actions.ActionInterface{expr, setv})

	if err := m.RenameVariable("count", "total"); err != nil {
		t.Fatalf("RenameVariable: %v", err)
	}

	// Declaration renamed, metadata preserved.
	d, ok := m.FindVariableDecl("total")
	if !ok {
		t.Fatalf("decl 'total' not found: %+v", m.VariableDecls)
	}
	if d.Type != VariableTypeNumber || d.InitialValue != "1" || d.Description != "loop counter" {
		t.Fatalf("decl metadata not preserved: %+v", d)
	}
	if _, ok := m.FindVariableDecl("count"); ok {
		t.Fatalf("old decl 'count' still present")
	}

	// Output binding renamed.
	if expr.VariableName != "total" {
		t.Fatalf("expr.VariableName = %q, want total", expr.VariableName)
	}
	// References rewritten.
	if v, _ := expr.Value.(string); v != "${total} + 1" {
		t.Fatalf("expr.Value = %v, want %q", expr.Value, "${total} + 1")
	}
	if v, _ := setv.Value.(string); v != "${total}" {
		t.Fatalf("setv.Value = %v, want ${total}", setv.Value)
	}
}

func TestRenameVariable_caseInsensitiveMatch(t *testing.T) {
	m := NewMacro("t", 0, nil)
	expr := actions.NewSetVariable("out", "${Count} * 2")
	m.Root = actions.NewLoop(1, "root", []actions.ActionInterface{expr})

	if err := m.RenameVariable("count", "tally"); err != nil {
		t.Fatalf("RenameVariable: %v", err)
	}
	if v, _ := expr.Value.(string); v != "${tally} * 2" {
		t.Fatalf("expr.Value = %v, want %q", expr.Value, "${tally} * 2")
	}
}

func TestRenameVariable_collisionRejected(t *testing.T) {
	m := NewMacro("t", 0, nil)
	m.UpsertVariable(VariableDecl{Name: "a", InitialValue: "1"})
	m.UpsertVariable(VariableDecl{Name: "b", InitialValue: "2"})
	if err := m.RenameVariable("a", "b"); err == nil {
		t.Fatal("expected collision error renaming a -> b")
	}
}

func TestRenameVariable_bracesPreserveStyle(t *testing.T) {
	m := NewMacro("t", 0, nil)
	typ := actions.NewType("value is {count} and ${count}", 0)
	m.Root = actions.NewLoop(1, "root", []actions.ActionInterface{typ})
	if err := m.RenameVariable("count", "n"); err != nil {
		t.Fatalf("RenameVariable: %v", err)
	}
	if typ.Text != "value is {n} and ${n}" {
		t.Fatalf("typ.Text = %q, want %q", typ.Text, "value is {n} and ${n}")
	}
}
