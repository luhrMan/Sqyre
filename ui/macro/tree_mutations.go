package macro

import (
	"Sqyre/internal/models/actions"
	"Sqyre/ui/actiondisplay"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/canvas"
	"fyne.io/fyne/v2/container"
	"fyne.io/fyne/v2/theme"
	"fyne.io/fyne/v2/widget"
	ttwidget "github.com/dweymouth/fyne-tooltip/widget"
	kxlayout "github.com/ErikKalkoken/fyne-kx/layout"
)

func (mt *MacroTree) moveNode(selectedUID string, up bool) {
	node := mt.Macro.Root.GetAction(selectedUID)
	if node == nil || node.GetParent() == nil {
		return
	}

	parent := node.GetParent()
	psa := parent.GetSubActions()
	index := -1
	for i, child := range psa {
		if child == node {
			index = i
			break
		}
	}

	moved := false
	if up && index > 0 {
		mt.recordMutation()
		psa[index-1], psa[index] = psa[index], psa[index-1]
		mt.Select(psa[index-1].GetUID())
		moved = true
	} else if !up && index < len(psa)-1 {
		mt.recordMutation()
		psa[index], psa[index+1] = psa[index+1], psa[index]
		mt.Select(psa[index+1].GetUID())
		moved = true
	}
	mt.Refresh()
	if moved && mt.OnTreeChanged != nil {
		mt.OnTreeChanged()
	}
}

// DeleteSelectedAction removes the currently selected action from the tree.
func (mt *MacroTree) DeleteSelectedAction() bool {
	if mt == nil || mt.SelectedNode == "" || mt.Macro == nil || mt.Macro.Root == nil {
		return false
	}
	node := mt.Macro.Root.GetAction(mt.SelectedNode)
	return mt.deleteAction(node)
}

func (mt *MacroTree) deleteAction(node actions.ActionInterface) bool {
	if node == nil || node.GetParent() == nil {
		return false
	}
	uid := node.GetUID()
	mt.recordMutation()
	mt.invalidateRowCache(uid)
	node.GetParent().RemoveSubAction(node)
	mt.RefreshItem(uid)
	if len(mt.Macro.Root.SubActions) == 0 || mt.SelectedNode == uid {
		mt.SelectedNode = ""
	}
	if mt.OnTreeChanged != nil {
		mt.OnTreeChanged()
	}
	return true
}

