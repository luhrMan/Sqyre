package actions

import (
	"image/color"
	"testing"
)

func TestActionPastelColorCustomOverride(t *testing.T) {
	t.Cleanup(ClearAllCustomActionColors)

	custom := color.NRGBA{R: 0x11, G: 0x22, B: 0x33, A: 0xFF}
	SetCustomActionColor(ActionColorKeyMouseKeyboard, custom)

	got := ActionPastelColor("click", false)
	if got != custom {
		t.Fatalf("ActionPastelColor(click) = %+v, want custom %+v", got, custom)
	}

	ClearCustomActionColor(ActionColorKeyMouseKeyboard)
	got = ActionPastelColor("click", false)
	if got == custom {
		t.Fatal("expected default color after clearing override")
	}
}

func TestActionPastelColorWaitOverride(t *testing.T) {
	t.Cleanup(ClearAllCustomActionColors)

	custom := color.NRGBA{R: 0xAA, G: 0xBB, B: 0xCC, A: 0xFF}
	SetCustomActionColor(ActionColorKeyWait, custom)

	got := ActionPastelColor("wait", false)
	if got != custom {
		t.Fatalf("ActionPastelColor(wait) = %+v, want custom %+v", got, custom)
	}
}

func TestDefaultNestedVarRefColorLighterThanVariableAction(t *testing.T) {
	for _, isDark := range []bool{false, true} {
		action := DefaultActionPastelColor("setvariable", isDark)
		nested := DefaultNestedVarRefColor(isDark)
		if nested.R <= action.R || nested.G <= action.G || nested.B <= action.B {
			t.Fatalf("isDark=%v nested %+v should be lighter than action %+v", isDark, nested, action)
		}
	}
}

func TestActionColorKey(t *testing.T) {
	tests := []struct {
		actionType string
		want       string
	}{
		{"click", ActionColorKeyMouseKeyboard},
		{"ImageSearch", ActionColorKeyDetection},
		{"setvariable", ActionColorKeyVariables},
		{"loop", ActionColorKeyMiscellaneous},
		{"wait", ActionColorKeyWait},
		{"unknown", ActionColorKeyDefault},
	}
	for _, tt := range tests {
		if got := actionColorKey(tt.actionType); got != tt.want {
			t.Errorf("actionColorKey(%q) = %q, want %q", tt.actionType, got, tt.want)
		}
	}
}
