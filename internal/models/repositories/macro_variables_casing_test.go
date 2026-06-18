package repositories

import (
	"testing"

	"Sqyre/internal/models/serialize"

	"gopkg.in/yaml.v3"
)

func TestDecodeMacro_variableKeyCasingFromYAML(t *testing.T) {
	setupTestConfig(t)
	yamlConfig := serialize.GetYAMLConfig()
	prevPath := yamlConfig.GetConfigFile()
	defer yamlConfig.SetConfigFile(prevPath)

	prevData := yamlConfig.Get("macros")
	defer func() {
		yamlConfig.Set("macros", prevData)
	}()

	yamlConfig.Set("macros", map[string]any{
		"casing-test": map[string]any{
			"name":        "casing-test",
			"globaldelay": 0,
			"hotkey":      []any{},
			"root": map[string]any{
				"type":  "loop",
				"name":  "root",
				"count": 1,
				"subactions": []any{},
			},
			"variables": map[string]any{
				"variables": map[string]any{
					"topY":     "10",
					"bottomY":  "20",
					"StackMax": "5",
				},
			},
		},
	})

	macro, err := decodeMacro("casing-test")
	if err != nil {
		t.Fatal(err)
	}
	for _, want := range []string{"topY", "bottomY", "StackMax"} {
		if _, ok := macro.Variables.Get(want); !ok {
			t.Errorf("missing %q after decode; keys=%v", want, macro.Variables.GetAll())
		}
	}
}

func TestGetStringMap_lowercasesNestedVariableKeys(t *testing.T) {
	setupTestConfig(t)
	yamlConfig := serialize.GetYAMLConfig()
	prevPath := yamlConfig.GetConfigFile()
	defer yamlConfig.SetConfigFile(prevPath)

	prevData := yamlConfig.Get("macros")
	defer func() {
		yamlConfig.Set("macros", prevData)
	}()

	yamlConfig.Set("macros", map[string]any{
		"casing-test": map[string]any{
			"variables": map[string]any{
				"variables": map[string]any{
					"topY":     "10",
					"StackMax": "5",
				},
			},
		},
	})

	macrosMap := yamlConfig.GetStringMap("macros")
	macroData := macrosMap["casing-test"]
	yamlBytes, err := yaml.Marshal(macroData)
	if err != nil {
		t.Fatal(err)
	}
	t.Logf("marshaled from GetStringMap:\n%s", yamlBytes)

	var raw map[string]any
	if err := yaml.Unmarshal(yamlBytes, &raw); err != nil {
		t.Fatal(err)
	}
	outer := raw["variables"].(map[string]any)
	inner := outer["variables"].(map[string]any)
	t.Logf("keys after yaml roundtrip from GetStringMap: %v", inner)
}

func TestYAMLUnmarshal_preservesVariableKeyCasing(t *testing.T) {
	const doc = `
variables:
  variables:
    topY: "10"
    StackMax: "5"
`
	var raw map[string]any
	if err := yaml.Unmarshal([]byte(doc), &raw); err != nil {
		t.Fatal(err)
	}
	outer := raw["variables"].(map[string]any)
	inner := outer["variables"].(map[string]any)
	if inner["topY"] != "10" {
		t.Fatalf("topY casing lost: %v", inner)
	}
	if inner["StackMax"] != "5" {
		t.Fatalf("StackMax casing lost: %v", inner)
	}
}
