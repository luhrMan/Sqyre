package macro

import (
	"testing"

	"Sqyre/internal/models"
	"Sqyre/internal/models/actions"

	"fyne.io/fyne/v2/canvas"
	"fyne.io/fyne/v2/container"
)

// buildDnDTree returns a tree:
//
//	root
//	├── waitA
//	├── loop (open)
//	│   └── waitB
//	└── waitC
func buildDnDTree(t *testing.T) (mt *MacroTree, waitA, waitB, waitC *actions.Wait, loop *actions.Loop) {
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

	mt = &MacroTree{Macro: &models.Macro{Root: root}}
	mt.setTree()
	return mt, waitA, waitB, waitC, loop
}

func childUIDs(adv actions.AdvancedActionInterface) []string {
	subs := adv.GetSubActions()
	out := make([]string, len(subs))
	for i, s := range subs {
		out[i] = s.GetUID()
	}
	return out
}

func TestVisibleRowUIDs_collapsedAndOpen(t *testing.T) {
	mt, waitA, waitB, waitC, loop := buildDnDTree(t)

	got := mt.visibleRowUIDs()
	want := []string{waitA.GetUID(), loop.GetUID(), waitC.GetUID()}
	if len(got) != len(want) {
		t.Fatalf("collapsed visible = %v, want %v", got, want)
	}
	for i := range want {
		if got[i] != want[i] {
			t.Fatalf("collapsed visible = %v, want %v", got, want)
		}
	}

	mt.OpenBranch(loop.GetUID())
	got = mt.visibleRowUIDs()
	want = []string{waitA.GetUID(), loop.GetUID(), waitB.GetUID(), waitC.GetUID()}
	if len(got) != len(want) {
		t.Fatalf("open visible = %v, want %v", got, want)
	}
	for i := range want {
		if got[i] != want[i] {
			t.Fatalf("open visible = %v, want %v", got, want)
		}
	}
}

func TestPerformMove_reorderSiblings(t *testing.T) {
	mt, waitA, _, waitC, loop := buildDnDTree(t)

	// Move waitC before waitA at root level.
	mt.dragSrcUID = waitC.GetUID()
	mt.dropParent = mt.Macro.Root
	mt.dropTargetUID = waitA.GetUID()
	mt.dropMode = dropBefore
	mt.dropValid = true

	if !mt.performMove() {
		t.Fatal("performMove returned false")
	}
	got := childUIDs(mt.Macro.Root)
	want := []string{waitC.GetUID(), waitA.GetUID(), loop.GetUID()}
	for i := range want {
		if got[i] != want[i] {
			t.Fatalf("root children = %v, want %v", got, want)
		}
	}
}

func TestPerformMove_reparentIntoBranch(t *testing.T) {
	mt, waitA, waitB, _, loop := buildDnDTree(t)

	// Move waitA into loop as first child.
	mt.dragSrcUID = waitA.GetUID()
	mt.dropParent = loop
	mt.dropTargetUID = loop.GetUID()
	mt.dropMode = dropIntoStart
	mt.dropValid = true

	if !mt.performMove() {
		t.Fatal("performMove returned false")
	}
	if got := childUIDs(loop); len(got) != 2 || got[0] != waitA.GetUID() || got[1] != waitB.GetUID() {
		t.Fatalf("loop children = %v, want [waitA waitB]", got)
	}
	if waitA.GetParent() != actions.AdvancedActionInterface(loop) {
		t.Fatal("waitA parent not updated to loop")
	}
	rootKids := childUIDs(mt.Macro.Root)
	if len(rootKids) != 2 {
		t.Fatalf("root should have 2 children after move, got %v", rootKids)
	}
}

func TestPerformMove_appendIntoEnd(t *testing.T) {
	mt, _, waitB, waitC, loop := buildDnDTree(t)

	mt.dragSrcUID = waitC.GetUID()
	mt.dropParent = loop
	mt.dropTargetUID = loop.GetUID()
	mt.dropMode = dropIntoEnd
	mt.dropValid = true

	if !mt.performMove() {
		t.Fatal("performMove returned false")
	}
	if got := childUIDs(loop); len(got) != 2 || got[0] != waitB.GetUID() || got[1] != waitC.GetUID() {
		t.Fatalf("loop children = %v, want [waitB waitC]", got)
	}
}

func TestPerformMove_noOpOnSelfTarget(t *testing.T) {
	mt, waitA, _, _, _ := buildDnDTree(t)

	mt.dragSrcUID = waitA.GetUID()
	mt.dropParent = mt.Macro.Root
	mt.dropTargetUID = waitA.GetUID()
	mt.dropMode = dropAfter

	if mt.performMove() {
		t.Fatal("expected no-op move to return false when target is the dragged node")
	}
	if got := childUIDs(mt.Macro.Root); len(got) != 3 {
		t.Fatalf("root children changed unexpectedly: %v", got)
	}
}

func TestValidateDrop_rejectsCycle(t *testing.T) {
	mt, _, waitB, _, loop := buildDnDTree(t)
	mt.OpenBranch(loop.GetUID())

	// Drag the loop, attempt to drop it into its own child waitB's position
	// (before waitB) — parent would be the loop itself, a cycle.
	mt.dragSrcUID = loop.GetUID()
	mt.dragVisible = mt.visibleRowUIDs()

	// before waitB => parent is loop (descendant target of the dragged loop).
	mt.setDropSibling(waitB, dropBefore)
	mt.validateDrop()
	if mt.dropValid {
		t.Fatal("expected drop into own subtree to be invalid")
	}

	// into the loop itself => parent == src.
	mt.setDropInto(loop, dropIntoEnd)
	mt.validateDrop()
	if mt.dropValid {
		t.Fatal("expected drop into the dragged node itself to be invalid")
	}
}

