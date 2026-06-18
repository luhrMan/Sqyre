package models

import (
	"os"
	"strings"
	"testing"

	"Sqyre/internal/models/actions"

	"github.com/spf13/viper"
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

func TestVariableStore_NormalizeKeys(t *testing.T) {
	vs := NewVariableStore()
	vs.Variables["stackmax"] = 1
	vs.Variables["StackMax"] = 2
	vs.NormalizeKeys()
	if len(vs.Variables) != 1 {
		t.Fatalf("expected 1 key, got %v", vs.Variables)
	}
	for k, v := range vs.Variables {
		if k != "StackMax" || v != 2 {
			t.Fatalf("got %q=%v", k, v)
		}
	}
}

func TestCollectVariableDefs_noCaseInsensitiveDuplicates(t *testing.T) {
	m := NewMacro("test", 0, nil)
	m.Variables.Set("topy", "10")
	m.Variables.Set("bottomy", "20")
	m.Variables.Set("stackmax", "1")
	m.Root = actions.NewLoop(1, "root", []actions.ActionInterface{
		actions.NewImageSearch("search", nil, nil, actions.SearchArea{}, 1, 1, 0.95, 5),
	})
	m.Variables.Set("StackMax", 15)

	defs := CollectVariableDefs(m)
	for i, a := range defs {
		for j := i + 1; j < len(defs); j++ {
			if strings.EqualFold(a.Name, defs[j].Name) && a.Name != defs[j].Name {
				t.Fatalf("case-insensitive duplicates in defs: %v", defs)
			}
		}
	}
}

func TestVariableStore_viperRoundtripPreservesKeyCasing(t *testing.T) {
	m := NewMacro("test", 0, nil)
	m.SetInitialVariable("topY", "100")
	m.SetInitialVariable("StackMax", "5")
	m.SetInitialVariable("bottomY", "200")

	path := t.TempDir() + "/macro.yaml"
	v := viper.New()
	v.Set("macro", m)
	if err := v.WriteConfigAs(path); err != nil {
		t.Fatal(err)
	}
	data, err := os.ReadFile(path)
	if err != nil {
		t.Fatal(err)
	}
	t.Logf("YAML:\n%s", data)

	vs := VariableStoreFromYAMLBytes([]byte(`
variables:
  variables:
    StackMax: "5"
    bottomY: "200"
    topY: "100"
`))
	if vs == nil {
		t.Fatal("expected variables from yaml bytes")
	}
	for _, want := range []string{"topY", "StackMax", "bottomY"} {
		if _, ok := vs.Get(want); !ok {
			t.Errorf("missing key %q; keys=%v", want, vs.GetAll())
		}
	}
}
