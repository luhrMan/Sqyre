package macro

import (
	"math"

	"Sqyre/internal/models/actions"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/driver/desktop"
	"fyne.io/fyne/v2/theme"
	"fyne.io/fyne/v2/widget"
)

// dropMode describes where a dragged action will be inserted relative to the
// resolved drop target.
type dropMode int

const (
	dropNone dropMode = iota
	dropBefore
	dropAfter
	dropIntoStart
	dropIntoEnd
)

// dragOrigin records where a dragged action lived when the gesture began.
type dragOrigin struct {
	parent actions.AdvancedActionInterface
	index  int
}

type dragHandle struct {
	widget.BaseWidget
	tree *MacroTree
	uid  string
	icon *widget.Icon
}

func newDragHandle() *dragHandle {
	h := &dragHandle{icon: widget.NewIcon(theme.MenuIcon())}
	h.ExtendBaseWidget(h)
	return h
}

func (h *dragHandle) CreateRenderer() fyne.WidgetRenderer {
	return widget.NewSimpleRenderer(h.icon)
}

func (h *dragHandle) MinSize() fyne.Size {
	return fyne.NewSize(macroTreeRowHeight, macroTreeRowHeight)
}

func (h *dragHandle) Cursor() desktop.Cursor {
	return desktop.PointerCursor
}

func (h *dragHandle) Dragged(e *fyne.DragEvent) {
	if h.tree == nil {
		return
	}
	if !h.tree.dragActive {
		if !h.tree.beginDrag(h, e) {
			return
		}
	}
	h.tree.updateDrag(e)
}

func (h *dragHandle) DragEnd() {
	if h.tree != nil {
		h.tree.endDrag()
	}
}

func (h *dragHandle) TappedSecondary(pe *fyne.PointEvent) {
	showMacroTreeActionContextMenu(h.tree, h, pe, h.uid)
}

var _ fyne.SecondaryTappable = (*dragHandle)(nil)

func (mt *MacroTree) SetExecuting(running bool) {
	if running && mt.dragActive {
		mt.endDrag()
	}
	if running {
		mt.executing = true
		mt.beginExecutionExpand()
		return
	}
	mt.executing = false
	mt.endExecutionExpand()
	mt.stopCollapseDebounce()
	mt.collapseStaleBranches()
}

func (mt *MacroTree) visibleRowUIDs() []string {
	if mt.Macro == nil || mt.Macro.Root == nil {
		return nil
	}
	var out []string
	var walk func(uid string)
	walk = func(uid string) {
		for _, c := range mt.ChildUIDs(uid) {
			out = append(out, c)
			if mt.IsBranch(c) && mt.IsBranchOpen(c) {
				walk(c)
			}
		}
	}
	walk(mt.Macro.Root.GetUID())
	return out
}

func (mt *MacroTree) dragMetrics() (rowH, pitch float32) {
	bH, lH := treeRowHeights(&mt.Tree)
	rowH = lH
	if bH > rowH {
		rowH = bH
	}
	pad := mt.Theme().Size(theme.SizeNamePadding)
	return rowH, rowH + pad
}

func (mt *MacroTree) beginDrag(h *dragHandle, e *fyne.DragEvent) bool {
	if mt.executing || h.uid == "" {
		return false
	}
	vis := mt.visibleRowUIDs()
	idx := indexOfString(vis, h.uid)
	if idx < 0 {
		return false
	}
	rowH, pitch := mt.dragMetrics()
	handleH := h.Size().Height
	if handleH <= 0 {
		handleH = rowH
	}
	handleCenterY := (e.AbsolutePosition.Y - e.Position.Y) + handleH/2
	scroll0, _ := treeScrollOffsetY(&mt.Tree)
	mt.dragTreeTop = handleCenterY - float32(idx)*pitch - rowH/2 + scroll0

	mt.dragSrcUID = h.uid
	mt.dragVisible = vis
	mt.dragActive = true
	dismissActiveActionTooltips()
	mt.dragLastPointerY = e.AbsolutePosition.Y
	mt.dropIndicatorKey = ""
	mt.dropGhostContentKey = ""
	mt.dragPreviewInTree = false
	mt.dragPreviewKey = ""
	mt.dragUndoSnapshotOK = false
	mt.cancelDragPreviewDebounce()
	mt.initDragBranchState()

	node := mt.Macro.Root.GetAction(h.uid)
	if node == nil {
		return false
	}
	if snap, err := snapshotTree(mt.Macro.Root, mt.SelectedNode); err != nil {
		mt.dragUndoSnapshotOK = false
	} else {
		mt.dragUndoSnapshot = snap
		mt.dragUndoSnapshotOK = true
	}
	mt.captureDragOrigin(node)
	mt.rebuildDropGhostRow(node)
	mt.markHighlightRefresh(h.uid)
	mt.RefreshItem(h.uid)
	return true
}

