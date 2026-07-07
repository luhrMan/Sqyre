package custom_widgets

import (
	"testing"

	"Sqyre/internal/config"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/test"
	"fyne.io/fyne/v2/theme"
)

func TestInnerPaddingScalesWithFontSize(t *testing.T) {
	t.Helper()
	test.NewApp()

	basePad := theme.DefaultTheme().Size(theme.SizeNameInnerPadding)
	for _, fontSize := range []int{14, 28} {
		th := scaledInnerPaddingTheme{Theme: theme.DefaultTheme(), fontSize: float32(fontSize)}
		got := th.Size(theme.SizeNameInnerPadding)
		want := basePad * float32(fontSize) / config.DefaultUIFontSize
		if got != want {
			t.Fatalf("font=%d innerPadding = %v, want %v", fontSize, got, want)
		}
	}
}

// scaledInnerPaddingTheme mirrors sqyreTheme's inner-padding scaling for unit tests.
type scaledInnerPaddingTheme struct {
	fyne.Theme
	fontSize float32
}

func (t scaledInnerPaddingTheme) Size(name fyne.ThemeSizeName) float32 {
	base := t.Theme.Size(name)
	fontRatio := t.fontSize / config.DefaultUIFontSize
	switch name {
	case theme.SizeNameText:
		return t.fontSize
	case theme.SizeNameInnerPadding, theme.SizeNameLineSpacing:
		return base * fontRatio
	default:
		return base
	}
}
