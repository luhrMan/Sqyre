package macro

import (
	"image/color"
	"math"
	"slices"
	"time"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/canvas"
)

func (mt *MacroTree) SetCursor(uid string) {
	old := mt.cursorUID
	if old == uid {
		return
	}
	mt.cursorUID = uid
	if old != "" {
		mt.refreshHighlightOverlay(old)
	}
	if uid != "" {
		if !mt.execFullyExpanded {
			mt.openAncestorBranches(uid)
		}
		mt.refreshHighlightOverlay(uid)
		targetUID := uid
		fyne.Do(func() {
			if mt.cursorUID == targetUID {
				mt.scrollToIfNeeded(targetUID)
			}
		})
	} else {
		mt.lastScrollUID = ""
	}
	if !mt.executing {
		mt.scheduleCollapseStale()
	}
}

// SetFill sets the horizontal fill fraction (0..1) for a container action and
// reveals it the first time. Must be called on the Fyne UI thread.
func (mt *MacroTree) SetFill(uid string, fraction float64) {
	if uid == "" {
		return
	}
	if mt.fills == nil {
		mt.fills = map[string]float64{}
	}
	prev, existed := mt.fills[uid]
	if existed && fillNearlyEqual(prev, fraction) {
		return
	}
	mt.fills[uid] = fraction
	if !existed {
		if !mt.execFullyExpanded {
			mt.openAncestorBranches(uid)
		}
		if !mt.executing {
			mt.scheduleCollapseStale()
		}
		targetUID := uid
		fyne.Do(func() {
			if _, ok := mt.fills[targetUID]; ok {
				mt.scrollToIfNeeded(targetUID)
			}
		})
	}
	mt.refreshHighlightOverlay(uid)
}

// ClearHighlight removes any highlight (fill or cursor) on a single action.
func (mt *MacroTree) ClearHighlight(uid string) {
	changed := false
	if _, ok := mt.fills[uid]; ok {
		delete(mt.fills, uid)
		changed = true
	}
	if mt.cursorUID == uid {
		mt.cursorUID = ""
		changed = true
	}
	if changed {
		mt.refreshHighlightOverlay(uid)
	}
	mt.stopCollapseDebounce()
	mt.collapseStaleBranches()
}

// ClearAllHighlights removes every execution highlight from the tree.
func (mt *MacroTree) ClearAllHighlights() {
	affected := make([]string, 0, len(mt.fills)+1)
	for k := range mt.fills {
		affected = append(affected, k)
	}
	if mt.cursorUID != "" {
		affected = append(affected, mt.cursorUID)
	}
	mt.fills = map[string]float64{}
	mt.cursorUID = ""
	mt.lastScrollUID = ""
	mt.execOpenedBranches = nil
	for _, k := range affected {
		mt.refreshHighlightOverlay(k)
	}
	mt.stopCollapseDebounce()
	mt.collapseStaleBranches()
}

// openAncestorBranches expands parent branches so uid is visible in the tree.
func (mt *MacroTree) openAncestorBranches(uid string) {
	if mt.Macro == nil || mt.Macro.Root == nil {
		return
	}
	node := mt.Macro.Root.GetAction(uid)
	if node == nil {
		return
	}
	rootUID := mt.Macro.Root.GetUID()
	var ancestors []string
	for p := node.GetParent(); p != nil && p.GetUID() != rootUID; p = p.GetParent() {
		ancestors = append(ancestors, p.GetUID())
	}
	mt.suppressBranchOpenScroll++
	defer func() { mt.suppressBranchOpenScroll-- }()
	for _, a := range slices.Backward(ancestors) {

		if !mt.IsBranchOpen(a) {
			mt.OpenBranch(a)
			mt.trackExecOpened(a)
		}
	}
}

// OpenAllBranches expands every branch in the macro tree.
func (mt *MacroTree) OpenAllBranches() {
	mt.stopCollapseDebounce()
	mt.execOpenedBranches = nil
	mt.suppressBranchOpenScroll++
	defer func() { mt.suppressBranchOpenScroll-- }()
	mt.Tree.OpenAllBranches()
}

// CloseAllBranches collapses every branch in the macro tree.
func (mt *MacroTree) CloseAllBranches() {
	mt.stopCollapseDebounce()
	mt.execOpenedBranches = nil
	mt.Tree.CloseAllBranches()
	mt.scheduleClampScroll()
}