func (mt *MacroTree) updateDrag(e *fyne.DragEvent) {
	mt.dragLastPointerY = e.AbsolutePosition.Y
	mt.updateAutoScroll(e.AbsolutePosition.Y)
	mt.resolveDropAt(e.AbsolutePosition.Y)
}

func (mt *MacroTree) resolveDropAt(pointerY float32) {
	mt.dragVisible = mt.visibleRowUIDs()
	if len(mt.dragVisible) == 0 {
		return
	}
	rowH, pitch := mt.dragMetrics()
	if pitch <= 0 {
		return
	}
	scroll, _ := treeScrollOffsetY(&mt.Tree)
	contentY := pointerY - mt.dragTreeTop + scroll

	resolveAt := func() (k int) {
		n := len(mt.dragVisible)
		if n == 0 {
			return -1
		}
		k = min(max(int(math.Floor(float64(contentY/pitch))), 0), n-1)
		offset := contentY - (float32(k)*pitch + rowH/2)
		mt.resolveDrop(k, offset, rowH)
		if mt.shouldDropAtRootBelowLastBranch(k, offset, rowH) {
			mt.setDropRootAfterLastChild()
			mt.validateDrop()
		}
		return k
	}

	n := len(mt.dragVisible)
	if contentY >= float32(n)*pitch-rowH*0.25 {
		mt.setDropRootAfterLastChild()
		mt.validateDrop()
		if mt.syncDragAutoOpenedBranches(-1) {
			mt.dragVisible = mt.visibleRowUIDs()
		}
		mt.updateBranchOpenDebounce(-1)
		indicatorKey := mt.dropIndicatorKeyForState()
		if indicatorKey != mt.dropIndicatorKey {
			mt.dropIndicatorKey = indicatorKey
			mt.updateDropIndicator()
		}
		mt.scheduleDragPreview()
		return
	}

	k := resolveAt()
	if k < 0 {
		return
	}
	if mt.syncDragAutoOpenedBranches(k) {
		mt.dragVisible = mt.visibleRowUIDs()
		k = resolveAt()
		if k < 0 {
			return
		}
	}

	mt.updateBranchOpenDebounce(k)

	indicatorKey := mt.dropIndicatorKeyForState()
	if indicatorKey != mt.dropIndicatorKey {
		mt.dropIndicatorKey = indicatorKey
		mt.updateDropIndicator()
	}
	mt.scheduleDragPreview()
}

func (mt *MacroTree) setDropRootAfterLastChild() {
	if mt.Macro == nil || mt.Macro.Root == nil {
		return
	}
	subs := mt.Macro.Root.GetSubActions()
	if len(subs) == 0 {
		mt.setDropInto(mt.Macro.Root, dropIntoEnd)
		return
	}
	mt.setDropSibling(subs[len(subs)-1], dropAfter)
	mt.dropParent = mt.Macro.Root
}

func (mt *MacroTree) lastRootChildUID() string {
	if mt.Macro == nil || mt.Macro.Root == nil {
		return ""
	}
	subs := mt.Macro.Root.GetSubActions()
	if len(subs) == 0 {
		return mt.Macro.Root.GetUID()
	}
	return subs[len(subs)-1].GetUID()
}

