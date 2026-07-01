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

func TestUpdateBranchOpenDebounce_skipsInvalidCandidates(t *testing.T) {
	mt, _, _, _, loop := buildDnDTree(t)
	mt.dragActive = true
	mt.dragVisible = mt.visibleRowUIDs()
	loopIdx := indexOfString(mt.dragVisible, loop.GetUID())

	// Dragging the branch itself: it must not schedule its own expansion.
	mt.dragSrcUID = loop.GetUID()
	mt.updateBranchOpenDebounce(loopIdx)
	if mt.autoExpandUID != "" {
		t.Fatal("should not schedule expansion of the dragged branch itself")
	}

	// Dragging some other node: the collapsed branch becomes a candidate.
	mt.dragSrcUID = "other"
	mt.updateBranchOpenDebounce(loopIdx)
	if mt.autoExpandUID != loop.GetUID() {
		t.Fatalf("expected scheduled expand of loop, got %q", mt.autoExpandUID)
	}
	mt.cancelAutoExpand()
	if mt.autoExpandTimer != nil || mt.autoExpandUID != "" {
		t.Fatal("cancelAutoExpand should clear timer and uid")
	}
}

func TestResolveDrop_belowOpenBranchSubtree_targetsRoot(t *testing.T) {
	waitA := actions.NewWait(1)
	waitB := actions.NewWait(2)
	loop := actions.NewLoop(1, "loop", nil)
	loop.AddSubAction(waitB)
	root := actions.NewLoop(1, "root", nil)
	root.AddSubAction(waitA)
	root.AddSubAction(loop)

	mt := &MacroTree{Macro: &models.Macro{Root: root}}
	mt.setTree()
	mt.OpenBranch(loop.GetUID())
	mt.dragVisible = mt.visibleRowUIDs()

	rowH, _ := mt.dragMetrics()
	k := len(mt.dragVisible) - 1
	mt.resolveDrop(k, rowH*0.4, rowH)
	if mt.shouldDropAtRootBelowLastBranch(k, rowH*0.4, rowH) {
		mt.setDropRootAfterLastChild()
		mt.validateDrop()
	}

	if mt.dropParent != mt.Macro.Root {
		t.Fatalf("expected root parent, got %T", mt.dropParent)
	}
	if mt.dropMode != dropAfter || mt.dropTargetUID != loop.GetUID() {
		t.Fatalf("expected drop after loop at root, got mode=%v target=%q", mt.dropMode, mt.dropTargetUID)
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
		t.Fatal("expected drag source row highlight")
	}
	if hlSimple.FillColor != dragSourceColor {
		t.Fatalf("drag source color = %v, want %v", hlSimple.FillColor, dragSourceColor)
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

func TestPreviewVisibleRowUIDs_previewSlot(t *testing.T) {
	mt, waitA, _, waitC, loop := buildDnDTree(t)
	mt.dragActive = true
	mt.dragSrcUID = waitC.GetUID()
	mt.dragVisible = mt.visibleRowUIDs()

	mt.dropParent = mt.Macro.Root
	mt.dropTargetUID = waitA.GetUID()
	mt.dropMode = dropBefore
	mt.dropValid = true

	got := mt.previewVisibleRowUIDs()
	want := []string{waitC.GetUID(), waitA.GetUID(), loop.GetUID()}
	for i := range want {
		if got[i] != want[i] {
			t.Fatalf("preview visible = %v, want %v", got, want)
		}
	}
	if got := childUIDs(mt.Macro.Root); got[2] != waitC.GetUID() {
		t.Fatalf("model unchanged before live preview, root = %v", got)
	}
}

func TestApplyDragPreview_shiftsSiblings(t *testing.T) {
	mt, waitA, _, waitC, loop := buildDnDTree(t)
	mt.dragActive = true
	mt.dragSrcUID = waitC.GetUID()
	mt.dragVisible = mt.visibleRowUIDs()
	mt.captureDragOrigin(waitC)

	mt.dropParent = mt.Macro.Root
	mt.dropTargetUID = waitA.GetUID()
	mt.dropMode = dropBefore
	mt.dropValid = true

	key := mt.dropFingerprint()
	mt.applyDragPreview(key)

	if !mt.dragPreviewInTree || mt.dragPreviewKey != key {
		t.Fatal("expected live preview to be active")
	}
	got := childUIDs(mt.Macro.Root)
	want := []string{waitC.GetUID(), waitA.GetUID(), loop.GetUID()}
	for i := range want {
		if got[i] != want[i] {
			t.Fatalf("root children after preview = %v, want %v", got, want)
		}
	}
}

func TestDragDrop_undoRestoresMovedAction(t *testing.T) {
	mt, waitA, _, waitC, loop := buildDnDTree(t)
	mt.history = newTreeHistory()

	snap, err := snapshotTree(mt.Macro.Root, "")
	if err != nil {
		t.Fatalf("snapshotTree: %v", err)
	}
	mt.dragUndoSnapshot = snap
	mt.dragUndoSnapshotOK = true
	mt.dragSrcUID = waitC.GetUID()
	mt.captureDragOrigin(waitC)
	mt.dropParent = mt.Macro.Root
	mt.dropTargetUID = waitA.GetUID()
	mt.dropMode = dropBefore
	mt.dropValid = true

	if !mt.relocateDraggedNode(false) {
		t.Fatal("relocateDraggedNode failed")
	}
	mt.commitDragUndoSnapshot()

	if got := childUIDs(mt.Macro.Root); got[0] != waitC.GetUID() {
		t.Fatalf("after move root = %v", got)
	}
	if !mt.Undo() {
		t.Fatal("Undo failed")
	}
	if mt.Macro.Root.GetAction(waitC.GetUID()) == nil {
		t.Fatal("moved action missing after undo")
	}
	got := childUIDs(mt.Macro.Root)
	want := []string{waitA.GetUID(), loop.GetUID(), waitC.GetUID()}
	for i := range want {
		if got[i] != want[i] {
			t.Fatalf("after undo root = %v, want %v", got, want)
		}
	}
}

func TestDragDrop_undoAfterPreviewCommit(t *testing.T) {
	mt, waitA, _, waitC, loop := buildDnDTree(t)
	mt.history = newTreeHistory()
	mt.dragActive = true
	mt.dragSrcUID = waitC.GetUID()
	mt.captureDragOrigin(waitC)

	snap, err := snapshotTree(mt.Macro.Root, "")
	if err != nil {
		t.Fatalf("snapshotTree: %v", err)
	}
	mt.dragUndoSnapshot = snap
	mt.dragUndoSnapshotOK = true

	mt.dropParent = mt.Macro.Root
	mt.dropTargetUID = waitA.GetUID()
	mt.dropMode = dropBefore
	mt.dropValid = true
	mt.applyDragPreview(mt.dropFingerprint())
	mt.commitDragUndoSnapshot()

	if !mt.Undo() {
		t.Fatal("Undo failed")
	}
	if mt.Macro.Root.GetAction(waitC.GetUID()) == nil {
		t.Fatal("moved action missing after undo")
	}
	got := childUIDs(mt.Macro.Root)
	want := []string{waitA.GetUID(), loop.GetUID(), waitC.GetUID()}
	for i := range want {
		if got[i] != want[i] {
			t.Fatalf("after undo root = %v, want %v", got, want)
		}
	}
}

func TestRevertDragPreview_restoresOrigin(t *testing.T) {
	mt, waitA, _, waitC, loop := buildDnDTree(t)
	mt.dragActive = true
	mt.dragSrcUID = waitC.GetUID()
	mt.dragVisible = mt.visibleRowUIDs()
	mt.captureDragOrigin(waitC)

	mt.dropParent = mt.Macro.Root
	mt.dropTargetUID = waitA.GetUID()
	mt.dropMode = dropBefore
	mt.dropValid = true
	mt.applyDragPreview(mt.dropFingerprint())
	mt.revertDragPreview()

	if mt.dragPreviewInTree {
		t.Fatal("preview should be cleared after revert")
	}
	got := childUIDs(mt.Macro.Root)
	want := []string{waitA.GetUID(), loop.GetUID(), waitC.GetUID()}
	for i := range want {
		if got[i] != want[i] {
			t.Fatalf("root children after revert = %v, want %v", got, want)
		}
	}
}

func TestDragAutoOpenedBranches_collapseWhenLeaving(t *testing.T) {
	mt, _, _, _, loop := buildDnDTree(t)
	mt.dragActive = true
	mt.initDragBranchState()
	mt.dragAutoOpenedBranches = map[string]struct{}{loop.GetUID(): {}}

	mt.suppressBranchOpenScroll++
	mt.OpenBranch(loop.GetUID())
	mt.suppressBranchOpenScroll--
	mt.dragVisible = mt.visibleRowUIDs()

	if !mt.IsBranchOpen(loop.GetUID()) {
		t.Fatal("loop should be open for test setup")
	}

	if !mt.syncDragAutoOpenedBranches(-1) {
		t.Fatal("expected collapse when pointer leaves branch")
	}
	if mt.IsBranchOpen(loop.GetUID()) {
		t.Fatal("auto-opened branch should collapse after leaving")
	}
	if _, ok := mt.dragAutoOpenedBranches[loop.GetUID()]; ok {
		t.Fatal("collapsed branch should be removed from drag auto-opened set")
	}
}

func TestDragAutoOpenedBranches_keepsOpenWhileInside(t *testing.T) {
	mt, _, waitB, _, loop := buildDnDTree(t)
	mt.dragActive = true
	mt.initDragBranchState()
	mt.dragAutoOpenedBranches = map[string]struct{}{loop.GetUID(): {}}
	mt.suppressBranchOpenScroll++
	mt.OpenBranch(loop.GetUID())
	mt.suppressBranchOpenScroll--
	mt.dragVisible = mt.visibleRowUIDs()

	childIdx := indexOfString(mt.dragVisible, waitB.GetUID())
	if childIdx < 0 {
		t.Fatal("waitB should be visible inside open loop")
	}
	if mt.syncDragAutoOpenedBranches(childIdx) {
		t.Fatal("should not collapse while pointer is inside auto-opened branch")
	}
	if !mt.IsBranchOpen(loop.GetUID()) {
		t.Fatal("branch should stay open while hovering a child row")
	}
}

func TestDragBranchesToKeepOpen_viaDropParent(t *testing.T) {
	mt, _, waitB, _, loop := buildDnDTree(t)
	mt.dragActive = true
	mt.dragAutoOpenedBranches = map[string]struct{}{loop.GetUID(): {}}
	mt.suppressBranchOpenScroll++
	mt.OpenBranch(loop.GetUID())
	mt.suppressBranchOpenScroll--
	mt.dragVisible = mt.visibleRowUIDs()

	mt.setDropSibling(waitB, dropAfter)
	mt.dropValid = true
	childIdx := indexOfString(mt.dragVisible, waitB.GetUID())
	keep := mt.dragBranchesToKeepOpen(childIdx)
	if _, ok := keep[loop.GetUID()]; !ok {
		t.Fatal("expected auto-opened branch kept while drop parent is inside it")
	}
}

func TestDragBranchesToKeepOpen_openBranchRow(t *testing.T) {
	mt, _, _, _, loop := buildDnDTree(t)
	mt.dragActive = true
	mt.dragAutoOpenedBranches = map[string]struct{}{loop.GetUID(): {}}
	mt.suppressBranchOpenScroll++
	mt.OpenBranch(loop.GetUID())
	mt.suppressBranchOpenScroll--
	mt.dragVisible = mt.visibleRowUIDs()

	loopIdx := indexOfString(mt.dragVisible, loop.GetUID())
	mt.setDropInto(loop, dropIntoStart)
	mt.dropValid = true
	keep := mt.dragBranchesToKeepOpen(loopIdx)
	if _, ok := keep[loop.GetUID()]; !ok {
		t.Fatal("expected auto-opened branch kept while dropping into it")
	}
}

func TestDragMutationNeedsFlush_reparentChangesDepth(t *testing.T) {
	mt, _, waitB, _, loop := buildDnDTree(t)
	mt.OpenBranch(loop.GetUID())

	if !mt.dragMutationNeedsFlush(loop.GetUID(), mt.Macro.Root.GetUID()) {
		t.Fatal("expected flush when moving from branch to root")
	}
	if mt.dragMutationNeedsFlush(mt.Macro.Root.GetUID(), mt.Macro.Root.GetUID()) {
		t.Fatal("expected no flush when reordering root siblings")
	}
	mt.captureDragOrigin(waitB)
	if !mt.dragMutationNeedsFlush(loop.GetUID(), mt.Macro.Root.GetUID()) {
		t.Fatal("expected flush via drag origin when preview returns to root")
	}
}

func TestInsertIndentDepth_usesDropParent(t *testing.T) {
	waitA := actions.NewWait(1)
	waitB := actions.NewWait(2)
	loop := actions.NewLoop(1, "loop", nil)
	loop.AddSubAction(waitB)
	root := actions.NewLoop(1, "root", nil)
	root.AddSubAction(waitA)
	root.AddSubAction(loop)

	mt := &MacroTree{Macro: &models.Macro{Root: root}}
	mt.setTree()
	mt.OpenBranch(loop.GetUID())

	mt.setDropRootAfterLastChild()
	mt.dropValid = true
	if got := mt.insertIndentDepth(); got != 0 {
		t.Fatalf("root sibling insert depth = %d, want 0", got)
	}

	mt.setDropSibling(waitB, dropAfter)
	mt.dropValid = true
	if got := mt.insertIndentDepth(); got != 1 {
		t.Fatalf("branch child insert depth = %d, want 1", got)
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

func TestScheduleClampScroll_skippedDuringDrag(t *testing.T) {
	mt, _, _, _, _ := buildDnDTree(t)
	mt.dragActive = true
	mt.scheduleClampScroll()
	if !mt.dragActive {
		t.Fatal("dragActive should remain true")
	}
}
