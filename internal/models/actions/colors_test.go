package actions

import (
	"image/color"
	"testing"
)

func TestActionPastelColorCustomOverride(t *testing.T) {
	t.Cleanup(ClearAllCustomActionColors)

	custom := color.NRGBA{R: 0x11, G: 0x22, B: 0x33, A: 0xFF}
	SetCustomActionColor(ActionColorKeyMouseKeyboard, custom)

	got := ActionPastelColor("click")
	if got != custom {
		t.Fatalf("ActionPastelColor(click) = %+v, want custom %+v", got, custom)
	}

	ClearCustomActionColor(ActionColorKeyMouseKeyboard)
	got = ActionPastelColor("click")
	if got == custom {
		t.Fatal("expected default color after clearing override")
	}
}

func TestActionPastelColorWaitOverride(t *testing.T) {
	t.Cleanup(ClearAllCustomActionColors)

	custom := color.NRGBA{R: 0xAA, G: 0xBB, B: 0xCC, A: 0xFF}
	SetCustomActionColor(ActionColorKeyWait, custom)

	got := ActionPastelColor("wait")
	if got != custom {
		t.Fatalf("ActionPastelColor(wait) = %+v, want custom %+v", got, custom)
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
