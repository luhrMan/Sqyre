package ui

import (
	"testing"

	"Sqyre/ui/completionentry"
	"Sqyre/ui/custom_widgets"

	"fyne.io/fyne/v2/test"
)

func TestShouldSaveActionDialogOnEnter(t *testing.T) {
	test.NewApp()
	w := test.NewWindow(nil)
	defer w.Close()
	c := w.Canvas()

	if !shouldSaveActionDialogOnEnter(w) {
		t.Fatal("expected save with no focused widget")
	}

	entry := completionentry.NewCompletionEntry([]string{"alpha", "beta"})
	entry.SetText("a")
	c.SetContent(entry)
	entry.ShowCompletion()
	if shouldSaveActionDialogOnEnter(w) {
		t.Fatal("expected no save while completion popup is active")
	}

	entry.HideCompletion()
	c.Focus(entry)
	if !shouldSaveActionDialogOnEnter(w) {
		t.Fatal("expected save after completion is hidden and entry is focused")
	}

	multi := custom_widgets.NewMultiLineVarEntry(func() []string { return nil })
	c.SetContent(multi)
	c.Focus(multi)
	if shouldSaveActionDialogOnEnter(w) {
		t.Fatal("expected no save when multi-line VarEntry is focused")
	}

	single := custom_widgets.NewFormEntry()
	c.SetContent(single)
	c.Focus(single)
	if !shouldSaveActionDialogOnEnter(w) {
		t.Fatal("expected save when single-line entry is focused")
	}
}
