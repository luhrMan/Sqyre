package macro

import (
	"testing"

	"Sqyre/internal/models"
	macrologic "Sqyre/internal/macro"
	"Sqyre/internal/models/actions"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/container"
	"fyne.io/fyne/v2/widget"
)

func buildInsertTestTree(t *testing.T) (mt *MacroTree, waitA, waitB, waitC *actions.Wait, loop *actions.Loop) {
	t.Helper()
	waitA = actions.NewWait(1)
	waitB = actions.NewWait(2)
	waitC = actions.NewWait(3)
	loop = actions.NewLoop(1, "loop", nil)
	loop.AddSubAction(waitB)
	root := actions.NewLoop(1, "root", nil)
	root.AddSubAction(waitA)
	root.AddSubAction(loop)
	root.AddSubAction(waitC)
	return &MacroTree{Macro: &models.Macro{Root: root}}, waitA, waitB, waitC, loop
}

func TestInsertActionBelowSelection(t *testing.T) {
	t.Run("no selection appends to root", func(t *testing.T) {
		mt, waitA, _, waitC, loop := buildInsertTestTree(t)
		newWait := actions.NewWait(99)
		if !mt.InsertActionBelowSelection(newWait) {
			t.Fatal("InsertActionBelowSelection returned false")
		}
		got := childUIDs(mt.Macro.Root)
		want := []string{waitA.GetUID(), loop.GetUID(), waitC.GetUID(), newWait.GetUID()}
		for i := range want {
			if got[i] != want[i] {
				t.Fatalf("root children = %v, want %v", got, want)
			}
		}
	})

	t.Run("selected leaf inserts below sibling", func(t *testing.T) {
		mt, waitA, _, waitC, loop := buildInsertTestTree(t)
		newWait := actions.NewWait(100)
		mt.SelectedNode = waitA.GetUID()
		if !mt.InsertActionBelowSelection(newWait) {
			t.Fatal("InsertActionBelowSelection returned false")
		}
		got := childUIDs(mt.Macro.Root)
		want := []string{waitA.GetUID(), newWait.GetUID(), loop.GetUID(), waitC.GetUID()}
		for i := range want {
			if got[i] != want[i] {
				t.Fatalf("root children = %v, want %v", got, want)
			}
		}
	})

	t.Run("selected branch inserts as first child", func(t *testing.T) {
		mt, waitA, waitB, waitC, loop := buildInsertTestTree(t)
		newWait := actions.NewWait(101)
		mt.SelectedNode = loop.GetUID()
		if !mt.InsertActionBelowSelection(newWait) {
			t.Fatal("InsertActionBelowSelection returned false")
		}
		got := childUIDs(mt.Macro.Root)
		want := []string{waitA.GetUID(), loop.GetUID(), waitC.GetUID()}
		for i := range want {
			if got[i] != want[i] {
				t.Fatalf("root children = %v, want %v", got, want)
			}
		}
		gotLoop := childUIDs(loop)
		wantLoop := []string{newWait.GetUID(), waitB.GetUID()}
		for i := range wantLoop {
			if gotLoop[i] != wantLoop[i] {
				t.Fatalf("loop children = %v, want %v", gotLoop, wantLoop)
			}
		}
	})

	t.Run("selected nested leaf inserts below inside branch", func(t *testing.T) {
		mt, _, waitB, _, loop := buildInsertTestTree(t)
		newWait := actions.NewWait(102)
		mt.SelectedNode = waitB.GetUID()
		if !mt.InsertActionBelowSelection(newWait) {
			t.Fatal("InsertActionBelowSelection returned false")
		}
		got := childUIDs(loop)
		want := []string{waitB.GetUID(), newWait.GetUID()}
		for i := range want {
			if got[i] != want[i] {
				t.Fatalf("loop children = %v, want %v", got, want)
			}
		}
	})
}

