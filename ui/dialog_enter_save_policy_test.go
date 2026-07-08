package ui

import (
	"testing"

	"Sqyre/ui/completionentry"
	"Sqyre/ui/custom_widgets"

	"fyne.io/fyne/v2/test"
)

func TestShouldSaveTooltipOnEnter(t *testing.T) {
	test.NewApp()
	SetAppInForegroundForTesting(true)
	t.Cleanup(func() { SetAppInForegroundForTesting(true) })
	w := test.NewWindow(nil)
	defer w.Close()
	c := w.Canvas()

	if !shouldSaveTooltipOnEnter(w) {
		t.Fatal("expected save with no focused widget")
	}

	entry := completionentry.NewCompletionEntry([]string{"alpha", "beta"})
	entry.SetText("a")
	c.SetContent(entry)
	entry.ShowCompletion()
	if shouldSaveTooltipOnEnter(w) {
		t.Fatal("expected no save while completion popup is active")
	}

	entry.HideCompletion()
	c.Focus(entry)
	if !shouldSaveTooltipOnEnter(w) {
		t.Fatal("expected save after completion is hidden and entry is focused")
	}

	multi := custom_widgets.NewMultiLineVarEntry(func() []string { return nil })
	c.SetContent(multi)
	c.Focus(multi)
	if shouldSaveTooltipOnEnter(w) {
		t.Fatal("expected no save when multi-line VarEntry is focused")
	}

	single := custom_widgets.NewFormEntry()
	c.SetContent(single)
	c.Focus(single)
	if !shouldSaveTooltipOnEnter(w) {
		t.Fatal("expected save when single-line entry is focused")
	}
}

func TestShouldSaveTooltipOnEnterRequiresForeground(t *testing.T) {
	test.NewApp()
	w := test.NewWindow(nil)
	defer w.Close()

	SetAppInForegroundForTesting(true)
	if !shouldSaveTooltipOnEnter(w) {
		t.Fatal("expected save while app is foreground")
	}

	SetAppInForegroundForTesting(false)
	if shouldSaveTooltipOnEnter(w) {
		t.Fatal("expected no save while app is in background")
	}
}