func TestDoAutoExpand_opensBranch(t *testing.T) {
	mt, _, waitB, waitC, loop := buildDnDTree(t)
	mt.dragActive = true
	mt.dragSrcUID = waitC.GetUID()
	mt.dragVisible = mt.visibleRowUIDs()

	if mt.IsBranchOpen(loop.GetUID()) {
		t.Fatal("loop should start collapsed")
	}
	mt.doAutoExpand(loop.GetUID())
	if !mt.IsBranchOpen(loop.GetUID()) {
		t.Fatal("expected loop to be expanded after auto-expand")
	}
	if indexOfString(mt.dragVisible, waitB.GetUID()) < 0 {
		t.Fatalf("dragVisible should include child after expand: %v", mt.dragVisible)
	}
}

func TestUpdateAutoExpand_skipsInvalidCandidates(t *testing.T) {
	mt, _, _, _, loop := buildDnDTree(t)
	mt.dragActive = true
	mt.dragVisible = mt.visibleRowUIDs()
	loopIdx := indexOfString(mt.dragVisible, loop.GetUID())

	// Dragging the branch itself: it must not schedule its own expansion.
	mt.dragSrcUID = loop.GetUID()
	mt.updateAutoExpand(loopIdx)
	if mt.autoExpandUID != "" {
		t.Fatal("should not schedule expansion of the dragged branch itself")
	}

	// Dragging some other node: the collapsed branch becomes a candidate.
	mt.dragSrcUID = "other"
	mt.updateAutoExpand(loopIdx)
	if mt.autoExpandUID != loop.GetUID() {
		t.Fatalf("expected scheduled expand of loop, got %q", mt.autoExpandUID)
	}
	mt.cancelAutoExpand()
	if mt.autoExpandTimer != nil || mt.autoExpandUID != "" {
		t.Fatal("cancelAutoExpand should clear timer and uid")
	}
}

func TestApplyHighlightOverlay_dragSource(t *testing.T) {
	mt, waitA, _, _, _ := buildDnDTree(t)

	hlSimple := canvas.NewRectangle(highlightSimpleColor)
	hlFill := canvas.NewRectangle(highlightFillColor)
	hlSimple.Hide()
	hlFill.Hide()
	hlBg := container.New(&fillLayout{}, hlSimple, hlFill)

	mt.dragActive = true
	mt.dragSrcUID = waitA.GetUID()

	mt.applyHighlightOverlay(waitA.GetUID(), hlBg)
	if !hlSimple.Visible() {
		t.Fatal("expected drag source row to show highlight rectangle")
	}
	if hlSimple.FillColor != dragSourceColor {
		t.Fatalf("drag source color = %v, want %v", hlSimple.FillColor, dragSourceColor)
	}
	if hlFill.Visible() {
		t.Fatal("fill rectangle should be hidden for a drag source")
	}

	// A non-source row shows no highlight while dragging.
	mt.applyHighlightOverlay("other-uid", hlBg)
	if hlSimple.Visible() || hlFill.Visible() {
		t.Fatal("non-source row should not be highlighted")
	}

	// After the drag ends the source row clears.
	mt.dragActive = false
	mt.applyHighlightOverlay(waitA.GetUID(), hlBg)
	if hlSimple.Visible() || hlFill.Visible() {
		t.Fatal("source highlight should clear once drag ends")
	}
}

func TestFlushNodeCache_restoresRoot(t *testing.T) {
	mt, _, _, _, _ := buildDnDTree(t)
	mt.flushNodeCache()
	if mt.Tree.Root != "" {
		t.Fatalf("Root after flush = %q, want empty", mt.Tree.Root)
	}
	// Structure must still be walkable (no panic, children intact).
	if got := mt.visibleRowUIDs(); len(got) != 3 {
		t.Fatalf("visible rows after flush = %v, want 3", got)
	}
}

func TestResolveDrop_branchZones(t *testing.T) {
	mt, _, _, _, loop := buildDnDTree(t)
	rowH, _ := mt.dragMetrics()
	loopIdx := indexOfString(mt.visibleRowUIDs(), loop.GetUID())
	mt.dragVisible = mt.visibleRowUIDs()
	mt.dragSrcUID = "" // no node dragged; only checking zone resolution

	// Closed branch, pointer near top => before.
	mt.resolveDrop(loopIdx, -rowH*0.4, rowH)
	if mt.dropMode != dropBefore {
		t.Fatalf("top zone mode = %v, want dropBefore", mt.dropMode)
	}
	// Closed branch, pointer at center => into (end).
	mt.resolveDrop(loopIdx, 0, rowH)
	if mt.dropMode != dropIntoEnd {
		t.Fatalf("center zone mode = %v, want dropIntoEnd", mt.dropMode)
	}
	// Closed branch, pointer near bottom => after.
	mt.resolveDrop(loopIdx, rowH*0.4, rowH)
	if mt.dropMode != dropAfter {
		t.Fatalf("bottom zone mode = %v, want dropAfter", mt.dropMode)
	}
}
