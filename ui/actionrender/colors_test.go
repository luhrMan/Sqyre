package actionrender

import (
	"testing"

	"fyne.io/fyne/v2/test"
)

func TestActionCategoryForType(t *testing.T) {
	tests := []struct {
		actionType string
		want       string
	}{
		{"click", "Mouse & Keyboard"},
		{"move", "Mouse & Keyboard"},
		{"key", "Mouse & Keyboard"},
		{"type", "Mouse & Keyboard"},
		{"imagesearch", "Detection"},
		{"ocr", "Detection"},
		{"findpixel", "Detection"},
		{"setvariable", "Variables"},
		{"calculate", "Variables"},
		{"datalist", "Variables"},
		{"savevariable", "Variables"},
		{"wait", "Miscellaneous"},
		{"focuswindow", "Miscellaneous"},
		{"runmacro", "Miscellaneous"},
		{"loop", "Miscellaneous"},
		{"unknown", ""},
		{"", ""},
	}
	for _, tt := range tests {
		t.Run(tt.actionType, func(t *testing.T) {
			got := ActionCategoryForType(tt.actionType)
			if got != tt.want {
				t.Errorf("ActionCategoryForType(%q) = %q, want %q", tt.actionType, got, tt.want)
			}
		})
	}
}

func TestActionPastelColor_NonZeroAlpha(t *testing.T) {
	test.NewApp()
	types := []string{"click", "wait", "imagesearch", "setvariable", "loop", "unknown"}
	for _, at := range types {
		t.Run(at, func(t *testing.T) {
			c := ActionPastelColor(at)
			if c.A == 0 {
				t.Errorf("ActionPastelColor(%q) has zero alpha", at)
			}
		})
	}
}

func TestActionPastelColor_WaitIsDifferent(t *testing.T) {
	test.NewApp()
	waitColor := ActionPastelColor("wait")
	loopColor := ActionPastelColor("loop")
	if waitColor == loopColor {
		t.Error("wait and loop should have different pastel colors")
	}
}
