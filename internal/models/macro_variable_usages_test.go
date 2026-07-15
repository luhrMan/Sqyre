package models

import (
	"testing"

	"Sqyre/internal/models/actions"
)

func TestCollectVariableUsages_declReferencesAndOutputs(t *testing.T) {
	m := NewMacro("t", 0, nil)
	m.UpsertVariable(VariableDecl{Name: "count", Type: VariableTypeNumber, InitialValue: "1"})

	setCount := actions.NewSetVariable("count", "${count} + 1")
	setv := actions.NewSetVariable("other", "${count}")
	save := actions.NewSaveVariable("count", "/tmp/out.txt", false, false)
	m.Root = actions.NewLoop(1, "root", []actions.ActionInterface{setCount, setv, save})

	usages := CollectVariableUsages(m, "count")
	if len(usages) < 5 {
		t.Fatalf("usages = %+v, want at least 5 entries", usages)
	}

	kinds := map[VariableUsageKind]int{}
	for _, u := range usages {
		kinds[u.Kind]++
	}
	if kinds[VariableUsageInitial] != 1 {
		t.Fatalf("initial usages = %d, want 1", kinds[VariableUsageInitial])
	}
	if kinds[VariableUsageDefined] != 1 {
		t.Fatalf("defined usages = %d, want 1 (set output)", kinds[VariableUsageDefined])
	}
	if kinds[VariableUsageReferenced] < 2 {
		t.Fatalf("referenced usages = %d, want at least 2", kinds[VariableUsageReferenced])
	}
	if kinds[VariableUsageRead] != 1 {
		t.Fatalf("read usages = %d, want 1", kinds[VariableUsageRead])
	}
}

func TestCollectVariableUsages_caseInsensitiveReference(t *testing.T) {
	m := NewMacro("t", 0, nil)
	typ := actions.NewType("value is {Count}", 0)
	m.Root = actions.NewLoop(1, "root", []actions.ActionInterface{typ})

	usages := CollectVariableUsages(m, "count")
	foundRef := false
	for _, u := range usages {
		if u.Kind == VariableUsageReferenced {
			foundRef = true
		}
	}
	if !foundRef {
		t.Fatalf("usages = %+v, want a referenced usage for Count", usages)
	}
}

func TestCollectVariableUsages_nestedSubActions(t *testing.T) {
	m := NewMacro("t", 0, nil)
	loop := actions.NewLoop("${loops}", "inner", []actions.ActionInterface{
		actions.NewWait("${delay}"),
	})
	m.Root = actions.NewLoop(1, "root", []actions.ActionInterface{loop})

	usages := CollectVariableUsages(m, "delay")
	if len(usages) != 1 || usages[0].Kind != VariableUsageReferenced {
		t.Fatalf("usages = %+v, want one referenced usage in nested loop", usages)
	}
}
