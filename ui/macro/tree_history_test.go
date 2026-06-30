package macro

import (
	"testing"

	"Sqyre/internal/models"
	"Sqyre/internal/models/actions"
)

func newHistoryTestRoot(t *testing.T, subs ...actions.ActionInterface) (*actions.Loop, *MacroTree) {
	t.Helper()
	root := actions.NewLoop(1, "root", nil)
	for _, sub := range subs {
		root.AddSubAction(sub)
	}
	mt := &MacroTree{
		Macro:   &models.Macro{Root: root},
		history: newTreeHistory(),
	}
	mt.setTree()
	return root, mt
}

func TestTreeHistory_undoRedo_insertAndRemove(t *testing.T) {
	waitA := actions.NewWait(1)
	waitB := actions.NewWait(2)
	_, mt := newHistoryTestRoot(t, waitA, waitB)

	mt.recordMutation()
	waitC := actions.NewWait(3)
	mt.insertActionAt(mt.Macro.Root, len(mt.Macro.Root.GetSubActions()), waitC)
	mt.Refresh()

	got := childUIDs(mt.Macro.Root)
	if len(got) != 3 || got[2] != waitC.GetUID() {
		t.Fatalf("after insert children = %v", got)
	}

	if !mt.Undo() {
		t.Fatal("Undo failed")
	}
	got = childUIDs(mt.Macro.Root)
	if len(got) != 2 {
		t.Fatalf("after undo children = %v, want 2", got)
	}

	if !mt.Redo() {
		t.Fatal("Redo failed")
	}
	got = childUIDs(mt.Macro.Root)
	if len(got) != 3 || got[2] != waitC.GetUID() {
		t.Fatalf("after redo children = %v", got)
	}
}

func TestTreeHistory_undoRedo_moveNode(t *testing.T) {
	waitA := actions.NewWait(1)
	waitB := actions.NewWait(2)
	_, mt := newHistoryTestRoot(t, waitA, waitB)
	mt.SelectedNode = waitB.GetUID()

	mt.moveNode(waitB.GetUID(), true)
	if childUIDs(mt.Macro.Root)[0] != waitB.GetUID() {
		t.Fatalf("move up failed: %v", childUIDs(mt.Macro.Root))
	}

	if !mt.Undo() {
		t.Fatal("Undo failed")
	}
	if childUIDs(mt.Macro.Root)[0] != waitA.GetUID() {
		t.Fatalf("after undo order = %v", childUIDs(mt.Macro.Root))
	}
}

func TestTreeHistory_snapshotPreservesUIDs(t *testing.T) {
	waitA := actions.NewWait(1)
	waitB := actions.NewWait(2)
	root := actions.NewLoop(1, "root", nil)
	root.AddSubAction(waitA)
	root.AddSubAction(waitB)
	uidA, uidB := waitA.GetUID(), waitB.GetUID()

	snap, err := snapshotTree(root, uidB)
	if err != nil {
		t.Fatalf("snapshotTree: %v", err)
	}
	restored, err := restoreTreeRoot(snap.rootMap)
	if err != nil {
		t.Fatalf("restoreTreeRoot: %v", err)
	}
	got := childUIDs(restored)
	if len(got) != 2 || got[0] != uidA || got[1] != uidB {
		t.Fatalf("restored uids = %v, want %q and %q", got, uidA, uidB)
	}
}

func TestTreeHistory_applyingHistoryDoesNotRecord(t *testing.T) {
	wait := actions.NewWait(1)
	_, mt := newHistoryTestRoot(t, wait)

	mt.recordMutation()
	mt.insertActionAt(mt.Macro.Root, 1, actions.NewWait(2))
	if len(mt.history.undo) != 1 {
		t.Fatalf("undo stack = %d, want 1", len(mt.history.undo))
	}

	mt.applyingHistory = true
	mt.recordMutation()
	mt.applyingHistory = false
	if len(mt.history.undo) != 1 {
		t.Fatalf("undo stack grew during apply: %d", len(mt.history.undo))
	}
}
