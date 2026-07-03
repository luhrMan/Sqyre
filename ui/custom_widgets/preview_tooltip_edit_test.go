package custom_widgets

import "testing"

func TestPreviewTooltipPanelEditCallback(t *testing.T) {
	t.Helper()
	called := false
	panel := newPreviewTooltipPanel(nil, func() { called = true })
	panel.TappedSecondary(nil)
	if !called {
		t.Fatal("expected onEdit to run on right-click")
	}
}

func TestPreviewTooltipPanelNoEditCallback(t *testing.T) {
	t.Helper()
	panel := newPreviewTooltipPanel(nil, nil)
	panel.TappedSecondary(nil) // must not panic
}
