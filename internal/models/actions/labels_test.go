package actions

import "testing"

func TestActionTypeLabel(t *testing.T) {
	t.Helper()
	cases := map[string]string{
		"click":        "Click",
		"imagesearch":  "Image Search",
		"semanticsearch": "Semantic Search",
		"setvariable":  "Set",
		"conditional":  "If",
		"focuswindow":  "Focus window",
		"unknown_type": "unknown_type",
	}
	for actionType, want := range cases {
		if got := ActionTypeLabel(actionType); got != want {
			t.Errorf("ActionTypeLabel(%q) = %q, want %q", actionType, got, want)
		}
	}
}
