package macro

import (
	"image/color"
	"math"
	"time"

	"Sqyre/internal/models/actions"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/canvas"
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

// dropLineColor draws the insertion line shown between rows during a drag.
var dropLineColor = color.NRGBA{R: 60, G: 140, B: 255, A: 255}

// dropZoneColor fills a branch row when the drop will reparent into it.
var dropZoneColor = color.NRGBA{R: 60, G: 140, B: 255, A: 70}

// dragSourceColor tints the row of the action currently being dragged.
var dragSourceColor = color.NRGBA{R: 60, G: 140, B: 255, A: 90}

const dropLineThickness = 3

// autoExpandDelayMs is how long a drag must dwell over a collapsed branch before
// it is expanded.
const autoExpandDelayMs = 500

// Edge auto-scroll tuning: how often the scroll ticker fires and how many pixels
// each tick advances the viewport while the pointer sits in an edge margin.
const (
	autoScrollIntervalMs = 16
	autoScrollSpeed      = 8
)

// dragHandle is the grip control on the left of each tree row. Dragging it
// reorders the action within the macro tree. It is a leaf Draggable so the tree
// scroller does not consume the gesture.
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

// attachDropOverlay connects the drop indicator objects created by the tab
// content. The overlay shares the tree's coordinate space (both fill the same
// Stack), so indicator positions are computed in tree-local pixels.
func (mt *MacroTree) attachDropOverlay(overlay *fyne.Container, line, box *canvas.Rectangle) {
	mt.dropOverlay = overlay
	mt.dropLine = line
	mt.dropBox = box
}

// SetExecuting toggles whether a macro is running. Drag-and-drop is disabled
// while executing to avoid mutating the tree under the highlight cursor.
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

// visibleRowUIDs returns the flattened list of currently visible row UIDs in
// top-to-bottom order (open branches expanded, closed branches collapsed).
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

// dragMetrics returns the row height and the vertical pitch (row height plus
// inter-row padding) used to map pointer position to rows.
func (mt *MacroTree) dragMetrics() (rowH, pitch float32) {
	bH, lH := treeRowHeights(&mt.Tree)
	rowH = lH
	if bH > rowH {
		rowH = bH
	}
	pad := mt.Theme().Size(theme.SizeNamePadding)
	return rowH, rowH + pad
}

// beginDrag captures the fixed tree-top anchor from the first drag event. It
// returns false if the drag cannot start (macro running, source not visible).
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
	// The drag event position is relative to the handle, so subtracting it from
	// the absolute position yields the handle's top in canvas coordinates. The
	// handle is vertically centered in its row, so its center approximates the
	// source row's center. From that known row we back out the canvas Y of the
	// tree content's top (content Y 0 at scroll 0), which stays fixed for the
	// whole gesture and lets hit-testing stay correct as the tree scrolls.
	handleCenterY := (e.AbsolutePosition.Y - e.Position.Y) + handleH/2
	scroll0, _ := treeScrollOffsetY(&mt.Tree)
	mt.dragTreeTop = handleCenterY - float32(idx)*pitch - rowH/2 + scroll0

	mt.dragSrcUID = h.uid
	mt.dragVisible = vis
	mt.dragActive = true
	mt.dragLastPointerY = e.AbsolutePosition.Y

	// Tint the source row so the user can see which action is being dragged.
	mt.markHighlightRefresh(h.uid)
	mt.RefreshItem(h.uid)
	return true
}

// updateDrag handles a pointer move during a drag: it updates edge auto-scroll
// and re-resolves the drop target.
func (mt *MacroTree) updateDrag(e *fyne.DragEvent) {
	mt.dragLastPointerY = e.AbsolutePosition.Y
	mt.updateAutoScroll(e.AbsolutePosition.Y)
	mt.resolveDropAt(e.AbsolutePosition.Y)
}

// resolveDropAt maps a canvas-absolute pointer Y to a target row using the live
// scroll offset, then resolves the drop and redraws the indicator. Working in
// content coordinates keeps results correct across scrolling and auto-expand.
func (mt *MacroTree) resolveDropAt(pointerY float32) {
	n := len(mt.dragVisible)
	if n == 0 {
		return
	}
	rowH, pitch := mt.dragMetrics()
	if pitch <= 0 {
		return
	}
	scroll, _ := treeScrollOffsetY(&mt.Tree)
	contentY := pointerY - mt.dragTreeTop + scroll
	k := int(math.Floor(float64(contentY / pitch)))
	if k < 0 {
		k = 0
	}
	if k > n-1 {
		k = n - 1
	}
	centerK := float32(k)*pitch + rowH/2
	offset := contentY - centerK

	mt.resolveDrop(k, offset, rowH)
	mt.updateDropIndicator(k, rowH, pitch)
	mt.updateAutoExpand(k)
}

