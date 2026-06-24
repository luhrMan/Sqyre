package macro

import (
	"testing"

	"Sqyre/internal/models"
	"Sqyre/internal/models/actions"

	"fyne.io/fyne/v2"
)

func TestGoToAction_ignoresEmptyUID(t *testing.T) {
	mt := &MacroTree{}
	mt.GoToAction("")
	if mt.SelectedNode != "" {
		t.Fatal("expected no selection for empty uid")
	}
}

func TestFillNearlyEqual(t *testing.T) {
	if !fillNearlyEqual(0.5, 0.5005) {
		t.Fatal("expected fractions within epsilon to be equal")
	}
	if fillNearlyEqual(0.5, 0.51) {
		t.Fatal("expected distinct fractions to differ")
	}
}

func TestAncestorUIDs_nestedAction(t *testing.T) {
	inner := actions.NewLoop(1, "outer", nil)
	child := actions.NewWait(10)
	inner.AddSubAction(child)
	root := actions.NewLoop(1, "root", []actions.ActionInterface{inner})

	mt := &MacroTree{Macro: &models.Macro{Root: root}}
	ancestors := mt.ancestorUIDs(child.GetUID())
	if len(ancestors) != 1 || ancestors[0] != inner.GetUID() {
		t.Fatalf("ancestors = %v, want [%s]", ancestors, inner.GetUID())
	}
}

func TestBranchesToKeepOpen(t *testing.T) {
	wait := actions.NewWait(100)
	inner := actions.NewLoop(1, "inner", nil)
	inner.AddSubAction(wait)
	outer := actions.NewLoop(1, "outer", nil)
	outer.AddSubAction(inner)
	root := actions.NewLoop(1, "root", nil)
	root.AddSubAction(outer)

	mt := &MacroTree{
		Macro: &models.Macro{Root: root},
		fills: map[string]float64{},
	}
	mt.setTree()

	t.Run("cursor inside nested branch keeps ancestors", func(t *testing.T) {
		mt.cursorUID = wait.GetUID()
		keep := mt.branchesToKeepOpen()
		if !keep[inner.GetUID()] || !keep[outer.GetUID()] {
			t.Fatalf("keep = %#v, want inner and outer", keep)
		}
	})

	t.Run("cursor on branch row keeps that branch", func(t *testing.T) {
		mt.cursorUID = outer.GetUID()
		keep := mt.branchesToKeepOpen()
		if !keep[outer.GetUID()] {
			t.Fatalf("keep = %#v, want outer when cursor is on branch row", keep)
		}
		if keep[inner.GetUID()] {
			t.Fatalf("keep = %#v, inner should not be kept without cursor inside", keep)
		}
	})

	t.Run("container fill keeps branch open", func(t *testing.T) {
		mt.cursorUID = ""
		mt.fills[inner.GetUID()] = 0.5
		keep := mt.branchesToKeepOpen()
		if !keep[inner.GetUID()] || !keep[outer.GetUID()] {
			t.Fatalf("keep = %#v, want fill target and its ancestors", keep)
		}
	})

	t.Run("empty highlight keeps nothing", func(t *testing.T) {
		mt.cursorUID = ""
		mt.fills = map[string]float64{}
		keep := mt.branchesToKeepOpen()
		if len(keep) != 0 {
			t.Fatalf("keep = %#v, want empty", keep)
		}
	})
}

func TestClampScrollY(t *testing.T) {
	tests := []struct {
		name      string
		contentH  float32
		viewH     float32
		currentY  float32
		want      float32
	}{
		{name: "content fits", contentH: 80, viewH: 200, currentY: 50, want: 0},
		{name: "within bounds", contentH: 400, viewH: 200, currentY: 100, want: 100},
		{name: "past bottom", contentH: 250, viewH: 200, currentY: 120, want: 50},
	}
	for _, tc := range tests {
		t.Run(tc.name, func(t *testing.T) {
			got := clampedScrollY(tc.contentH, tc.viewH, tc.currentY)
			if got != tc.want {
				t.Fatalf("clampedScrollY() = %v, want %v", got, tc.want)
			}
		})
	}
}

func TestOpenTreeContentHeight_nestedBranches(t *testing.T) {
	inner := actions.NewLoop(1, "inner", nil)
	inner.AddSubAction(actions.NewWait(10))
	outer := actions.NewLoop(1, "outer", nil)
	outer.AddSubAction(inner)
	root := actions.NewLoop(1, "root", nil)
	root.AddSubAction(outer)

	mt := &MacroTree{Macro: &models.Macro{Root: root}}
	mt.setTree()

	// Only top-level row visible when outer is collapsed.
	if h := mt.openTreeContentHeight(); h <= 0 {
		t.Fatalf("collapsed height = %v, want positive", h)
	}
	collapsed := h

	mt.OpenAllBranches()
	expanded := mt.openTreeContentHeight()
	if expanded <= collapsed {
		t.Fatalf("expanded height = %v, want > collapsed %v", expanded, collapsed)
	}
}

func TestOpenCloseAllBranches(t *testing.T) {
	inner := actions.NewLoop(1, "inner", nil)
	outer := actions.NewLoop(1, "outer", nil)
	outer.AddSubAction(inner)
	root := actions.NewLoop(1, "root", nil)
	root.AddSubAction(outer)

	mt := &MacroTree{Macro: &models.Macro{Root: root}}
	mt.setTree()

	mt.OpenAllBranches()
	if !mt.IsBranchOpen(outer.GetUID()) || !mt.IsBranchOpen(inner.GetUID()) {
		t.Fatalf("expected all branches open, outer=%v inner=%v",
			mt.IsBranchOpen(outer.GetUID()), mt.IsBranchOpen(inner.GetUID()))
	}

	mt.CloseAllBranches()
	if mt.IsBranchOpen(outer.GetUID()) || mt.IsBranchOpen(inner.GetUID()) {
		t.Fatalf("expected all branches closed, outer=%v inner=%v",
			mt.IsBranchOpen(outer.GetUID()), mt.IsBranchOpen(inner.GetUID()))
	}
}

func TestSetFillSkipsUnchangedFraction(t *testing.T) {
	uid := "fill-test"
	mt := &MacroTree{fills: map[string]float64{uid: 0.25}}
	mt.SetFill(uid, 0.2505)
	if len(mt.highlightOnlyRefresh) != 0 {
		t.Fatalf("unchanged fill should not mark highlight refresh, got %d", len(mt.highlightOnlyRefresh))
	}
}

func TestHighlightOnlyRefreshRequiresBoundNode(t *testing.T) {
	uid := "action-a"
	mt := &MacroTree{}
	obj := &fyne.Container{}

	if mt.nodeObjectShowsUID(obj, uid) {
		t.Fatal("unbound object should not match uid")
	}

	mt.markNodeBound(obj, uid)
	if !mt.nodeObjectShowsUID(obj, uid) {
		t.Fatal("expected object to be bound to uid")
	}
	if mt.nodeObjectShowsUID(obj, "other") {
		t.Fatal("object should not appear bound to a different uid")
	}

	mt.markHighlightRefresh(uid)
	if !mt.consumeHighlightRefresh(uid) {
		t.Fatal("expected highlight refresh flag to be consumed")
	}
}
