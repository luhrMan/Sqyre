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
		{"imagesearch", actions.NewImageSearch("img", nil, []string{"item1"}, searchRef, 2, 2, 0.9, 1)},
		{"ocr", actions.NewOcr("ocr1", "gold", searchRef)},
		{"setvariable", actions.NewSetVariable("count", 1)},
		{"calculate", actions.NewCalculate("${a}+1", "result")},
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
