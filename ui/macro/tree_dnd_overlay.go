package macro

import (
	"fmt"
	"image/color"

	"Sqyre/internal/models/actions"
	"Sqyre/ui/actiondisplay"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/canvas"
	"fyne.io/fyne/v2/container"
	"fyne.io/fyne/v2/theme"
	"fyne.io/fyne/v2/widget"
)

// dropGhostColor fills the overlay preview at the resolved drop slot.
var dropGhostColor = color.NRGBA{R: 60, G: 140, B: 255, A: 50}

// dropGhostStrokeColor outlines the placement preview row.
var dropGhostStrokeColor = color.NRGBA{R: 60, G: 140, B: 255, A: 200}

// dragSourceColor tints the row being dragged at its original position.
var dragSourceColor = color.NRGBA{R: 60, G: 140, B: 255, A: 90}

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