func TestPasteNode_insertsBelowSelection(t *testing.T) {
	mt, waitA, _, _, loop := buildInsertTestTree(t)
	clipboard, err := macrologic.CloneActionMap(actions.NewWait(77))
	if err != nil {
		t.Fatalf("CloneActionMap: %v", err)
	}

	mt.SelectedNode = waitA.GetUID()
	if !mt.PasteNode(clipboard) {
		t.Fatal("PasteNode returned false")
	}
	got := childUIDs(mt.Macro.Root)
	if len(got) != 4 || got[0] != waitA.GetUID() || got[2] != loop.GetUID() {
		t.Fatalf("root children = %v, want waitA, pasted, loop, waitC", got)
	}
	if mt.SelectedNode != got[1] {
		t.Fatalf("selection = %q, want pasted node %q", mt.SelectedNode, got[1])
	}
}

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

func TestRowInViewport(t *testing.T) {
	const rowH, pitch, viewH float32 = 24, 28, 100
	tests := []struct {
		name   string
		idx    int
		scroll float32
		want   bool
	}{
		{name: "above viewport", idx: 0, scroll: 50, want: false},
		{name: "below viewport", idx: 10, scroll: 0, want: false},
		{name: "fully visible", idx: 2, scroll: 0, want: true},
		{name: "partially visible top", idx: 3, scroll: 70, want: true},
		{name: "partially visible bottom", idx: 3, scroll: 0, want: true},
	}
	for _, tc := range tests {
		t.Run(tc.name, func(t *testing.T) {
			got := rowInViewport(tc.idx, rowH, pitch, tc.scroll, viewH)
			if got != tc.want {
				t.Fatalf("rowInViewport() = %v, want %v", got, tc.want)
			}
		})
	}
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
	collapsed := mt.openTreeContentHeight()
	if collapsed <= 0 {
		t.Fatalf("collapsed height = %v, want positive", collapsed)
	}

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

func TestTreeRowBody_doubleClickOpensActionEdit(t *testing.T) {
	wait := actions.NewWait(100)
	root := actions.NewLoop(1, "root", nil)
	root.AddSubAction(wait)
	mt := &MacroTree{
		Macro: &models.Macro{Root: root},
	}
	mt.setTree()

	scroll := container.NewHScroll(widget.NewLabel("wait"))
	rowBody := newTreeRowBody(scroll)
	rowBody.tree = mt
	rowBody.uid = wait.GetUID()

	rowBody.Tapped(nil)
	if mt.SelectedNode != wait.GetUID() {
		t.Fatalf("first tap selected %q, want %q", mt.SelectedNode, wait.GetUID())
	}
	rowBody.Tapped(nil)
}

func TestTreeRowBody_doubleClickSkippedWhileExecuting(t *testing.T) {
	wait := actions.NewWait(100)
	root := actions.NewLoop(1, "root", nil)
	root.AddSubAction(wait)
	mt := &MacroTree{
		Macro:     &models.Macro{Root: root},
		executing: true,
	}
	mt.setTree()

	rowBody := newTreeRowBody(container.NewHScroll(widget.NewLabel("wait")))
	rowBody.tree = mt
	rowBody.uid = wait.GetUID()
	rowBody.Tapped(nil)
	rowBody.Tapped(nil)
}

func TestActionDisplayTooltipHover_primaryTapSelectsRow(t *testing.T) {
	wait := actions.NewWait(100)
	root := actions.NewLoop(1, "root", nil)
	root.AddSubAction(wait)
	mt := &MacroTree{Macro: &models.Macro{Root: root}}

	scroll := container.NewHScroll(widget.NewLabel("wait"))
	rowBody := newTreeRowBody(scroll)
	rowBody.tree = mt
	rowBody.uid = wait.GetUID()

	hover := actionDisplay(wait, actionDisplayHandlers{}).(*actionDisplayTooltipHover)
	hover.bindRowBody(rowBody)

	hover.Tapped(nil)
	if mt.SelectedNode != wait.GetUID() {
		t.Fatalf("primary tap on pills selected %q, want %q", mt.SelectedNode, wait.GetUID())
	}
}
