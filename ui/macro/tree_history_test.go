package macro

import (
	"testing"

	"Sqyre/internal/models"
	"Sqyre/internal/models/actions"
)

func TestTreeHistory_undoRedo_insertAndRemove(t *testing.T) {
	waitA := actions.NewWait(1)
	waitB := actions.NewWait(2)
	root := actions.NewLoop(1, "root", []actions.ActionInterface{waitA, waitB})
	mt := &MacroTree{
		Macro:   &models.Macro{Root: root},
		history: newTreeHistory(),
	}
	mt.setTree()

	mt.recordMutation()
	waitC := actions.NewWait(3)
	mt.insertActionAt(root, len(root.GetSubActions()), waitC)
	mt.Refresh()

	got := childUIDs(root)
	if len(got) != 3 || got[2] != waitC.GetUID() {
		t.Fatalf("after insert children = %v", got)
	}

	if !mt.Undo() {
		t.Fatal("Undo failed")
	}
	got = childUIDs(root)
	if len(got) != 2 {
		t.Fatalf("after undo children = %v, want 2", got)
	}

	if !mt.Redo() {
		t.Fatal("Redo failed")
	}
	got = childUIDs(root)
	if len(got) != 3 || got[2] != waitC.GetUID() {
		t.Fatalf("after redo children = %v", got)
	}
}

func TestTreeHistory_undoRedo_moveNode(t *testing.T) {
	waitA := actions.NewWait(1)
	waitB := actions.NewWait(2)
	root := actions.NewLoop(1, "root", []actions.ActionInterface{waitA, waitB})
	mt := &MacroTree{
		Macro:        &models.Macro{Root: root},
		history:      newTreeHistory(),
		SelectedNode: waitB.GetUID(),
	}
	mt.setTree()

	mt.moveNode(waitB.GetUID(), true)
	if childUIDs(root)[0] != waitB.GetUID() {
		t.Fatalf("move up failed: %v", childUIDs(root))
	}

	if !mt.Undo() {
		t.Fatal("Undo failed")
	}
	if childUIDs(root)[0] != waitA.GetUID() {
		t.Fatalf("after undo order = %v", childUIDs(root))
	}
}

func TestTreeHistory_snapshotPreservesUIDs(t *testing.T) {
	waitA := actions.NewWait(1)
	waitB := actions.NewWait(2)
	root := actions.NewLoop(1, "root", []actions.ActionInterface{waitA, waitB})
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
	root := actions.NewLoop(1, "root", []actions.ActionInterface{wait})
	mt := &MacroTree{
		Macro:   &models.Macro{Root: root},
		history: newTreeHistory(),
	}
	mt.setTree()

	mt.recordMutation()
	mt.insertActionAt(root, 1, actions.NewWait(2))
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
