package custom_widgets

import (
	"testing"

	"Sqyre/internal/models"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/test"
)

func TestBorderlessEntryMinSizeGrowsWithText(t *testing.T) {
	t.Helper()
	test.NewApp()

	short := NewBorderlessEntry(nil)
	short.SetText("100")
	shortMin := short.MinSize()

	long := NewBorderlessEntry(nil)
	long.SetText("${veryLongVariableName}")
	longMin := long.MinSize()

	if longMin.Width <= shortMin.Width {
		t.Fatalf("long text min width %v should exceed short text min width %v", longMin.Width, shortMin.Width)
	}
}

func TestBorderlessEntryMinSizeHeightMatchesPillLine(t *testing.T) {
	t.Helper()
	test.NewApp()

	e := NewBorderlessEntry(nil)
	e.SetText("100")
	if got, want := e.MinSize().Height, PillLineHeight(); got != want {
		t.Fatalf("MinSize height = %v, want PillLineHeight %v", got, want)
	}
}

func TestBorderlessEntryUsesScrollNone(t *testing.T) {
	t.Helper()
	test.NewApp()

	e := NewBorderlessEntry(nil)
	if e.Scroll != fyne.ScrollNone {
		t.Fatalf("Scroll = %v, want ScrollNone", e.Scroll)
	}
}

func TestBorderlessEntry_completeVarRef(t *testing.T) {
	t.Helper()
	test.NewApp()

	e := NewBorderlessEntry(func() []models.VariableDef {
		return []models.VariableDef{{Name: "count"}}
	})
	e.SetText("${cou")
	e.CursorColumn = len("${cou")
	e.completeVarRef("count")
	if e.Text != "${count}" {
		t.Fatalf("Text = %q, want %q", e.Text, "${count}")
	}
}

func TestBorderlessEntry_showsPillOverlayWhenUnfocused(t *testing.T) {
	t.Helper()
	test.NewApp()

	e := NewBorderlessEntry(nil)
	e.SetText("${count}")
	e.hasFocus = false
	e.syncPillDisplay()
	if !e.hideTextForPills {
		t.Fatal("expected pill overlay when unfocused with var ref")
	}
}

func TestBorderlessEntry_rendererShowsPillOverlay(t *testing.T) {
	t.Helper()
	test.NewApp()
	w := test.NewWindow(nil)
	defer w.Close()

	e := NewBorderlessEntry(nil)
	e.SetText("${count}")
	e.hasFocus = false
	w.SetContent(e)
	e.Refresh()
	if !e.hideTextForPills {
		t.Fatal("borderless entry refresh should enable pill overlay for var refs")
	}
}

func TestBorderlessEntry_pillOverlayTapFocuses(t *testing.T) {
	t.Helper()
	test.NewApp()
	w := test.NewWindow(nil)
	defer w.Close()

	e := NewBorderlessEntry(nil)
	e.SetText("${count}")
	e.hasFocus = false
	w.SetContent(e)
	e.Refresh()

	host := e.overlay.object(&e.VarEntry)
	if tap, ok := host.(interface{ Tapped(*fyne.PointEvent) }); ok {
		tap.Tapped(&fyne.PointEvent{})
	} else {
		t.Fatal("expected pill overlay host to handle taps")
	}
}

func TestBorderlessEntry_focusOnCanvas(t *testing.T) {
	t.Helper()
	test.NewApp()
	w := test.NewWindow(nil)
	defer w.Close()

	e := NewBorderlessEntry(nil)
	e.SetText("${count}")
	w.SetContent(e)
	e.focusOnCanvas()
}

func TestBorderlessEntry_TappedSecondary(t *testing.T) {
	t.Helper()
	test.NewApp()
	w := test.NewWindow(nil)
	defer w.Close()

	e := NewBorderlessEntry(func() []models.VariableDef {
		return []models.VariableDef{{Name: "count"}}
	})
	e.SetText("100")
	w.SetContent(e)
	w.Resize(fyne.NewSize(400, 200))
	e.TappedSecondary(&fyne.PointEvent{Position: fyne.NewPos(4, 4)})
}