// updateAutoScroll starts, stops, or redirects edge auto-scroll based on whether
// the pointer is within the top or bottom margin of the viewport.
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

// setAutoScroll sets the scroll direction, launching or stopping the ticker
// goroutine as needed. Runs on the Fyne UI thread.
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
		return // ticker already running; it will read the new direction
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

// stopAutoScroll halts the ticker goroutine. Runs on the Fyne UI thread.
func (mt *MacroTree) stopAutoScroll() {
	if mt.autoScrollStop != nil {
		close(mt.autoScrollStop)
		mt.autoScrollStop = nil
	}
	mt.autoScrollDir = 0
}

// autoScrollStep advances the scroll offset one tick and re-resolves the drop
// target at the last known pointer position. Runs on the Fyne UI thread.
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
		return // already at the edge
	}
	mt.ScrollToOffset(newOff)
	mt.resolveDropAt(mt.dragLastPointerY)
}

// updateAutoExpand schedules expansion of the collapsed branch under the pointer
// (row k), cancelling any pending expansion when the hovered branch changes.
func (mt *MacroTree) updateAutoExpand(k int) {
	candidate := ""
	if k >= 0 && k < len(mt.dragVisible) {
		uid := mt.dragVisible[k]
		if mt.IsBranch(uid) && !mt.IsBranchOpen(uid) &&
			uid != mt.dragSrcUID && !mt.isDescendantOf(uid, mt.dragSrcUID) {
			candidate = uid
		}
	}
	if candidate == "" {
		mt.cancelAutoExpand()
		return
	}
	if mt.autoExpandUID == candidate {
		return
	}
	mt.cancelAutoExpand()
	mt.autoExpandUID = candidate
	uid := candidate
	mt.autoExpandTimer = time.AfterFunc(autoExpandDelayMs*time.Millisecond, func() {
		fyne.Do(func() {
			mt.doAutoExpand(uid)
		})
	})
}

// doAutoExpand opens the dwelled-on branch and rebuilds the visible row list.
// Hit-testing works in content coordinates, so the reflow from the inserted
// child rows is picked up automatically on the next resolve.
func (mt *MacroTree) doAutoExpand(uid string) {
	mt.autoExpandUID = ""
	mt.autoExpandTimer = nil
	if !mt.dragActive || !mt.IsBranch(uid) || mt.IsBranchOpen(uid) {
		return
	}
	mt.suppressBranchOpenScroll++
	mt.OpenBranch(uid)
	mt.suppressBranchOpenScroll--

	mt.dragVisible = mt.visibleRowUIDs()
	mt.resolveDropAt(mt.dragLastPointerY)
}

func (mt *MacroTree) cancelAutoExpand() {
	if mt.autoExpandTimer != nil {
		mt.autoExpandTimer.Stop()
		mt.autoExpandTimer = nil
	}
	mt.autoExpandUID = ""
}

// resolveDrop sets dropParent / dropTargetUID / dropMode / dropValid based on
// the target row k and the pointer's vertical offset from that row's center.
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
		// The visually-next row is this branch's first child, so anything below
		// the upper zone means "drop as first child".
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

// validateDrop rejects drops that would create a cycle (into the dragged node or
// its own subtree) or that target the dragged row itself.
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
	// Dropping a node immediately before/after itself is a no-op; treat it as
	// invalid so no indicator is shown.
	if (mt.dropMode == dropBefore || mt.dropMode == dropAfter) && mt.dropTargetUID == src {
		return
	}
	mt.dropValid = true
}

// updateDropIndicator positions the insertion line or reparent box in tree-local
// coordinates. localRowTop mirrors the scroll math in openTreeContentHeight
// (first row top at content Y 0).
func (mt *MacroTree) updateDropIndicator(k int, rowH, pitch float32) {
	if mt.dropOverlay == nil || mt.dropLine == nil || mt.dropBox == nil {
		return
	}
	if !mt.dropValid {
		mt.hideIndicators()
		return
	}
	scroll, _ := treeScrollOffsetY(&mt.Tree)
	localRowTop := float32(k)*pitch - scroll
	width := mt.Size().Width

	switch mt.dropMode {
	case dropBefore:
		mt.showLine(localRowTop-dropLineThickness/2, width)
	case dropAfter:
		mt.showLine(localRowTop+rowH-dropLineThickness/2, width)
	case dropIntoStart, dropIntoEnd:
		mt.showBox(localRowTop, rowH, width)
	default:
		mt.hideIndicators()
	}
}

