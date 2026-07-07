package macro

import (
	"image/color"
	"testing"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/canvas"
)

func TestRowWrapSingleLineWidth_sumsChildren(t *testing.T) {
	t.Helper()
	a := canvas.NewRectangle(color.NRGBA{A: 0xff})
	b := canvas.NewRectangle(color.NRGBA{A: 0xff})
	a.SetMinSize(fyne.NewSize(40, 10))
	b.SetMinSize(fyne.NewSize(60, 10))

	got := rowWrapSingleLineWidth([]fyne.CanvasObject{a, b})
	if got <= 100 {
		t.Fatalf("expected single-line width > 100, got %v", got)
	}
}

func TestMaxRowWrapSingleLineWidth_findsPillRow(t *testing.T) {
	t.Helper()
	row := newPillRow()
	chip := canvas.NewRectangle(color.NRGBA{A: 0xff})
	chip.SetMinSize(fyne.NewSize(80, 10))
	row.add(chip)
	row.add(chip)

	section := wrapTooltipSection(row.box)
	got := maxRowWrapSingleLineWidth(section)
	if got <= 160 {
		t.Fatalf("expected wrapped pill row width > 160, got %v", got)
	}
}

func TestRowWrapHeightAtWidth_fitsSingleRow(t *testing.T) {
	t.Helper()
	row := newPillRow()
	chip := canvas.NewRectangle(color.NRGBA{A: 0xff})
	chip.SetMinSize(fyne.NewSize(80, 20))
	row.add(chip)
	row.add(chip)
	row.add(chip)

	stackedMin := row.box.MinSize().Height
	singleRow := rowWrapHeightAtWidth(row.box, 500)
	if singleRow >= stackedMin {
		t.Fatalf("expected single-row height %v < stacked min %v", singleRow, stackedMin)
	}
}