func (mt *MacroTree) shouldDropAtRootBelowLastBranch(k int, offset, rowH float32) bool {
	if k != len(mt.dragVisible)-1 {
		return false
	}
	if offset <= rowH*0.25 {
		return false
	}
	lastRootUID := mt.lastRootChildUID()
	if lastRootUID == "" || !mt.IsBranch(lastRootUID) {
		return false
	}
	lastVisibleUID := mt.dragVisible[k]
	return lastVisibleUID == lastRootUID || mt.isDescendantOf(lastVisibleUID, lastRootUID)
}

func (mt *MacroTree) dropIndicatorKeyForState() string {
	if !mt.dropValid {
		return ""
	}
	return mt.dropFingerprint()
}

func (mt *MacroTree) resolveDrop(k int, offset, rowH float32) {
	mt.dropMode = dropNone
	mt.dropParent = nil
	mt.dropTargetUID = ""
	mt.dropValid = false

	if k < 0 || k >= len(mt.dragVisible) {
		return
	}
	targetUID := mt.dragVisible[k]
	node := mt.Macro.Root.GetAction(targetUID)
	if node == nil {
		return
	}
	upper := -rowH * 0.25
	lower := rowH * 0.25
	isBranch := mt.IsBranch(targetUID)
	isOpen := isBranch && mt.IsBranchOpen(targetUID)

	switch {
	case isBranch && isOpen:
		if offset < upper {
			mt.setDropSibling(node, dropBefore)
		} else {
			mt.setDropInto(node, dropIntoStart)
		}
	case isBranch && !isOpen:
		if offset < upper {
			mt.setDropSibling(node, dropBefore)
		} else if offset > lower {
			mt.setDropSibling(node, dropAfter)
		} else {
			mt.setDropInto(node, dropIntoEnd)
		}
	default:
		if offset < 0 {
			mt.setDropSibling(node, dropBefore)
		} else {
			mt.setDropSibling(node, dropAfter)
		}
	}
	mt.validateDrop()
}

func (mt *MacroTree) setDropSibling(node actions.ActionInterface, mode dropMode) {
	mt.dropMode = mode
	mt.dropTargetUID = node.GetUID()
	mt.dropParent = node.GetParent()
}

func (mt *MacroTree) setDropInto(node actions.ActionInterface, mode dropMode) {
	adv, ok := node.(actions.AdvancedActionInterface)
	if !ok {
		return
	}
	mt.dropMode = mode
	mt.dropTargetUID = node.GetUID()
	mt.dropParent = adv
}

func (mt *MacroTree) validateDrop() {
	mt.dropValid = false
	if mt.dropParent == nil || mt.dropMode == dropNone {
		return
	}
	src := mt.dragSrcUID
	pUID := mt.dropParent.GetUID()
	if pUID == src || mt.isDescendantOf(pUID, src) {
		return
	}
	if (mt.dropMode == dropBefore || mt.dropMode == dropAfter) && mt.dropTargetUID == src {
		return
	}
	mt.dropValid = true
}

