package models

import (
	"strings"
	"testing"

	"Sqyre/internal/models/actions"

	"gopkg.in/yaml.v3"
)

func TestVariableStore_SetMergesCaseInsensitiveKeys(t *testing.T) {
	vs := NewVariableStore()
	vs.Set("topy", "10")
	vs.Set("topY", 20)
	if _, ok := vs.Variables["topy"]; ok {
		t.Fatal("expected lowercase key to be replaced")
	}
	if v, ok := vs.Get("topY"); !ok || v != 20 {
		t.Fatalf("Get(topY) = %v, %v", v, ok)
	}
	if v, ok := vs.Get("topy"); !ok || v != 20 {
		t.Fatalf("Get(topy) = %v, %v", v, ok)
	}
}

func TestCollectVariableDefs_noCaseInsensitiveDuplicates(t *testing.T) {
	m := NewMacro("test", 0, nil)
	m.UpsertVariable(VariableDecl{Name: "topY", InitialValue: "10"})
	m.UpsertVariable(VariableDecl{Name: "bottomY", InitialValue: "20"})
	// Declared with different casing than the ImageSearch builtin "StackMax".
	m.UpsertVariable(VariableDecl{Name: "stackmax", InitialValue: "1"})
	m.Root = actions.NewLoop(1, "root", []actions.ActionInterface{
		actions.NewImageSearch("search", nil, nil, "", 1, 1, 0.95, 5),
	})

	defs := CollectVariableDefs(m)
	for i, a := range defs {
		for j := i + 1; j < len(defs); j++ {
			if strings.EqualFold(a.Name, defs[j].Name) && a.Name != defs[j].Name {
				t.Fatalf("case-insensitive duplicates in defs: %v", defs)
			}
		}
	}
}

func TestVariableDecls_yamlRoundtripPreservesNameCasing(t *testing.T) {
	m := NewMacro("test", 0, nil)
	m.UpsertVariable(VariableDecl{Name: "topY", InitialValue: "100"})
	m.UpsertVariable(VariableDecl{Name: "StackMax", Type: VariableTypeNumber, InitialValue: "5"})
	m.UpsertVariable(VariableDecl{Name: "bottomY", InitialValue: "200"})

	data, err := yaml.Marshal(m)
	if err != nil {
		t.Fatal(err)
	}
	t.Logf("YAML:\n%s", data)

	var got Macro
	if err := yaml.Unmarshal(data, &got); err != nil {
		t.Fatal(err)
	}
	gotNames := map[string]bool{}
	for _, d := range got.VariableDecls {
		gotNames[d.Name] = true
	}
	for _, want := range []string{"topY", "StackMax", "bottomY"} {
		if !gotNames[want] {
			t.Errorf("missing decl %q; got=%v", want, got.VariableDecls)
		}
	}
}
