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
	fyne.Do(func() {
		mt.clampScrollOffset()
	})
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
