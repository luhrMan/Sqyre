package actiondisplay_test

import (
	"testing"

	"Sqyre/internal/models/actions"
	"Sqyre/ui/actiondisplay"
)

func TestClick_Icon(t *testing.T) {
	if actiondisplay.Icon(actions.NewClick(actions.ClickButtonLeft, false)) == nil {
		t.Error("Icon() left click should not be nil")
	}
	if actiondisplay.Icon(actions.NewClick(actions.ClickButtonLeft, true)) == nil {
		t.Error("Icon() hold should not be nil")
	}
}

func TestActionTypes_Icon(t *testing.T) {
	cases := []struct {
		name string
		a    actions.ActionInterface
	}{
		{"Calculate", actions.NewCalculate("1", "x")},
		{"ForEachRow", actions.NewForEachRow("r", nil, nil)},
		{"FocusWindow", actions.NewFocusWindow("/app", "Window")},
		{"ImageSearch", actions.NewImageSearch("s", nil, nil, "", 0, 0, 0, 0)},
		{"Key", actions.NewKey("k", false)},
		{"KeyDown", actions.NewKey("k", true)},
		{"Loop", actions.NewLoop(1, "l", nil)},
		{"Move", actions.NewMove("", false)},
		{"Ocr", actions.NewOcr("o", "", "")},
		{"SaveVariable", actions.NewSaveVariable("v", "d", false, false)},
		{"SetVariable", actions.NewSetVariable("v", 0)},
		{"Wait", actions.NewWait(0)},
		{"Pause", actions.NewPause("", []string{"f9"}, false)},
		{"FindPixel", actions.NewFindPixel("w", "", "000000", 0)},
		{"Break", actions.NewBreak()},
		{"Continue", actions.NewContinue()},
		{"BaseAction", actions.NewClick(actions.ClickButtonLeft, false)},
	}
	for _, tt := range cases {
		t.Run(tt.name, func(t *testing.T) {
			if actiondisplay.Icon(tt.a) == nil {
				t.Error("Icon() should not be nil")
			}
		})
	}
}

func TestIcon_BaseActionDefault(t *testing.T) {
	b := actions.NewClick(actions.ClickButtonLeft, false)
	if actiondisplay.Icon(b) == nil {
		t.Error("Icon() should not be nil")
	}
}
