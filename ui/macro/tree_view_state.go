package macro

import (
	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/widget"
)

// treeViewState captures scroll position and branch expansion so undo/redo
// does not reset the user's place in the macro tree.
type treeViewState struct {
	scrollY      float32
	openBranches []string
}

func (mt *MacroTree) captureViewState() treeViewState {
	scrollY, _ := treeScrollOffsetY(&mt.Tree)
	return treeViewState{
		scrollY:      scrollY,
		openBranches: mt.collectOpenBranchUIDs(),
	}
}

func (mt *MacroTree) collectOpenBranchUIDs() []string {
	var open []string
	for _, uid := range mt.collectBranchUIDs() {
		if mt.IsBranchOpen(uid) {
			open = append(open, uid)
		}
	}
	return open
}

func (mt *MacroTree) restoreViewState(v treeViewState) {
	if mt.Macro == nil || mt.Macro.Root == nil {
		return
	}
	mt.suppressBranchOpenScroll++
	defer func() { mt.suppressBranchOpenScroll-- }()
	for _, uid := range v.openBranches {
		if uid == "" || !mt.IsBranch(uid) {
			continue
		}
		if mt.Macro.Root.GetAction(uid) == nil {
			continue
		}
		if !mt.IsBranchOpen(uid) {
			mt.OpenBranch(uid)
		}
	}
	if v.scrollY > 0 {
		mt.ScrollToOffset(v.scrollY)
	}
	mt.scheduleClampScroll()
}

// unselectMacroTreeAction clears macro tree selection and the focus highlight
// that remains after UnselectAll while the tree widget still has focus.
func unselectMacroTreeAction(mt *MacroTree) {
	if mt == nil {
		return
	}
	prev := mt.SelectedNode
	mt.SelectedNode = ""
	mt.UnselectAll()
	if c := fyne.CurrentApp().Driver().CanvasForObject(mt); c != nil && c.Focused() == mt {
		c.Unfocus()
	}
	if prev != "" {
		mt.RefreshItem(prev)
	}
}

// selectPreservingScroll updates selection without ScrollTo jumping the viewport.
func (mt *MacroTree) selectPreservingScroll(uid string) {
	mt.Select(widget.TreeNodeID(uid))
}
