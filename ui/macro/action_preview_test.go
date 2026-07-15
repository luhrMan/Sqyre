package macro

import (
	"testing"

	"Sqyre/internal/models/actions"
)

func TestActionPreviewLoader(t *testing.T) {
	t.Helper()
	if actionPreviewLoader(actions.NewWait(100)) != nil {
		t.Fatal("wait action should not have preview loader")
	}
	if actionPreviewLoader(actions.NewMove(actions.CoordinateRef(""), false)) != nil {
		t.Fatal("move with empty point should not have preview loader")
	}
	if actionPreviewLoader(actions.NewMove(actions.NewCoordinateRef("prog", "home"), false)) == nil {
		t.Fatal("move with point should have preview loader")
	}
	if actionPreviewLoader(actions.NewImageSearch("s", nil, nil, actions.NewCoordinateRef("prog", "box"), 0.95, 0)) == nil {
		t.Fatal("image search with search area should have preview loader")
	}
	if actionPreviewLoader(actions.NewOcr("o", "t", actions.CoordinateRef(""))) != nil {
		t.Fatal("empty search area should not have preview loader")
	}
	if actionPreviewLoader(actions.NewFindPixel("f", actions.NewCoordinateRef("prog", "box"), "ffffff", 0)) == nil {
		t.Fatal("find pixel with search area should have preview loader")
	}
}

func TestActionDisplayUsesCombinedTooltipWhenPreviewAndExtraParams(t *testing.T) {
	t.Helper()
	move := actions.NewMove(actions.NewCoordinateRef("prog", "home"), true)
	_, extra := actions.DisplayParams(move.Params())
	if len(extra) == 0 {
		t.Fatal("smooth move should have extra params for pill tooltip")
	}
	if actionPreviewLoader(move) == nil {
		t.Fatal("move with point should have preview loader")
	}
}
