package serialize

import (
	"testing"

	"Sqyre/internal/models/actions"
)

func TestDecodeMacroFromMap_tags(t *testing.T) {
	data := map[string]any{
		"name": "tagged",
		"tags": []any{"combat", "daily"},
		"root": map[string]any{
			"type":  "loop",
			"name":  "root",
			"count": 1,
		},
	}
	m, err := DecodeMacroFromMap(data)
	if err != nil {
		t.Fatal(err)
	}
	if len(m.Tags) != 2 || m.Tags[0] != "combat" || m.Tags[1] != "daily" {
		t.Fatalf("tags = %v", m.Tags)
	}
}

func TestDecodeMacroFromMap_delays(t *testing.T) {
	data := map[string]any{
		"name":          "delayed",
		"globaldelay":   50,
		"keyboarddelay": 25,
		"mousedelay":    75,
		"root": map[string]any{
			"type":  "loop",
			"name":  "root",
			"count": 1,
		},
	}
	m, err := DecodeMacroFromMap(data)
	if err != nil {
		t.Fatal(err)
	}
	if m.GlobalDelay != 50 {
		t.Fatalf("globaldelay = %d", m.GlobalDelay)
	}
	if m.KeyboardDelay != 25 {
		t.Fatalf("keyboarddelay = %d", m.KeyboardDelay)
	}
	if m.MouseDelay != 75 {
		t.Fatalf("mousedelay = %d", m.MouseDelay)
	}
}

func TestDecodeMacroFromMap_minimal(t *testing.T) {
	data := map[string]any{
		"name":        "test-macro",
		"globaldelay": 50,
		"hotkey":      []any{"ctrl", "a"},
		"root": map[string]any{
			"type":  "loop",
			"name":  "root",
			"count": 1,
			"subactions": []any{
				map[string]any{"type": "wait", "time": 100},
			},
		},
	}
	m, err := DecodeMacroFromMap(data)
	if err != nil {
		t.Fatal(err)
	}
	if m.Name != "test-macro" {
		t.Fatalf("name = %q", m.Name)
	}
	if m.GlobalDelay != 50 {
		t.Fatalf("globaldelay = %d", m.GlobalDelay)
	}
	if len(m.Hotkey) != 2 {
		t.Fatalf("hotkey = %v", m.Hotkey)
	}
	if m.Root == nil || len(m.Root.GetSubActions()) != 1 {
		t.Fatalf("root subactions = %d", len(m.Root.GetSubActions()))
	}
	if m.Root.GetSubActions()[0].GetType() != "wait" {
		t.Fatalf("subaction type = %s", m.Root.GetSubActions()[0].GetType())
	}
}

func TestDecodeMacroFromMap_waitVariableTime(t *testing.T) {
	data := map[string]any{
		"name": "wait-var",
		"root": map[string]any{
			"type":  "loop",
			"name":  "root",
			"count": 1,
			"subactions": []any{
				map[string]any{"type": "wait", "time": "${delay}"},
			},
		},
	}
	m, err := DecodeMacroFromMap(data)
	if err != nil {
		t.Fatal(err)
	}
	wait, ok := m.Root.GetSubActions()[0].(*actions.Wait)
	if !ok {
		t.Fatalf("subaction type = %T", m.Root.GetSubActions()[0])
	}
	if time, ok := wait.Time.(string); !ok || time != "${delay}" {
		t.Fatalf("wait time = %v", wait.Time)
	}
}

func TestDecodeMacroFromMap_invalidRoot(t *testing.T) {
	_, err := DecodeMacroFromMap(map[string]any{
		"name": "bad",
		"root": "not-a-loop",
	})
	if err == nil {
		t.Fatal("expected error")
	}
}

func TestDecodeMacroFromMap_conditional(t *testing.T) {
	data := map[string]any{
		"name": "cond",
		"root": map[string]any{
			"type":  "loop",
			"name":  "root",
			"count": 1,
			"subactions": []any{
				map[string]any{
					"type": "conditional",
					"name": "if",
					"match": "all",
					"clauses": []any{
						map[string]any{
							"operator": "==",
							"left":     "1",
							"right":    "1",
						},
					},
					"subactions": []any{
						map[string]any{"type": "break"},
					},
				},
			},
		},
	}
	m, err := DecodeMacroFromMap(data)
	if err != nil {
		t.Fatal(err)
	}
	cond, ok := m.Root.GetSubActions()[0].(*actions.Conditional)
	if !ok {
		t.Fatalf("expected conditional, got %T", m.Root.GetSubActions()[0])
	}
	if len(cond.GetSubActions()) != 1 {
		t.Fatalf("conditional subactions = %d", len(cond.GetSubActions()))
	}
}