// GoToAction selects uid, expands ancestor branches, and scrolls it into view.
func (mt *MacroTree) GoToAction(uid string) {
	if uid == "" || mt.Macro == nil || mt.Macro.Root == nil {
		return
	}
	if mt.Macro.Root.GetAction(uid) == nil {
		return
	}
	mt.Select(uid)
	mt.SelectedNode = uid
	mt.lastScrollUID = ""
	mt.revealNode(uid)
}

// revealNode expands ancestor branches and scrolls so uid is visible.
func (mt *MacroTree) revealNode(uid string) {
	mt.openAncestorBranches(uid)
	mt.scrollToIfNeeded(uid)
}

// ancestorUIDs returns parent branch UIDs from the macro root down to uid's parent.
func (mt *MacroTree) ancestorUIDs(uid string) []string {
	if mt.Macro == nil || mt.Macro.Root == nil || uid == "" {
		return nil
	}
	node := mt.Macro.Root.GetAction(uid)
	if node == nil {
		return nil
	}
	rootUID := mt.Macro.Root.GetUID()
	var ancestors []string
	for p := node.GetParent(); p != nil && p.GetUID() != rootUID; p = p.GetParent() {
		ancestors = append(ancestors, p.GetUID())
	}
	return ancestors
}

func fillNearlyEqual(a, b float64) bool {
	return math.Abs(a-b) < 0.001
}

func (mt *MacroTree) trackExecOpened(uid string) {
	if uid == "" {
		return
	}
	if mt.execOpenedBranches == nil {
		mt.execOpenedBranches = map[string]struct{}{}
	}
	mt.execOpenedBranches[uid] = struct{}{}
}

func (mt *MacroTree) markHighlightRefresh(uid string) {
	if uid == "" {
		return
	}
	if mt.highlightOnlyRefresh == nil {
		mt.highlightOnlyRefresh = map[string]struct{}{}
	}
	mt.highlightOnlyRefresh[uid] = struct{}{}
}

func (mt *MacroTree) consumeHighlightRefresh(uid string) bool {
	if mt.highlightOnlyRefresh == nil {
		return false
	}
	_, ok := mt.highlightOnlyRefresh[uid]
	if ok {
		delete(mt.highlightOnlyRefresh, uid)
	}
	return ok
}

func (mt *MacroTree) markNodeBound(obj fyne.CanvasObject, uid string) {
	if mt.nodeBoundUID == nil {
		mt.nodeBoundUID = map[fyne.CanvasObject]string{}
	}
	mt.nodeBoundUID[obj] = uid
}

func (mt *MacroTree) nodeObjectShowsUID(obj fyne.CanvasObject, uid string) bool {
	if mt.nodeBoundUID == nil {
		return false
	}
	return mt.nodeBoundUID[obj] == uid
}

func (mt *MacroTree) registerHighlightOverlay(uid string, stack *fyne.Container, hlBg *fyne.Container) {
	if mt.highlightOverlays == nil {
		mt.highlightOverlays = map[string]highlightRow{}
	}
	mt.highlightOverlays[uid] = highlightRow{stack: stack, hlBg: hlBg}
	mt.markNodeBound(stack, uid)
}

// refreshHighlightOverlay updates the execution highlight on uid when its row
// overlay is already bound, avoiding RefreshItem and tree relayout.
func (mt *MacroTree) refreshHighlightOverlay(uid string) {
	if uid == "" {
		return
	}
	if row, ok := mt.highlightOverlays[uid]; ok && mt.nodeObjectShowsUID(row.stack, uid) {
		mt.applyHighlightOverlay(uid, row.hlBg)
		return
	}
	mt.markHighlightRefresh(uid)
	mt.RefreshItem(uid)
}

func rgbaEqual(a, b color.Color) bool {
	if a == nil && b == nil {
		return true
	}
	if a == nil || b == nil {
		return false
	}
	ar, ag, ab, aa := a.RGBA()
	br, bg, bb, ba := b.RGBA()
	return ar == br && ag == bg && ab == bb && aa == ba
}

