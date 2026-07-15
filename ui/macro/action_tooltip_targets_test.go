package macro

import (
	"testing"

	"Sqyre/internal/models/actions"

	"fyne.io/fyne/v2"
)

func TestActionDisplayParamsForTree_imageSearchOmitsItems(t *testing.T) {
	is := actions.NewImageSearch("find", nil, []string{"prog|item"}, actions.CoordinateRef("area"), 0.95, 0)
	params := actionDisplayParamsForTree(is)
	for _, p := range params {
		if p.Label == "Items" {
			t.Fatalf("tree display params should omit Items, got %#v", params)
		}
	}
}

func TestImageSearchRowTargetIcons_prependsCountPillWithTargetGlyph(t *testing.T) {
	row := imageSearchRowTargetIcons([]string{"prog|missing-icon"})
	if row == nil {
		t.Fatal("expected row content even when item icons are missing")
	}
	box, ok := row.(*fyne.Container)
	if !ok {
		t.Fatalf("row type = %T, want *fyne.Container", row)
	}
	if len(box.Objects) != 1 {
		t.Fatalf("row objects = %d, want count pill only when item icons are missing", len(box.Objects))
	}
	if _, ok := box.Objects[0].(*fyne.Container); !ok {
		t.Fatalf("first object = %T, want count pill container", box.Objects[0])
	}
}

func TestImageSearchTargetIconsView_includesCountPill(t *testing.T) {
	view := imageSearchTargetIconsView([]string{"prog|missing-icon"})
	if view == nil {
		t.Fatal("expected tooltip view even when item icons are missing")
	}
	rowWrap := findRowWrapContainer(view.(*fyne.Container))
	if rowWrap == nil {
		t.Fatal("expected row-wrap container in tooltip section")
	}
	if len(rowWrap.Objects) < 1 {
		t.Fatalf("row-wrap objects = %d, want count pill at minimum", len(rowWrap.Objects))
	}
}
