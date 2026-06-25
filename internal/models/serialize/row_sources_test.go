package serialize

import (
	"testing"

	"Sqyre/internal/models/actions"
)

func TestActionToMap_forEachRow_roundTrip(t *testing.T) {
	orig := actions.NewForEachRow("rows", []actions.ListColumn{
		{Source: "a\nb", OutputVar: "x", IsFile: false},
		{Source: "c.txt", OutputVar: "y", IsFile: true, SkipBlankLines: true},
	}, []actions.ActionInterface{actions.NewWait(1)})

	m, err := ActionToMap(orig)
	if err != nil {
		t.Fatal(err)
	}
	if m["type"] != "foreachrow" {
		t.Fatalf("type = %v", m["type"])
	}
	back, err := ViperSerializer.CreateActionFromMap(m, nil)
	if err != nil {
		t.Fatal(err)
	}
	fer, ok := back.(*actions.ForEachRow)
	if !ok {
		t.Fatalf("got %T", back)
	}
	if fer.Name != "rows" || len(fer.Sources) != 2 {
		t.Fatalf("fer = %+v", fer)
	}
	if len(fer.GetSubActions()) != 1 {
		t.Fatalf("subactions = %d", len(fer.GetSubActions()))
	}
}

func TestActionToMap_forEachRow_rowRangeRoundTrip(t *testing.T) {
	orig := actions.NewForEachRow("rows", []actions.ListColumn{
		{Source: "a\nb", OutputVar: "x"},
	}, []actions.ActionInterface{actions.NewWait(1)})
	orig.StartRow = 2
	orig.EndRow = "${last}"

	m, err := ActionToMap(orig)
	if err != nil {
		t.Fatal(err)
	}
	if m["startrow"] != 2 {
		t.Fatalf("startrow = %v", m["startrow"])
	}
	if m["endrow"] != "${last}" {
		t.Fatalf("endrow = %v", m["endrow"])
	}

	back, err := ViperSerializer.CreateActionFromMap(m, nil)
	if err != nil {
		t.Fatal(err)
	}
	fer := back.(*actions.ForEachRow)
	if fer.StartRow != 2 {
		t.Fatalf("StartRow = %v, want 2", fer.StartRow)
	}
	if fer.EndRow != "${last}" {
		t.Fatalf("EndRow = %v, want ${last}", fer.EndRow)
	}
}

func TestActionToMap_forEachRow_unsetRangeOmitted(t *testing.T) {
	orig := actions.NewForEachRow("rows", []actions.ListColumn{
		{Source: "a", OutputVar: "x"},
	}, nil)

	m, err := ActionToMap(orig)
	if err != nil {
		t.Fatal(err)
	}
	if _, ok := m["startrow"]; ok {
		t.Fatalf("startrow should be omitted when unset")
	}
	if _, ok := m["endrow"]; ok {
		t.Fatalf("endrow should be omitted when unset")
	}
}

func TestCreateActionFromMap_forEachRow_missingSourcesDefaults(t *testing.T) {
	back, err := ViperSerializer.CreateActionFromMap(map[string]any{
		"type": "foreachrow",
		"name": "rows",
	}, nil)
	if err != nil {
		t.Fatal(err)
	}
	fer, ok := back.(*actions.ForEachRow)
	if !ok {
		t.Fatalf("got %T", back)
	}
	if fer.Name != "rows" {
		t.Fatalf("name = %q", fer.Name)
	}
	if len(fer.Sources) != 0 {
		t.Fatalf("sources = %+v, want empty", fer.Sources)
	}
}

func TestCreateActionFromMap_forEachRow_malformedSourcesDefaults(t *testing.T) {
	back, err := ViperSerializer.CreateActionFromMap(map[string]any{
		"type":    "foreachrow",
		"name":    "rows",
		"sources": "not-a-list",
	}, nil)
	if err != nil {
		t.Fatal(err)
	}
	fer := back.(*actions.ForEachRow)
	if len(fer.Sources) != 0 {
		t.Fatalf("sources = %+v, want empty", fer.Sources)
	}
}

func TestCreateActionFromMap_forEachRow_partialSourceEntryUsesDefaults(t *testing.T) {
	back, err := ViperSerializer.CreateActionFromMap(map[string]any{
		"type": "foreachrow",
		"name": "rows",
		"sources": []any{
			map[string]any{"source": "a\nb"},
			"bad-entry",
		},
	}, nil)
	if err != nil {
		t.Fatal(err)
	}
	fer := back.(*actions.ForEachRow)
	if len(fer.Sources) != 1 {
		t.Fatalf("sources = %+v", fer.Sources)
	}
	if fer.Sources[0].Source != "a\nb" || fer.Sources[0].OutputVar != "" {
		t.Fatalf("source column = %+v", fer.Sources[0])
	}
}