func (mt *MacroTree) applyHighlightOverlay(uid string, hlBg *fyne.Container) {
	fl := hlBg.Layout.(*fillLayout)
	hlSimple := hlBg.Objects[0].(*canvas.Rectangle)
	hlFill := hlBg.Objects[1].(*canvas.Rectangle)

	var wantFrac float64
	var wantSimpleVisible, wantFillVisible bool
	var wantSimpleColor color.Color

	switch {
	case mt.dragActive && uid == mt.dragSrcUID:
		wantSimpleVisible = true
		wantSimpleColor = dragSourceColor
	default:
		if frac, ok := mt.fills[uid]; ok {
			wantFrac = frac
			wantFillVisible = true
		} else if uid == mt.cursorUID {
			wantSimpleVisible = true
			wantSimpleColor = highlightSimpleColor
		}
	}

	simpleVisible := hlSimple.Visible()
	fillVisible := hlFill.Visible()
	if fl.fraction == wantFrac &&
		simpleVisible == wantSimpleVisible &&
		fillVisible == wantFillVisible &&
		(!wantSimpleVisible || rgbaEqual(hlSimple.FillColor, wantSimpleColor)) {
		return
	}

	fl.fraction = wantFrac
	if wantFillVisible {
		hlFill.Show()
		hlSimple.Hide()
	} else if wantSimpleVisible {
		hlSimple.FillColor = wantSimpleColor
		hlSimple.Show()
		hlFill.Hide()
	} else {
		hlSimple.Hide()
		hlFill.Hide()
	}
	hlSimple.Refresh()
	hlFill.Refresh()
}

func (mt *MacroTree) scheduleCollapseStale() {
	if mt.executing {
		return
	}
	if mt.collapseDebounce != nil {
		mt.collapseDebounce.Stop()
	}
	mt.collapseDebounce = time.AfterFunc(collapseDebounceMs*time.Millisecond, func() {
		fyne.Do(func() {
			mt.collapseDebounce = nil
			mt.collapseStaleBranches()
		})
	})
}

func (mt *MacroTree) stopCollapseDebounce() {
	if mt.collapseDebounce != nil {
		mt.collapseDebounce.Stop()
		mt.collapseDebounce = nil
	}
}

// branchesToKeepOpen returns branch UIDs that must stay expanded for the current
// cursor position and any in-progress container fill highlights.
func (mt *MacroTree) branchesToKeepOpen() map[string]bool {
	keep := map[string]bool{}
	addAncestors := func(uid string) {
		for _, a := range mt.ancestorUIDs(uid) {
			keep[a] = true
		}
	}
	if mt.cursorUID != "" {
		addAncestors(mt.cursorUID)
		if mt.IsBranch(mt.cursorUID) {
			keep[mt.cursorUID] = true
		}
	}
	for fillUID := range mt.fills {
		addAncestors(fillUID)
		if mt.IsBranch(fillUID) {
			keep[fillUID] = true
		}
	}
	return keep
}

// collapseStaleBranches closes branches opened during execution that no longer
// contain the active highlight.
func (mt *MacroTree) collapseStaleBranches() {
	if mt.executing || mt.execOpenedBranches == nil {
		return
	}
	keep := mt.branchesToKeepOpen()
	closed := false
	for uid := range mt.execOpenedBranches {
		if keep[uid] {
			continue
		}
		if mt.IsBranchOpen(uid) {
			mt.Tree.CloseBranch(uid)
			closed = true
		}
		mt.untrackExecOpenedBranch(uid)
	}
	if closed {
		mt.scheduleClampScroll()
	}
}

func (mt *MacroTree) untrackExecOpenedBranch(uid string) {
	if mt.execOpenedBranches == nil {
		return
	}
	for openUID := range mt.execOpenedBranches {
		if openUID == uid || mt.isDescendantOf(openUID, uid) {
			delete(mt.execOpenedBranches, openUID)
		}
	}
}

func (mt *MacroTree) isDescendantOf(childUID, ancestorUID string) bool {
	if mt.Macro == nil || mt.Macro.Root == nil || childUID == "" || ancestorUID == "" {
		return false
	}
	node := mt.Macro.Root.GetAction(childUID)
	if node == nil {
		return false
	}
	for p := node.GetParent(); p != nil; p = p.GetParent() {
		if p.GetUID() == ancestorUID {
			return true
		}
	}
	return false
}