func (mt *MacroTree) endDrag() {
	if !mt.dragActive {
		return
	}
	dropScrollY, dropScrollOK := treeScrollOffsetY(&mt.Tree)

	mt.cancelAutoExpand()
	mt.finishDragBranchState()
	mt.cancelDragPreviewDebounce()
	mt.stopAutoScroll()
	mt.hideDropIndicator()

	src := mt.dragSrcUID
	originParent := ""
	if mt.dragOrigin.parent != nil {
		originParent = mt.dragOrigin.parent.GetUID()
	}

	committed := false
	if mt.dropValid {
		key := mt.dropFingerprint()
		if mt.dragPreviewInTree && mt.dragPreviewKey == key {
			committed = true
		} else if mt.relocateDraggedNode(false) {
			newParentUID := mt.dropParentUID()
			mt.refreshAfterDragLayout(mt.dragMutationNeedsFlush(originParent, newParentUID))
			if src != "" {
				mt.invalidateRowCache(src)
			}
			committed = true
		} else if mt.dragPreviewInTree {
			mt.revertDragPreview()
		}
	} else if mt.dragPreviewInTree {
		mt.revertDragPreview()
	}

	if committed {
		mt.commitDragUndoSnapshot()
		mt.dragPreviewInTree = false
		mt.dragPreviewKey = ""
		if node := mt.Macro.Root.GetAction(src); node != nil && mt.dragMutationNeedsFlush(mt.dragOriginParentUID(), mt.draggedNodeParentUID()) {
			mt.refreshAfterDragLayout(true)
		}
		mt.openAncestorBranches(src)
		mt.Tree.Select(widget.TreeNodeID(src))
		mt.SelectedNode = src
		if mt.OnTreeChanged != nil {
			mt.OnTreeChanged()
		}
	}

	mt.dragActive = false

	mt.dragSrcUID = ""
	mt.dropParent = nil
	mt.dropTargetUID = ""
	mt.dropMode = dropNone
	mt.dropValid = false
	mt.dragVisible = nil
	mt.dropIndicatorKey = ""
	mt.dropGhostContentKey = ""
	mt.dragPreviewInTree = false
	mt.dragPreviewKey = ""
	mt.dragUndoSnapshotOK = false

	if src != "" {
		mt.withPreservedScroll(func() {
			mt.markHighlightRefresh(src)
			mt.RefreshItem(src)
		})
	}
	if dropScrollOK {
		mt.restoreScrollOffset(dropScrollY)
	}
}

func (mt *MacroTree) commitDragUndoSnapshot() {
	if mt.applyingHistory || !mt.dragUndoSnapshotOK {
		return
	}
	if mt.history == nil {
		mt.history = newTreeHistory()
	}
	mt.history.pushSnapshot(mt.dragUndoSnapshot)
	mt.notifyHistoryChanged()
}

func (mt *MacroTree) performMove() bool {
	return mt.relocateDraggedNode(true)
}

func (mt *MacroTree) relocateDraggedNode(record bool) bool {
	src := mt.dragSrcUID
	if (mt.dropMode == dropBefore || mt.dropMode == dropAfter) && mt.dropTargetUID == src {
		return false
	}
	node := mt.Macro.Root.GetAction(src)
	if node == nil {
		return false
	}
	if record {
		mt.recordMutation()
	}
	oldParent := node.GetParent()
	if oldParent != nil {
		oldParent.RemoveSubAction(node)
	}
	parent := mt.dropParent
	if parent == nil {
		return false
	}

	subs := parent.GetSubActions()
	index := max(min(mt.dropInsertIndex(subs), len(subs)), 0)
	newSubs := make([]actions.ActionInterface, 0, len(subs)+1)
	newSubs = append(newSubs, subs[:index]...)
	newSubs = append(newSubs, node)
	newSubs = append(newSubs, subs[index:]...)
	parent.SetSubActions(newSubs)
	node.SetParent(parent)
	return true
}

const flushNodeCacheSentinel = "\x00sqyre-flush-node-cache\x00"

// flushNodeCache forces Fyne's tree to drop its internal node widget cache by
// briefly swapping Root to a sentinel. Two Refresh passes are required: the
// first binds rows against the sentinel (invalidating stale nodes), the second
// restores the real root and re-binds visible rows.
func (mt *MacroTree) flushNodeCache() {
	mt.suppressBranchOpenScroll++
	defer func() { mt.suppressBranchOpenScroll-- }()
	mt.clearRowCache()
	mt.Tree.Root = flushNodeCacheSentinel
	mt.Refresh()
	mt.Tree.Root = ""
	mt.Refresh()
}

func indexOfString(s []string, v string) int {
	for i, x := range s {
		if x == v {
			return i
		}
	}
	return -1
}

func indexOfAction(subs []actions.ActionInterface, uid string) int {
	for i, a := range subs {
		if a.GetUID() == uid {
			return i
		}
	}
	return -1
}
