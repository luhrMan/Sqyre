package macro

import (
	"reflect"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/theme"
	"fyne.io/fyne/v2/widget"
)

const macroTreeRowHeight = 24

func clampedScrollY(contentH, viewH, currentY float32) float32 {
	maxOffset := contentH - viewH
	if maxOffset < 0 {
		maxOffset = 0
	}
	if currentY > maxOffset {
		return maxOffset
	}
	return currentY
}

// scheduleClampScroll runs clampScrollOffset on the next UI frame after layout.
func (mt *MacroTree) scheduleClampScroll() {
	if mt.dragActive {
		return
	}
	fyne.Do(func() {
		if mt.dragActive {
			return
		}
		mt.clampScrollOffset()
	})
}

// withPreservedScroll runs fn and restores the tree scroll offset afterward.
func (mt *MacroTree) withPreservedScroll(fn func()) {
	scrollY, ok := treeScrollOffsetY(&mt.Tree)
	fn()
	if ok {
		mt.ScrollToOffset(scrollY)
	}
}

// restoreScrollOffset sets the tree scroll position now and again on the next
// frame so deferred layout handlers cannot override a drop-time restore.
func (mt *MacroTree) restoreScrollOffset(scrollY float32) {
	mt.ScrollToOffset(scrollY)
	fyne.Do(func() {
		mt.ScrollToOffset(scrollY)
	})
}

// scrollToIfNeeded scrolls when uid is not already in the tree viewport.
// ScrollTo relayouts the entire tree, so we skip it when the row is on screen.
func (mt *MacroTree) scrollToIfNeeded(uid string) {
	if uid == "" {
		return
	}
	if mt.lastScrollUID == uid {
		return
	}
	if mt.isRowInViewport(uid) {
		mt.lastScrollUID = uid
		return
	}
	mt.ScrollTo(uid)
	mt.lastScrollUID = uid
}

// isRowInViewport reports whether uid's row intersects the tree's visible area.
func (mt *MacroTree) isRowInViewport(uid string) bool {
	idx := indexOfString(mt.visibleRowUIDs(), uid)
	if idx < 0 {
		return false
	}
	rowH, pitch := mt.dragMetrics()
	scroll, ok := treeScrollOffsetY(&mt.Tree)
	if !ok {
		return false
	}
	viewH := mt.Size().Height
	if viewH <= 0 {
		return false
	}
	return rowInViewport(idx, rowH, pitch, scroll, viewH)
}

// rowInViewport is the viewport intersection test used by isRowInViewport.
func rowInViewport(idx int, rowH, pitch, scroll, viewH float32) bool {
	localRowTop := float32(idx)*pitch - scroll
	localRowBottom := localRowTop + rowH
	return localRowBottom > 0 && localRowTop < viewH
}

// clampScrollOffset keeps the scroll position within the visible tree height after
// branches collapse and content shrinks.
func (mt *MacroTree) clampScrollOffset() {
	offsetY, ok := treeScrollOffsetY(&mt.Tree)
	if !ok {
		return
	}
	contentH := mt.openTreeContentHeight()
	if contentH <= 0 {
		return
	}
	viewH := mt.Size().Height
	if viewH <= 0 {
		return
	}
	clamped := clampedScrollY(contentH, viewH, offsetY)
	if clamped != offsetY {
		mt.ScrollToOffset(clamped)
	}
}

// treeScrollOffsetY reads the tree scroller Y offset via reflect. It must not
// call Interface() on unexported fields — that panics across packages.
func treeScrollOffsetY(t *widget.Tree) (float32, bool) {
	v := reflect.ValueOf(t).Elem()
	sf := v.FieldByName("scroller")
	if !sf.IsValid() || sf.IsNil() {
		return 0, false
	}
	sc := sf.Elem()
	of := sc.FieldByName("Offset")
	if !of.IsValid() {
		return 0, false
	}
	yf := of.FieldByName("Y")
	if !yf.IsValid() {
		return 0, false
	}
	return float32(yf.Float()), true
}

// openTreeContentHeight estimates total height of currently open tree rows.
func (mt *MacroTree) openTreeContentHeight() float32 {
	if mt.Macro == nil || mt.Macro.Root == nil {
		return 0
	}
	th := mt.Theme()
	pad := th.Size(theme.SizeNamePadding)
	branchH, leafH := treeRowHeights(&mt.Tree)

	var height float32
	var walk func(uid string)
	walk = func(uid string) {
		if mt.Tree.Root == "" && uid == "" {
			for _, child := range mt.ChildUIDs(uid) {
				walk(child)
			}
			return
		}
		isBranch := mt.IsBranch(uid)
		rowH := leafH
		if isBranch {
			rowH = branchH
		}
		if height > 0 {
			height += pad
		}
		height += rowH
		if isBranch && mt.IsBranchOpen(uid) {
			for _, child := range mt.ChildUIDs(uid) {
				walk(child)
			}
		}
	}
	walk(mt.Tree.Root)
	return height
}

// RowCenterForScreenshot returns the canvas-absolute center of the visible row
// for uid so docs frames can anchor a click guide on real tree geometry instead
// of hardcoded coordinates. ok is false when uid is not currently visible.
// It assumes the tree is laid out with scroll offset 0 (fresh docs capture).
func (mt *MacroTree) RowCenterForScreenshot(uid string) (fyne.Position, bool) {
	if mt.Macro == nil || mt.Macro.Root == nil || uid == "" {
		return fyne.Position{}, false
	}
	pad := mt.Theme().Size(theme.SizeNamePadding)
	branchH, leafH := treeRowHeights(&mt.Tree)

	top := fyne.CurrentApp().Driver().AbsolutePositionForObject(mt)
	var y float32
	var rowH float32
	first := true
	found := false
	var walk func(id string) bool
	walk = func(id string) bool {
		for _, child := range mt.ChildUIDs(id) {
			h := leafH
			if mt.IsBranch(child) {
				h = branchH
			}
			if !first {
				y += pad
			}
			first = false
			if child == uid {
				rowH = h
				found = true
				return true
			}
			y += h
			if mt.IsBranch(child) && mt.IsBranchOpen(child) {
				if walk(child) {
					return true
				}
			}
		}
		return false
	}
	walk(mt.Macro.Root.GetUID())
	if !found {
		return fyne.Position{}, false
	}
	return fyne.NewPos(top.X+treeItemIconSize*2, top.Y+y+rowH/2), true
}

func treeRowHeights(t *widget.Tree) (branchH, leafH float32) {
	branchH = macroTreeRowHeight
	leafH = macroTreeRowHeight
	v := reflect.ValueOf(t).Elem()
	if bh := v.FieldByName("branchMinSize"); bh.IsValid() {
		if h := bh.FieldByName("Height"); h.IsValid() && h.Float() > 0 {
			branchH = float32(h.Float())
		}
	}
	if lh := v.FieldByName("leafMinSize"); lh.IsValid() {
		if h := lh.FieldByName("Height"); h.IsValid() && h.Float() > 0 {
			leafH = float32(h.Float())
		}
	}
	return branchH, leafH
}
