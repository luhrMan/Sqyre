package actionrender

import (
	"testing"

	"Sqyre/internal/models/actions"

	"fyne.io/fyne/v2/test"
)

func TestDisplayWidget_NonNilForAllTypes(t *testing.T) {
	test.NewApp()
	types := []struct {
		name string
		a    actions.ActionInterface
	}{
		{"Click", actions.NewClick(false, true)},
		{"Wait", actions.NewWait(100)},
		{"Move", actions.NewMove(actions.Point{Name: "P", X: 1, Y: 2}, false)},
		{"Key", actions.NewKey("a", true)},
		{"Loop", actions.NewLoop(3, "L", nil)},
		{"Ocr", actions.NewOcr("O", nil, "text", actions.SearchArea{Name: "A"})},
		{"ImageSearch", actions.NewImageSearch("S", nil, []string{"a.png"}, actions.SearchArea{Name: "R"}, 1, 1, 0.9, 5)},
		{"FindPixel", actions.NewFindPixel("FP", actions.SearchArea{}, "ff0000", 0, nil)},
		{"SetVariable", actions.NewSetVariable("v", "val")},
		{"SaveVariable", actions.NewSaveVariable("v", "dest", false, false)},
		{"Calculate", actions.NewCalculate("1+1", "r")},
		{"DataList", actions.NewDataList("a\nb", "v", false)},
		{"FocusWindow", actions.NewFocusWindow("chrome")},
		{"RunMacro", actions.NewRunMacro("m")},
		{"Type", actions.NewType("hello", 0)},
	}
	for _, tt := range types {
		t.Run(tt.name, func(t *testing.T) {
			w := DisplayWidget(tt.a)
			if w == nil {
				t.Error("DisplayWidget() returned nil")
			}
		})
	}
}

func TestDisplayWidget_SkipsEmptyValues(t *testing.T) {
	test.NewApp()
	a := actions.NewSetVariable("", "")
	w := DisplayWidget(a)
	if w == nil {
		t.Fatal("DisplayWidget() returned nil")
	}
}