func (mt *MacroTree) showLine(y, width float32) {
	mt.dropBox.Hide()
	mt.dropLine.FillColor = dropLineColor
	mt.dropLine.Move(fyne.NewPos(0, y))
	mt.dropLine.Resize(fyne.NewSize(width, dropLineThickness))
	mt.dropLine.Show()
	mt.dropOverlay.Refresh()
}

func (mt *MacroTree) showBox(y, rowH, width float32) {
	mt.dropLine.Hide()
	mt.dropBox.FillColor = dropZoneColor
	mt.dropBox.Move(fyne.NewPos(0, y))
	mt.dropBox.Resize(fyne.NewSize(width, rowH))
	mt.dropBox.Show()
	mt.dropOverlay.Refresh()
}

func (mt *MacroTree) hideIndicators() {
	if mt.dropLine != nil {
		mt.dropLine.Hide()
	}
	if mt.dropBox != nil {
		mt.dropBox.Hide()
	}
	if mt.dropOverlay != nil {
		mt.dropOverlay.Refresh()
	}
}

// endDrag commits the move (if valid), refreshes the tree, and clears drag
// state.
func (mt *MacroTree) endDrag() {
	if !mt.dragActive {
		return
	}
	mt.dragActive = false
	mt.cancelAutoExpand()
	mt.stopAutoScroll()
	mt.hideIndicators()

	src := mt.dragSrcUID

	if mt.dropValid && mt.dropParent != nil {
		if mt.performMove() {
			mt.flushNodeCache()
			mt.openAncestorBranches(mt.dragSrcUID)
			mt.Select(mt.dragSrcUID)
			mt.SelectedNode = mt.dragSrcUID
			if mt.OnTreeChanged != nil {
				mt.OnTreeChanged()
			}
		}
	}

	mt.dragSrcUID = ""
	mt.dropParent = nil
	mt.dropTargetUID = ""
	mt.dropMode = dropNone
	mt.dropValid = false
	mt.dragVisible = nil

	// Clear the source-row tint (no-op if a move already rebuilt the rows).
	if src != "" {
		mt.markHighlightRefresh(src)
		mt.RefreshItem(src)
	}
}

// performMove relocates the dragged action into dropParent at the resolved
// index. It returns false on a no-op or invalid move.
func (mt *MacroTree) performMove() bool {
	src := mt.dragSrcUID
	if (mt.dropMode == dropBefore || mt.dropMode == dropAfter) && mt.dropTargetUID == src {
		return false
	}
	node := mt.Macro.Root.GetAction(src)
	if node == nil {
		return false
	}
	oldParent := node.GetParent()
	if oldParent == nil {
		return false
	}
	parent := mt.dropParent

	mt.recordMutation()
	oldParent.RemoveSubAction(node)
	subs := parent.GetSubActions()

	index := len(subs)
	switch mt.dropMode {
	case dropIntoStart:
		index = 0
	case dropIntoEnd:
		index = len(subs)
	case dropBefore:
		if i := indexOfAction(subs, mt.dropTargetUID); i >= 0 {
			index = i
		}
	case dropAfter:
		if i := indexOfAction(subs, mt.dropTargetUID); i >= 0 {
			index = i + 1
		}
	}
	if index > len(subs) {
		index = len(subs)
	}
	if index < 0 {
		index = 0
	}

	newSubs := make([]actions.ActionInterface, 0, len(subs)+1)
	newSubs = append(newSubs, subs[:index]...)
	newSubs = append(newSubs, node)
	newSubs = append(newSubs, subs[index:]...)
	parent.SetSubActions(newSubs)
	node.SetParent(parent)
	return true
}

// flushNodeCacheSentinel is a transient Root value used to evict every cached
// tree row. It must never collide with a real action UID (a UUID).
const flushNodeCacheSentinel = "\x00sqyre-flush-node-cache\x00"

// flushNodeCache forces widget.Tree to discard its cached row objects so that
// reparented actions are rebuilt with the correct depth (indentation).
//
// Fyne's tree assigns a row's depth only when the row object is created on a
// cache miss; a reused row keeps its previous depth. After a reparent the moved
// subtree therefore renders at its old sub-level. Pointing Root at a sentinel
// and refreshing walks none of the real nodes, returning them all to the object
// pool; restoring Root rebuilds every visible node as a cache miss at its new
// depth. The two refreshes are synchronous, so only the final state is painted.
func (mt *MacroTree) flushNodeCache() {
	mt.suppressBranchOpenScroll++
	defer func() { mt.suppressBranchOpenScroll-- }()
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
