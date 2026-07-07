package macro

import (
	"testing"

	"Sqyre/internal/models/actions"
)

func TestViewParamPills_AllActionTypes(t *testing.T) {
	t.Helper()
	cases := []struct {
		name   string
		node   actions.ActionInterface
	}{
		{"click", actions.NewClick(actions.ClickButtonLeft, false)},
		{"key", actions.NewKey("a", true)},
		{"wait", actions.NewWait(100)},
		{"move", actions.NewMove(actions.CoordinateRef(""), false)},
		{"loop", actions.NewLoop(3, "inner", nil)},
		{"conditional", actions.NewConditional(nil, actions.MatchAll, "c", nil)},
		{"setvariable", actions.NewSetVariable("x", 1)},
		{"calculate", actions.NewCalculate("1+1", "out")},
		{"runmacro", actions.NewRunMacro("other")},
		{"break", actions.NewBreak()},
		{"continue", actions.NewContinue()},
		{"type", actions.NewType("hello", 0)},
		{"pause", actions.NewPause("wait", nil, false)},
		{"savevariable", actions.NewSaveVariable("v", "dest", false, false)},
		{"focuswindow", actions.NewFocusWindow("title", "path")},
		{"foreachrow", actions.NewForEachRow("rows", nil, nil)},
		{"imagesearch", actions.NewImageSearch("s", nil, nil, actions.CoordinateRef(""), 1, 1, 0.95, 0)},
		{"findpixel", actions.NewFindPixel("f", actions.CoordinateRef(""), "ffffff", 0)},
		{"ocr", actions.NewOcr("o", "target", actions.CoordinateRef(""))},
	}
	for _, tc := range cases {
		t.Run(tc.name, func(t *testing.T) {
			if viewParamPills(tc.node, tc.node.GetType()) == nil {
				t.Fatalf("%s: expected view param pills", tc.name)
			}
		})
	}
}

func TestBuildParamEditPills_AllEditableActionTypes(t *testing.T) {
	t.Helper()
	cases := []struct {
		name string
		node actions.ActionInterface
	}{
		{"click", actions.NewClick(actions.ClickButtonLeft, false)},
		{"key", actions.NewKey("a", true)},
		{"wait", actions.NewWait(100)},
		{"move", actions.NewMove(actions.CoordinateRef(""), false)},
		{"loop", actions.NewLoop(3, "inner", nil)},
		{"conditional", actions.NewConditional(nil, actions.MatchAll, "c", nil)},
		{"setvariable", actions.NewSetVariable("x", 1)},
		{"calculate", actions.NewCalculate("1+1", "out")},
		{"runmacro", actions.NewRunMacro("other")},
	}
	for _, tc := range cases {
		t.Run(tc.name, func(t *testing.T) {
			pills, _ := buildParamEditPills(tc.node, tc.node.GetType(), nil)
			if pills == nil {
				t.Fatalf("%s: expected edit param pills", tc.name)
			}
		})
	}
}
