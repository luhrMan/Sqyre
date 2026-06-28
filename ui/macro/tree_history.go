package macro

import (
	"Sqyre/internal/models/actions"
	"Sqyre/internal/models/serialize"
	"fmt"
	"log"
)

const maxTreeHistoryEntries = 50

type treeSnapshot struct {
	rootMap     map[string]any
	selectedUID string
}

type treeHistory struct {
	undo []treeSnapshot
	redo []treeSnapshot
}

func newTreeHistory() *treeHistory {
	return &treeHistory{}
}

func snapshotTree(root *actions.Loop, selectedUID string) (treeSnapshot, error) {
	if root == nil {
		return treeSnapshot{}, nil
	}
	rootMap, err := serialize.ActionToMap(root)
	if err != nil {
		return treeSnapshot{}, err
	}
	return treeSnapshot{rootMap: rootMap, selectedUID: selectedUID}, nil
}

func (h *treeHistory) push(root *actions.Loop, selectedUID string) {
	snap, err := snapshotTree(root, selectedUID)
	if err != nil {
		log.Printf("tree history: snapshot failed: %v", err)
		return
	}
	h.undo = append(h.undo, snap)
	if len(h.undo) > maxTreeHistoryEntries {
		h.undo = append([]treeSnapshot(nil), h.undo[len(h.undo)-maxTreeHistoryEntries:]...)
	}
	h.redo = h.redo[:0]
}

func (h *treeHistory) canUndo() bool {
	return h != nil && len(h.undo) > 0
}

func (h *treeHistory) canRedo() bool {
	return h != nil && len(h.redo) > 0
}

func (h *treeHistory) popUndo() (treeSnapshot, bool) {
	if !h.canUndo() {
		return treeSnapshot{}, false
	}
	i := len(h.undo) - 1
	snap := h.undo[i]
	h.undo = h.undo[:i]
	return snap, true
}

func (h *treeHistory) popRedo() (treeSnapshot, bool) {
	if !h.canRedo() {
		return treeSnapshot{}, false
	}
	i := len(h.redo) - 1
	snap := h.redo[i]
	h.redo = h.redo[:i]
	return snap, true
}

func (h *treeHistory) pushRedo(snap treeSnapshot) {
	h.redo = append(h.redo, snap)
	if len(h.redo) > maxTreeHistoryEntries {
		h.redo = append([]treeSnapshot(nil), h.redo[len(h.redo)-maxTreeHistoryEntries:]...)
	}
}

func (h *treeHistory) pushUndo(snap treeSnapshot) {
	h.undo = append(h.undo, snap)
	if len(h.undo) > maxTreeHistoryEntries {
		h.undo = append([]treeSnapshot(nil), h.undo[len(h.undo)-maxTreeHistoryEntries:]...)
	}
}

func restoreTreeRoot(rootMap map[string]any) (*actions.Loop, error) {
	rootAction, err := serialize.ViperSerializer.CreateActionFromMap(rootMap, nil)
	if err != nil {
		return nil, err
	}
	root, ok := rootAction.(*actions.Loop)
	if !ok {
		return nil, fmt.Errorf("macro root is not a loop")
	}
	return root, nil
}