func (mt *MacroTree) setTree() {
	mt.ChildUIDs = func(uid string) []string {
		if aa, ok := mt.Macro.Root.GetAction(uid).(actions.AdvancedActionInterface); ok {
			sa := aa.GetSubActions()
			childIDs := make([]string, len(sa))
			for i, child := range sa {
				childIDs[i] = child.GetUID()
			}
			return childIDs
		}

		return []string{}
	}
	mt.IsBranch = func(uid string) bool {
		node := mt.Macro.Root.GetAction(uid)
		_, ok := node.(actions.AdvancedActionInterface)
		return ok
	}
	mt.CreateNode = func(branch bool) fyne.CanvasObject {
		actionIconBtn := ttwidget.NewButtonWithIcon("", theme.ErrorIcon(), nil)
		actionIconBtn.Importance = widget.LowImportance
		iconBg := canvas.NewRectangle(actiondisplay.ActionPastelColorForApp(""))
		iconBg.CornerRadius = 6
		iconBg.StrokeColor = theme.Color(theme.ColorNameShadow)
		iconBg.StrokeWidth = 1
		iconStack := container.NewStack(iconBg, actionIconBtn)
		dh := newDragHandle()
		dh.tree = mt
		leftSide := container.NewHBox(
			dh,
			iconStack,
		)
		displayContainer := container.New(kxlayout.NewRowWrapLayout())
		itemIconsBox := container.NewHBox()
		displayHolder := container.NewCenter(displayContainer)
		itemIconsHolder := container.NewCenter(itemIconsBox)
		scrollContent := container.NewHBox(displayHolder, itemIconsHolder)
		contentScroll := container.NewHScroll(scrollContent)
		contentScroll.SetMinSize(fyne.NewSize(0, treeItemIconSize))
		rowBody := newTreeRowBody(contentScroll)
		rowBody.tree = mt
		removeBtn := &widget.Button{Icon: theme.CancelIcon(), Importance: widget.LowImportance}
		border := container.NewBorder(nil, nil, leftSide, removeBtn, rowBody)

		hlSimple := canvas.NewRectangle(highlightSimpleColor)
		hlSimple.CornerRadius = 6
		hlSimple.Hide()
		hlFill := canvas.NewRectangle(highlightFillColor)
		hlFill.CornerRadius = 6
		hlFill.Hide()
		hlBg := container.New(&fillLayout{}, hlSimple, hlFill)

		// Highlight overlay is drawn on top of the row. canvas.Rectangle is not
		// tappable, so taps still reach the icon/remove buttons beneath it.
		return container.NewStack(border, hlBg)
	}
	mt.UpdateNode = func(uid string, branch bool, obj fyne.CanvasObject) {
		stack := obj.(*fyne.Container)
		c := stack.Objects[0].(*fyne.Container)
		hlBg := stack.Objects[1].(*fyne.Container)
		if mt.consumeHighlightRefresh(uid) && mt.nodeObjectShowsUID(obj, uid) {
			mt.registerHighlightOverlay(uid, stack, hlBg)
			mt.applyHighlightOverlay(uid, hlBg)
			return
		}

		node := mt.Macro.Root.GetAction(uid)
		if node == nil {
			// Can occur transiently during a node-cache flush (sentinel root).
			return
		}
		leftSide := c.Objects[1].(*fyne.Container)
		dh := leftSide.Objects[0].(*dragHandle)
		dh.tree = mt
		dh.uid = uid
		iconStack := leftSide.Objects[1].(*fyne.Container)
		iconBg := iconStack.Objects[0].(*canvas.Rectangle)
		actionIconBtn := iconStack.Objects[1].(*ttwidget.Button)
		removeButton := c.Objects[2].(*widget.Button)
		rowBody := c.Objects[0].(*treeRowBody)
		rowBody.tree = mt
		rowBody.uid = uid
		contentScroll := rowBody.scroll
		scrollContent, ok := contentScroll.Content.(*fyne.Container)
		if !ok || len(scrollContent.Objects) < 2 {
			return
		}
		displayHolder := scrollContent.Objects[0].(*fyne.Container)
		itemIconsHolder := scrollContent.Objects[1].(*fyne.Container)
		displayContainer := displayHolder.Objects[0].(*fyne.Container)
		itemIconsBox := itemIconsHolder.Objects[0].(*fyne.Container)

		rowContent := mt.cachedRowContent(node)
		displayContainer.Objects = []fyne.CanvasObject{rowContent.display}
		if hover, ok := rowContent.display.(*actionDisplayTooltipHover); ok {
			hover.bindRowBody(rowBody)
		}
		if mt.executing {
			itemIconsBox.Objects = nil
		} else if rowContent.itemIcons != nil {
			itemIconsBox.Objects = []fyne.CanvasObject{rowContent.itemIcons}
		} else {
			itemIconsBox.Objects = nil
		}
		displayContainer.Refresh()
		iconBg.FillColor = macroTreeActionColor(node)
		iconBg.Refresh()
		actionIconBtn.SetIcon(actiondisplay.Icon(node))
		actionIconBtn.SetToolTip(node.GetType())
		actionIconBtn.Importance = widget.LowImportance
		actionIconBtn.OnTapped = nil
		if mt.OnOpenActionDialog != nil {
			action := node
			actionIconBtn.OnTapped = func() { mt.OnOpenActionDialog(action) }
		}
		itemIconsBox.Refresh()

		removeButton.OnTapped = func() {
			mt.deleteAction(node)
		}
		removeButton.Show()

		mt.registerHighlightOverlay(uid, stack, hlBg)
		mt.applyHighlightOverlay(uid, hlBg)
	}
	mt.Tree.OnBranchOpened = func(uid widget.TreeNodeID) {
		if mt.suppressBranchOpenScroll > 0 || mt.dragActive {
			return
		}
		target := uid
		if children := mt.ChildUIDs(uid); len(children) > 0 {
			target = children[0]
		}
		scrollUID := target
		fyne.Do(func() {
			if mt.suppressBranchOpenScroll > 0 || mt.dragActive {
				return
			}
			mt.ScrollTo(scrollUID)
		})
	}
	mt.Tree.OnBranchClosed = func(widget.TreeNodeID) {
		mt.scheduleClampScroll()
	}
}

// insertLocationBelowSelection returns the parent and index at which a new action
// should be inserted relative to the current selection. With no selection, or when
// root is selected, appends to the end of root. When a branch is selected, inserts
// as its first child. Otherwise inserts directly below the selected leaf.
func (mt *MacroTree) insertLocationBelowSelection() (actions.AdvancedActionInterface, int, bool) {
	if mt.Macro == nil || mt.Macro.Root == nil {
		return nil, 0, false
	}
	root := actions.AdvancedActionInterface(mt.Macro.Root)
	if mt.SelectedNode == "" {
		return root, len(root.GetSubActions()), true
	}
	selected := mt.Macro.Root.GetAction(mt.SelectedNode)
	if selected == nil || selected.GetUID() == mt.Macro.Root.GetUID() {
		return root, len(root.GetSubActions()), true
	}
	if branch, ok := selected.(actions.AdvancedActionInterface); ok {
		return branch, 0, true
	}
	parent := selected.GetParent()
	if parent == nil {
		return root, len(root.GetSubActions()), true
	}
	insertIndex := len(parent.GetSubActions())
	for i, c := range parent.GetSubActions() {
		if c.GetUID() == mt.SelectedNode {
			insertIndex = i + 1
			break
		}
	}
	return parent, insertIndex, true
}

func (mt *MacroTree) insertActionAt(parent actions.AdvancedActionInterface, insertIndex int, action actions.ActionInterface) {
	action.SetParent(parent)
	subActions := parent.GetSubActions()
	newSubs := make([]actions.ActionInterface, 0, len(subActions)+1)
	newSubs = append(newSubs, subActions[:insertIndex]...)
	newSubs = append(newSubs, action)
	newSubs = append(newSubs, subActions[insertIndex:]...)
	parent.SetSubActions(newSubs)
}

// InsertActionBelowSelection inserts action relative to the current selection.
// Branches receive the action as their first child; leaves receive it as the next
// sibling. With no selection, appends to the end of root. Returns false when the
// macro tree has no root.
func (mt *MacroTree) InsertActionBelowSelection(action actions.ActionInterface) bool {
	parent, insertIndex, ok := mt.insertLocationBelowSelection()
	if !ok {
		return false
	}
	mt.recordMutation()
	mt.insertActionAt(parent, insertIndex, action)
	return true
}
