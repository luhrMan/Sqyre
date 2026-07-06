package macro

import (
	"fmt"
	"image/color"
	"math"
	"time"

	"Sqyre/internal/config"
	"Sqyre/internal/models/actions"
	"Sqyre/ui/actiondisplay"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/canvas"
	"fyne.io/fyne/v2/container"
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

// dropGhostColor fills the overlay preview at the resolved drop slot.
var dropGhostColor = color.NRGBA{R: 60, G: 140, B: 255, A: 50}

// dropGhostStrokeColor outlines the placement preview row.
var dropGhostStrokeColor = color.NRGBA{R: 60, G: 140, B: 255, A: 200}

// dragSourceColor tints the row being dragged at its original position.
var dragSourceColor = color.NRGBA{R: 60, G: 140, B: 255, A: 90}

const branchOpenDebounceMs = 200

// dragOrigin records where a dragged action lived when the gesture began.
type dragOrigin struct {
	parent actions.AdvancedActionInterface
	index  int
}

// Edge auto-scroll tuning.
const (
	autoScrollIntervalMs = 16
	autoScrollSpeed      = 8
)

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

func (mt *MacroTree) attachDropOverlay(overlay *fyne.Container, ghost *fyne.Container, ghostInset *canvas.Rectangle, ghostRow *fyne.Container) {
	mt.dropOverlay = overlay
	mt.dropGhost = ghost
	mt.dropGhostInset = ghostInset
	mt.dropGhostRow = ghostRow
}

func newDropGhostShell() (ghost *fyne.Container, inset *canvas.Rectangle, row *fyne.Container) {
	bg := canvas.NewRectangle(dropGhostColor)
	bg.CornerRadius = 6
	bg.StrokeColor = dropGhostStrokeColor
	bg.StrokeWidth = 1
	inset = canvas.NewRectangle(color.Transparent)
	row = container.NewHBox()
	inner := container.NewBorder(nil, nil, inset, nil, row)
	ghost = container.NewStack(bg, inner)
	ghost.Hide()
	return ghost, inset, row
}

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

func (mt *MacroTree) branchOpenCandidate(k int) string {
	if (mt.dropMode == dropIntoStart || mt.dropMode == dropIntoEnd) &&
		mt.IsBranch(mt.dropTargetUID) && !mt.IsBranchOpen(mt.dropTargetUID) &&
		mt.dropTargetUID != mt.dragSrcUID && !mt.isDescendantOf(mt.dropTargetUID, mt.dragSrcUID) {
		return mt.dropTargetUID
	}
	if k >= 0 && k < len(mt.dragVisible) {
		uid := mt.dragVisible[k]
		if mt.IsBranch(uid) && !mt.IsBranchOpen(uid) &&
			uid != mt.dragSrcUID && !mt.isDescendantOf(uid, mt.dragSrcUID) {
			return uid
		}
	}
	return ""
}

func (mt *MacroTree) updateBranchOpenDebounce(k int) {
	uid := mt.branchOpenCandidate(k)
	if uid == "" {
		mt.cancelAutoExpand()
		return
	}
	mt.cancelAutoExpand()
	mt.autoExpandUID = uid
	mt.autoExpandTimer = time.AfterFunc(branchOpenDebounceMs*time.Millisecond, func() {
		fyne.Do(func() {
			mt.doAutoExpand(uid)
		})
	})
}

func (mt *MacroTree) dropIndicatorKeyForState() string {
	if !mt.dropValid {
		return ""
	}
	return mt.dropFingerprint()
}

func (mt *MacroTree) updateAutoScroll(pointerY float32) {
	viewH := mt.Size().Height
	if viewH <= 0 {
		mt.setAutoScroll(0)
		return
	}
	rowH, _ := mt.dragMetrics()
	margin := rowH
	top := mt.dragTreeTop
	switch {
	case pointerY < top+margin:
		mt.setAutoScroll(-1)
	case pointerY > top+viewH-margin:
		mt.setAutoScroll(1)
	default:
		mt.setAutoScroll(0)
	}
}

func (mt *MacroTree) setAutoScroll(dir int) {
	if mt.autoScrollDir == dir {
		return
	}
	mt.autoScrollDir = dir
	if dir == 0 {
		mt.stopAutoScroll()
		return
	}
	if mt.autoScrollStop != nil {
		return
	}
	stop := make(chan struct{})
	mt.autoScrollStop = stop
	go func() {
		ticker := time.NewTicker(autoScrollIntervalMs * time.Millisecond)
		defer ticker.Stop()
		for {
			select {
			case <-stop:
				return
			case <-ticker.C:
				fyne.Do(mt.autoScrollStep)
			}
		}
	}()
}

func (mt *MacroTree) stopAutoScroll() {
	if mt.autoScrollStop != nil {
		close(mt.autoScrollStop)
		mt.autoScrollStop = nil
	}
	mt.autoScrollDir = 0
}

func (mt *MacroTree) autoScrollStep() {
	if !mt.dragActive || mt.autoScrollDir == 0 {
		return
	}
	scroll, ok := treeScrollOffsetY(&mt.Tree)
	if !ok {
		return
	}
	maxOff := mt.openTreeContentHeight() - mt.Size().Height
	if maxOff < 0 {
		maxOff = 0
	}
	newOff := scroll + float32(mt.autoScrollDir)*autoScrollSpeed
	if newOff < 0 {
		newOff = 0
	}
	if newOff > maxOff {
		newOff = maxOff
	}
	if newOff == scroll {
		return
	}
	mt.ScrollToOffset(newOff)
	mt.resolveDropAt(mt.dragLastPointerY)
}

func (mt *MacroTree) doAutoExpand(uid string) {
	mt.autoExpandUID = ""
	mt.autoExpandTimer = nil
	if !mt.dragActive || !mt.IsBranch(uid) || mt.IsBranchOpen(uid) {
		return
	}
	mt.suppressBranchOpenScroll++
	mt.OpenBranch(uid)
	mt.suppressBranchOpenScroll--
	if !mt.wasOpenAtDragStart(uid) {
		if mt.dragAutoOpenedBranches == nil {
			mt.dragAutoOpenedBranches = map[string]struct{}{}
		}
		mt.dragAutoOpenedBranches[uid] = struct{}{}
	}
	mt.dragVisible = mt.visibleRowUIDs()
	mt.dropIndicatorKey = "" // branch open changes preview slot
	mt.updateDropIndicator()
	mt.resolveDropAt(mt.dragLastPointerY)
	mt.scheduleDragPreview()
}

func (mt *MacroTree) cancelAutoExpand() {
	if mt.autoExpandTimer != nil {
		mt.autoExpandTimer.Stop()
		mt.autoExpandTimer = nil
	}
	mt.autoExpandUID = ""
}

func (mt *MacroTree) initDragBranchState() {
	mt.dragStartOpenBranches = map[string]struct{}{}
	for _, uid := range mt.collectOpenBranchUIDs() {
		mt.dragStartOpenBranches[uid] = struct{}{}
	}
	mt.dragAutoOpenedBranches = nil
}

func (mt *MacroTree) finishDragBranchState() {
	mt.collapseDragAutoOpenedBranchesExcept(mt.dragBranchesToKeepAfterDrop())
	mt.dragStartOpenBranches = nil
	mt.dragAutoOpenedBranches = nil
}

func (mt *MacroTree) dragBranchesToKeepAfterDrop() map[string]struct{} {
	keep := map[string]struct{}{}
	if !mt.dropValid || mt.dragAutoOpenedBranches == nil {
		return keep
	}
	for uid := range mt.dragAutoOpenedBranches {
		if mt.dropMode == dropIntoStart || mt.dropMode == dropIntoEnd {
			if mt.dropTargetUID == uid {
				keep[uid] = struct{}{}
			}
		}
		if mt.dropParent != nil {
			pUID := mt.dropParent.GetUID()
			if pUID == uid || mt.isDescendantOf(pUID, uid) {
				keep[uid] = struct{}{}
			}
		}
	}
	return keep
}

func (mt *MacroTree) wasOpenAtDragStart(uid string) bool {
	if mt.dragStartOpenBranches == nil {
		return mt.IsBranchOpen(uid)
	}
	_, ok := mt.dragStartOpenBranches[uid]
	return ok
}

func (mt *MacroTree) dragBranchesToKeepOpen(k int) map[string]struct{} {
	keep := map[string]struct{}{}
	if mt.dragAutoOpenedBranches == nil {
		if cand := mt.branchOpenCandidate(k); cand != "" {
			keep[cand] = struct{}{}
		}
		if mt.autoExpandUID != "" {
			keep[mt.autoExpandUID] = struct{}{}
		}
		return keep
	}

	var rowUID string
	if k >= 0 && k < len(mt.dragVisible) {
		rowUID = mt.dragVisible[k]
	}

	for uid := range mt.dragAutoOpenedBranches {
		if rowUID != "" && (rowUID == uid || mt.isDescendantOf(rowUID, uid)) {
			keep[uid] = struct{}{}
		}
		if mt.dropParent != nil {
			pUID := mt.dropParent.GetUID()
			if pUID == uid || mt.isDescendantOf(pUID, uid) {
				keep[uid] = struct{}{}
			}
		}
		if mt.dropTargetUID != "" && (mt.dropTargetUID == uid || mt.isDescendantOf(mt.dropTargetUID, uid)) {
			keep[uid] = struct{}{}
		}
		if (mt.dropMode == dropIntoStart || mt.dropMode == dropIntoEnd) && mt.dropTargetUID == uid {
			keep[uid] = struct{}{}
		}
	}

	if cand := mt.branchOpenCandidate(k); cand != "" {
		keep[cand] = struct{}{}
	}
	if mt.autoExpandUID != "" {
		keep[mt.autoExpandUID] = struct{}{}
	}
	return keep
}

func (mt *MacroTree) syncDragAutoOpenedBranches(k int) bool {
	if mt.dragAutoOpenedBranches == nil {
		return false
	}
	return mt.collapseDragAutoOpenedBranchesExcept(mt.dragBranchesToKeepOpen(k))
}

func (mt *MacroTree) collapseDragAutoOpenedBranchesExcept(keep map[string]struct{}) bool {
	if mt.dragAutoOpenedBranches == nil {
		return false
	}
	changed := false
	for uid := range mt.dragAutoOpenedBranches {
		if keep != nil {
			if _, ok := keep[uid]; ok {
				continue
			}
		}
		if mt.IsBranchOpen(uid) {
			mt.suppressBranchOpenScroll++
			mt.CloseBranch(uid)
			mt.suppressBranchOpenScroll--
			changed = true
		}
		delete(mt.dragAutoOpenedBranches, uid)
	}
	if mt.autoExpandUID != "" {
		if keep == nil {
			mt.cancelAutoExpand()
		} else if _, ok := keep[mt.autoExpandUID]; !ok {
			mt.cancelAutoExpand()
		}
	}
	return changed
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

// updateDropIndicator moves a lightweight overlay ghost to the slot where the
// action would be inserted. After the debounced live preview applies the ghost
// is hidden because the real row occupies that slot.
func (mt *MacroTree) updateDropIndicator() {
	if mt.dropGhost == nil {
		return
	}
	if !mt.dropValid {
		mt.hideDropIndicator()
		return
	}
	if mt.dragPreviewInTree && mt.dragPreviewKey == mt.dropFingerprint() {
		mt.hideDropIndicator()
		return
	}
	y, depth, isBranch, ok := mt.dropGhostGeometry()
	if !ok {
		mt.hideDropIndicator()
		return
	}
	key := mt.dropFingerprint()
	if key != mt.dropGhostContentKey {
		mt.dropGhostContentKey = key
		if node := mt.Macro.Root.GetAction(mt.dragSrcUID); node != nil {
			mt.rebuildDropGhostRow(node)
		}
	}
	rowH, _ := mt.dragMetrics()
	mt.showDropGhost(y, rowH, mt.Size().Width, mt.rowContentLeftInset(depth, isBranch))
}

func (mt *MacroTree) dropFingerprint() string {
	if mt.dropParent == nil {
		return ""
	}
	return fmt.Sprintf("%s|%s|%d", mt.dropParent.GetUID(), mt.dropTargetUID, mt.dropMode)
}

func (mt *MacroTree) previewVisibleRowUIDs() []string {
	vis := mt.dragVisible
	if len(vis) == 0 {
		vis = mt.visibleRowUIDs()
	}
	src := mt.dragSrcUID
	filtered := make([]string, 0, len(vis))
	for _, uid := range vis {
		if uid != src {
			filtered = append(filtered, uid)
		}
	}
	insertAt := mt.previewInsertIndex(filtered)
	out := make([]string, 0, len(filtered)+1)
	out = append(out, filtered[:insertAt]...)
	out = append(out, src)
	out = append(out, filtered[insertAt:]...)
	return out
}

func (mt *MacroTree) previewInsertIndex(vis []string) int {
	switch mt.dropMode {
	case dropBefore:
		if i := indexOfString(vis, mt.dropTargetUID); i >= 0 {
			return i
		}
	case dropAfter:
		if i := indexOfString(vis, mt.dropTargetUID); i >= 0 {
			if mt.IsBranch(mt.dropTargetUID) && mt.IsBranchOpen(mt.dropTargetUID) {
				return mt.lastVisibleDescendantIndexInList(vis, mt.dropTargetUID) + 1
			}
			return i + 1
		}
	case dropIntoStart, dropIntoEnd:
		if mt.IsBranch(mt.dropTargetUID) && !mt.IsBranchOpen(mt.dropTargetUID) {
			if i := indexOfString(vis, mt.dropTargetUID); i >= 0 {
				return i
			}
		}
		if mt.dropMode == dropIntoStart {
			if i := indexOfString(vis, mt.dropTargetUID); i >= 0 {
				return i + 1
			}
		}
		if i := indexOfString(vis, mt.dropTargetUID); i >= 0 {
			return mt.lastVisibleDescendantIndexInList(vis, mt.dropTargetUID) + 1
		}
	}
	return len(vis)
}

func (mt *MacroTree) lastVisibleDescendantIndexInList(vis []string, branchUID string) int {
	idx := indexOfString(vis, branchUID)
	if idx < 0 {
		return idx
	}
	last := idx
	for i := idx + 1; i < len(vis); i++ {
		if mt.isDescendantOf(vis[i], branchUID) {
			last = i
			continue
		}
		break
	}
	return last
}

func (mt *MacroTree) dropGhostGeometry() (y float32, depth int, isBranch bool, ok bool) {
	rowH, pitch := mt.dragMetrics()
	scroll, scrollOK := treeScrollOffsetY(&mt.Tree)
	if pitch <= 0 || !scrollOK {
		return 0, 0, false, false
	}
	src := mt.Macro.Root.GetAction(mt.dragSrcUID)
	if src == nil {
		return 0, 0, false, false
	}
	_, isBranch = src.(actions.AdvancedActionInterface)

	if mt.dropMode == dropIntoStart || mt.dropMode == dropIntoEnd {
		if mt.IsBranch(mt.dropTargetUID) && !mt.IsBranchOpen(mt.dropTargetUID) {
			k := indexOfString(mt.dragVisible, mt.dropTargetUID)
			if k < 0 {
				return 0, 0, false, false
			}
			return float32(k)*pitch - scroll, mt.rowIndentDepth(mt.dropTargetUID) + 1, isBranch, true
		}
	}

	preview := mt.previewVisibleRowUIDs()
	slot := indexOfString(preview, mt.dragSrcUID)
	if slot < 0 {
		return 0, 0, false, false
	}
	depth = mt.insertIndentDepth()
	_ = rowH
	return float32(slot)*pitch - scroll, depth, isBranch, true
}

func (mt *MacroTree) rowIndentDepth(uid string) int {
	node := mt.Macro.Root.GetAction(uid)
	if node == nil || mt.Macro == nil || mt.Macro.Root == nil {
		return 0
	}
	rootUID := mt.Macro.Root.GetUID()
	depth := 0
	for p := node.GetParent(); p != nil; p = p.GetParent() {
		if p.GetUID() == rootUID {
			break
		}
		depth++
	}
	return depth
}

func (mt *MacroTree) rowContentLeftInset(depth int, isBranch bool) float32 {
	th := mt.Theme()
	pad := th.Size(theme.SizeNamePadding)
	iconSize := th.Size(theme.SizeNameInlineIcon)
	unit := iconSize + pad
	x := pad + float32(depth)*unit
	if isBranch {
		x += iconSize + pad
	}
	return x
}

func (mt *MacroTree) rebuildDropGhostRow(node actions.ActionInterface) {
	if mt.dropGhostRow == nil {
		return
	}
	iconBg := canvas.NewRectangle(macroTreeActionColor(node))
	iconBg.CornerRadius = 6
	iconBg.StrokeColor = theme.Color(theme.ColorNameShadow)
	iconBg.StrokeWidth = 1
	iconBg.SetMinSize(fyne.NewSize(treeItemIconSize, treeItemIconSize))
	iconBtn := widget.NewIcon(actiondisplay.Icon(node))
	iconStack := container.NewStack(iconBg, iconBtn)
	display := actionDisplay(node, actionDisplayHandlers{})
	mt.dropGhostRow.Objects = []fyne.CanvasObject{iconStack, display}
	mt.dropGhostRow.Refresh()
}

func (mt *MacroTree) showDropGhost(y, rowH, width, insetX float32) {
	if mt.dropGhost == nil {
		return
	}
	if mt.dropGhostInset != nil {
		mt.dropGhostInset.SetMinSize(fyne.NewSize(insetX, rowH))
	}
	mt.dropGhost.Move(fyne.NewPos(0, y))
	mt.dropGhost.Resize(fyne.NewSize(width, rowH))
	mt.dropGhost.Show()
	if mt.dropOverlay != nil {
		mt.dropOverlay.Refresh()
	}
}

func (mt *MacroTree) hideDropIndicator() {
	if mt.dropGhost != nil {
		mt.dropGhost.Hide()
	}
	if mt.dropOverlay != nil {
		mt.dropOverlay.Refresh()
	}
}

func (mt *MacroTree) cancelDragPreviewDebounce() {
	if mt.dragPreviewTimer != nil {
		mt.dragPreviewTimer.Stop()
		mt.dragPreviewTimer = nil
	}
}

func (mt *MacroTree) scheduleDragPreview() {
	if !mt.dragActive {
		return
	}
	if mt.dragPreviewInTree && mt.dropValid && mt.dragPreviewKey == mt.dropFingerprint() {
		return
	}
	mt.cancelDragPreviewDebounce()
	if !mt.dropValid {
		if mt.dragPreviewInTree {
			mt.revertDragPreview()
		}
		return
	}
	key := mt.dropFingerprint()
	if mt.dragPreviewInTree && mt.dragPreviewKey != key {
		mt.revertDragPreview()
	}
	scheduled := key
	debounce := mt.dragPreviewDebounceDuration()
	mt.dragPreviewTimer = time.AfterFunc(debounce, func() {
		fyne.Do(func() {
			mt.applyDragPreview(scheduled)
		})
	})
}

func (mt *MacroTree) dragPreviewDebounceDuration() time.Duration {
	app := fyne.CurrentApp()
	if app == nil {
		return time.Duration(config.DefaultDragPreviewDebounceMs) * time.Millisecond
	}
	ms := app.Preferences().IntWithFallback(config.PrefDragPreviewDebounceMs, config.DefaultDragPreviewDebounceMs)
	if ms < config.MinDragPreviewDebounceMs {
		ms = config.DefaultDragPreviewDebounceMs
	}
	return time.Duration(ms) * time.Millisecond
}

func (mt *MacroTree) applyDragPreview(key string) {
	mt.dragPreviewTimer = nil
	if !mt.dragActive || !mt.dropValid || mt.dropFingerprint() != key {
		return
	}
	if mt.dragPreviewInTree && mt.dragPreviewKey == key {
		return
	}
	prevParentUID := mt.draggedNodeParentUID()
	if !mt.relocateDraggedNode(false) {
		return
	}
	newParentUID := mt.dropParentUID()
	mt.dragPreviewInTree = true
	mt.dragPreviewKey = key
	mt.refreshAfterDragLayout(mt.dragMutationNeedsFlush(prevParentUID, newParentUID))
	if mt.dragSrcUID != "" {
		mt.withPreservedScroll(func() {
			mt.invalidateRowCache(mt.dragSrcUID)
			mt.RefreshItem(mt.dragSrcUID)
		})
	}
	mt.updateDropIndicator()
}

func (mt *MacroTree) restoreDragOrigin() bool {
	node := mt.Macro.Root.GetAction(mt.dragSrcUID)
	if node == nil || mt.dragOrigin.parent == nil {
		return false
	}
	if cur := node.GetParent(); cur != nil {
		cur.RemoveSubAction(node)
	}
	subs := mt.dragOrigin.parent.GetSubActions()
	idx := min(max(mt.dragOrigin.index, 0), len(subs))
	newSubs := make([]actions.ActionInterface, 0, len(subs)+1)
	newSubs = append(newSubs, subs[:idx]...)
	newSubs = append(newSubs, node)
	newSubs = append(newSubs, subs[idx:]...)
	mt.dragOrigin.parent.SetSubActions(newSubs)
	node.SetParent(mt.dragOrigin.parent)
	return true
}

func (mt *MacroTree) revertDragPreview() {
	if !mt.dragPreviewInTree {
		return
	}
	prevParentUID := mt.draggedNodeParentUID()
	originParentUID := mt.dragOriginParentUID()
	mt.restoreDragOrigin()
	mt.dragPreviewInTree = false
	mt.dragPreviewKey = ""
	mt.refreshAfterDragLayout(mt.dragMutationNeedsFlush(prevParentUID, originParentUID))
	if mt.dragSrcUID != "" {
		mt.withPreservedScroll(func() {
			mt.invalidateRowCache(mt.dragSrcUID)
			mt.RefreshItem(mt.dragSrcUID)
		})
	}
}

func (mt *MacroTree) captureDragOrigin(node actions.ActionInterface) {
	parent := node.GetParent()
	if parent == nil {
		return
	}
	mt.dragOrigin = dragOrigin{
		parent: parent,
		index:  indexOfAction(parent.GetSubActions(), node.GetUID()),
	}
}

func (mt *MacroTree) draggedNodeParentUID() string {
	node := mt.Macro.Root.GetAction(mt.dragSrcUID)
	if node == nil {
		return ""
	}
	if p := node.GetParent(); p != nil {
		return p.GetUID()
	}
	return ""
}

func (mt *MacroTree) dragOriginParentUID() string {
	if mt.dragOrigin.parent == nil {
		return ""
	}
	return mt.dragOrigin.parent.GetUID()
}

func (mt *MacroTree) dropParentUID() string {
	if mt.dropParent == nil {
		return ""
	}
	return mt.dropParent.GetUID()
}

// dragMutationNeedsFlush reports whether the Fyne tree must rebuild row depth
// after a drag mutation. Refresh alone is enough when only sibling order changes
// under the same parent at the same indent level.
func (mt *MacroTree) dragMutationNeedsFlush(prevParentUID, newParentUID string) bool {
	if prevParentUID != newParentUID {
		return true
	}
	originUID := mt.dragOriginParentUID()
	if originUID != "" && originUID != newParentUID {
		return true
	}
	return mt.childIndentDepthForParentUID(prevParentUID) != mt.childIndentDepthForParentUID(newParentUID)
}

func (mt *MacroTree) childIndentDepthForParentUID(parentUID string) int {
	if mt.Macro == nil || mt.Macro.Root == nil || parentUID == "" {
		return 0
	}
	if parentUID == mt.Macro.Root.GetUID() {
		return 0
	}
	return mt.rowIndentDepth(parentUID) + 1
}

func (mt *MacroTree) childIndentDepth(parent actions.AdvancedActionInterface) int {
	if parent == nil || mt.Macro == nil || mt.Macro.Root == nil {
		return 0
	}
	return mt.childIndentDepthForParentUID(parent.GetUID())
}

func (mt *MacroTree) insertIndentDepth() int {
	if mt.dropParent == nil {
		return 0
	}
	switch mt.dropMode {
	case dropIntoStart, dropIntoEnd:
		if mt.IsBranch(mt.dropTargetUID) && !mt.IsBranchOpen(mt.dropTargetUID) {
			return mt.rowIndentDepth(mt.dropTargetUID) + 1
		}
		return mt.childIndentDepth(mt.dropParent)
	default:
		return mt.childIndentDepth(mt.dropParent)
	}
}

func (mt *MacroTree) dropInsertIndex(subs []actions.ActionInterface) int {
	switch mt.dropMode {
	case dropIntoStart:
		return 0
	case dropIntoEnd:
		return len(subs)
	case dropBefore:
		if i := indexOfAction(subs, mt.dropTargetUID); i >= 0 {
			return i
		}
	case dropAfter:
		if i := indexOfAction(subs, mt.dropTargetUID); i >= 0 {
			return i + 1
		}
	}
	return len(subs)
}

func (mt *MacroTree) dragLayoutNeedsFlush(prevParentUID, newParentUID string) bool {
	return mt.dragMutationNeedsFlush(prevParentUID, newParentUID)
}

func (mt *MacroTree) refreshAfterDragLayout(flushDepth bool) {
	mt.withPreservedScroll(func() {
		mt.suppressBranchOpenScroll++
		if flushDepth {
			mt.flushNodeCache()
		} else {
			mt.Refresh()
		}
		mt.suppressBranchOpenScroll--
	})
	mt.dragVisible = mt.visibleRowUIDs()
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
