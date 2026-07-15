package models

import (
	"testing"

	"Sqyre/internal/models/actions"
)

func TestCollectDefinedVariableNames_fromActions(t *testing.T) {
	m := NewMacro("t", 0, nil)
	m.Root = actions.NewLoop(1, "root", []actions.ActionInterface{
		actions.NewSetVariable("counter", 0),
		actions.NewSetVariable("result", "counter + 1"),
		actions.NewImageSearch("search", nil, nil, "", 0.95, 5),
	})

	names := CollectDefinedVariableNames(m)
	required := []string{
		"counter", "result", "foundX", "foundY",
		"StackMax", "Cols", "Rows", "ItemName", "ImagePixelWidth", "ImagePixelHeight",
		"monitor1Width", "monitor1Height",
	}
	got := map[string]bool{}
	for _, n := range names {
		got[n] = true
	}
	for _, name := range required {
		if !got[name] {
			t.Fatalf("missing variable %q in %v", name, names)
		}
	}
}

func TestCollectDefinedVariableNames_findPixelOutputs(t *testing.T) {
	m := NewMacro("t", 0, nil)
	fp := actions.NewFindPixel("px", "", "ffffff", 0)
	fp.OutputXVariable = "pxX"
	fp.OutputYVariable = "pxY"
	m.Root = actions.NewLoop(1, "root", []actions.ActionInterface{fp})

	names := CollectDefinedVariableNames(m)
	got := map[string]bool{}
	for _, n := range names {
		got[n] = true
	}
	if !got["pxX"] || !got["pxY"] {
		t.Fatalf("missing find pixel outputs in %v", names)
	}
}

func TestCollectVariableDefs_initialValue(t *testing.T) {
	m := NewMacro("t", 0, nil)
	m.UpsertVariable(VariableDecl{Name: "seed", InitialValue: "42"})
	defs := CollectVariableDefs(m)
	found := false
	for _, d := range defs {
		if d.Name == "seed" && d.InitialValue == "42" {
			found = true
		}
	}
	if !found {
		t.Fatalf("defs = %+v, want seed with initial 42", defs)
	}
}
