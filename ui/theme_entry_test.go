package ui

import (
	"strconv"
	"testing"

	"Sqyre/ui/custom_widgets"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/test"
	"fyne.io/fyne/v2/theme"
)

func TestEntryMinHeightFitsTextAtLargeFontSize(t *testing.T) {
	t.Helper()
	test.NewApp()

	for _, fontSize := range []int{14, 20, 28} {
		t.Run(strconv.Itoa(fontSize), func(t *testing.T) {
			t.Helper()
			applyAppearanceTheme(fontSize, 1.0)

			th := fyne.CurrentApp().Settings().Theme()
			textSize := th.Size(theme.SizeNameText)
			innerPad := th.Size(theme.SizeNameInnerPadding)
			inputBorder := th.Size(theme.SizeNameInputBorder)

			entry := custom_widgets.NewFormEntry()
			entry.SetText("Mg")
			entryMin := entry.MinSize()
			textHeight := fyne.MeasureText("Mg", textSize, fyne.TextStyle{}).Height
			scrollHeight := entryMin.Height - inputBorder*2

			if scrollHeight < textHeight {
				t.Fatalf("font=%d innerPad=%.1f entryMin=%.1f scrollH=%.1f textH=%.1f",
					fontSize, innerPad, entryMin.Height, scrollHeight, textHeight)
			}
		})
	}
}

func TestFormEntryDisablesTruncationScroll(t *testing.T) {
	t.Helper()
	test.NewApp()

	entry := custom_widgets.NewFormEntry()
	if entry.Wrapping != fyne.TextWrapOff {
		t.Fatalf("Wrapping = %v, want TextWrapOff", entry.Wrapping)
	}
	if entry.Scroll != fyne.ScrollNone {
		t.Fatalf("Scroll = %v, want ScrollNone", entry.Scroll)
	}
}
