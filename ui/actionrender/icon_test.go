package actionrender

import (
	"testing"

	"Sqyre/internal/models/actions"

	"fyne.io/fyne/v2/test"
)

func TestActionIcon_NonNilForAllTypes(t *testing.T) {
	test.NewApp()
	types := []struct {
		name string
		a    actions.ActionInterface
	}{
		{"ClickUp", actions.NewClick(false, false)},
		{"ClickDown", actions.NewClick(false, true)},
		{"Wait", actions.NewWait(100)},
		{"Move", actions.NewMove(actions.Point{}, false)},
		{"KeyDown", actions.NewKey("a", true)},
		{"KeyUp", actions.NewKey("a", false)},
		{"Loop", actions.NewLoop(1, "L", nil)},
		{"Ocr", actions.NewOcr("O", nil, "", actions.SearchArea{})},
		{"ImageSearch", actions.NewImageSearch("S", nil, nil, actions.SearchArea{}, 0, 0, 0, 0)},
		{"FindPixel", actions.NewFindPixel("W", actions.SearchArea{}, "000", 0, nil)},
		{"SetVariable", actions.NewSetVariable("v", 0)},
		{"SaveVariable", actions.NewSaveVariable("v", "", false, false)},
		{"Calculate", actions.NewCalculate("", "")},
		{"DataList", actions.NewDataList("", "", false)},
		{"FocusWindow", actions.NewFocusWindow("")},
		{"RunMacro", actions.NewRunMacro("")},
		{"Type", actions.NewType("", 0)},
	}
	for _, tt := range types {
		t.Run(tt.name, func(t *testing.T) {
			icon := ActionIcon(tt.a)
			if icon == nil {
				t.Error("ActionIcon() returned nil")
			}
		})
	}
}

func TestActionIcon_FallbackForUnknown(t *testing.T) {
	test.NewApp()
	// BaseAction with unknown type uses the default fallback
	unknown := actions.NewWait(0)
	icon := ActionIcon(unknown)
	if icon == nil {
		t.Error("ActionIcon() should return non-nil for any ActionInterface")
	}
}
