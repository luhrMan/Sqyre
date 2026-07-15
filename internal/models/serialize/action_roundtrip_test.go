package serialize

import (
	"testing"

	"Sqyre/internal/models/actions"
)

func TestActionCodec_roundTrip(t *testing.T) {
	searchRef := actions.CoordinateRef("prog:search")
	pointRef := actions.CoordinateRef("prog:point")

	tests := []struct {
		name   string
		action actions.ActionInterface
	}{
		{"loop", actions.NewLoop(3, "main", []actions.ActionInterface{actions.NewWait(10)})},
		{"wait", actions.NewWait(100)},
		{"pause", actions.NewPause("hold", []string{"f9"}, false)},
		{"findpixel", actions.NewFindPixel("fp", searchRef, "#ff0000", 10)},
		{"click", actions.NewClick("left", true)},
		{"move", actions.NewMove(pointRef, true)},
		{"key", actions.NewKey("enter", true)},
		{"type", actions.NewType("hello", 50)},
		{"imagesearch", actions.NewImageSearch("img", nil, []string{"item1"}, searchRef, 0.9, 1)},
		{"ocr", actions.NewOcr("ocr1", "gold", searchRef)},
		{"setvariable", actions.NewSetVariable("count", 1)},
		{"conditional", actions.NewConditional([]actions.ConditionClause{
			{Left: "${x}", Operator: actions.OpEquals, Right: 1},
		}, actions.MatchAll, "if", []actions.ActionInterface{actions.NewBreak()})},
		{"foreachrow", actions.NewForEachRow("rows", []actions.ListColumn{
			{Source: "list.txt", OutputVar: "col"},
		}, []actions.ActionInterface{actions.NewContinue()})},
		{"savevariable", actions.NewSaveVariable("out", "vars/out.txt", false, false)},
		{"focuswindow", actions.NewFocusWindow("/usr/bin/app", "Title")},
		{"runmacro", actions.NewRunMacro("helper")},
		{"break", actions.NewBreak()},
		{"continue", actions.NewContinue()},
	}

	for _, tc := range tests {
		t.Run(tc.name, func(t *testing.T) {
			m, err := ActionToMap(tc.action)
			if err != nil {
				t.Fatalf("ActionToMap: %v", err)
			}
			if m["type"] != tc.action.GetType() {
				t.Fatalf("type = %v, want %s", m["type"], tc.action.GetType())
			}
			back, err := ViperSerializer.CreateActionFromMap(m, nil)
			if err != nil {
				t.Fatalf("CreateActionFromMap: %v", err)
			}
			if back.GetType() != tc.action.GetType() {
				t.Fatalf("round-trip type = %s, want %s", back.GetType(), tc.action.GetType())
			}
		})
	}
}

func TestCreateActionFromMap_legacyCalculateDecodesAsSet(t *testing.T) {
	back, err := ViperSerializer.CreateActionFromMap(map[string]any{
		"type":       "calculate",
		"expression": "${a}+1",
		"outputvar":  "result",
	}, nil)
	if err != nil {
		t.Fatalf("CreateActionFromMap: %v", err)
	}
	sv, ok := back.(*actions.SetVariable)
	if !ok {
		t.Fatalf("got %T, want *SetVariable", back)
	}
	if sv.VariableName != "result" {
		t.Fatalf("VariableName = %q, want result", sv.VariableName)
	}
	if v, _ := sv.Value.(string); v != "${a}+1" {
		t.Fatalf("Value = %v, want ${a}+1", sv.Value)
	}
}

func TestCreateActionFromMap_legacyImageSearchSplitsIgnored(t *testing.T) {
	back, err := ViperSerializer.CreateActionFromMap(map[string]any{
		"type":      "imagesearch",
		"name":      "img",
		"targets":   []any{"item1"},
		"searcharea": "prog:search",
		"rowsplit":  2,
		"colsplit":  3,
		"tolerance": 0.9,
		"blur":      1,
	}, nil)
	if err != nil {
		t.Fatalf("CreateActionFromMap: %v", err)
	}
	is, ok := back.(*actions.ImageSearch)
	if !ok {
		t.Fatalf("got %T, want *ImageSearch", back)
	}
	m, err := ActionToMap(is)
	if err != nil {
		t.Fatalf("ActionToMap: %v", err)
	}
	if _, ok := m["rowsplit"]; ok {
		t.Fatal("rowsplit should not be encoded")
	}
	if _, ok := m["colsplit"]; ok {
		t.Fatal("colsplit should not be encoded")
	}
}
