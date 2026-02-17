package ui

import (
	"Squire/internal/assets"
	"Squire/internal/models"
	"Squire/internal/models/actions"
	"Squire/internal/models/serialize"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/canvas"
	"fyne.io/fyne/v2/container"
	"fyne.io/fyne/v2/theme"
	"fyne.io/fyne/v2/widget"
	ttwidget "github.com/dweymouth/fyne-tooltip/widget"
)

// var selectedTreeItem = ""

// OnOpenActionDialog is called when the user taps an action's icon to edit it.
// If non-nil, the tree will open the action dialog from this callback.
type OnOpenActionDialogFunc func(action actions.ActionInterface)

type MacroTree struct {
	widget.Tree
	Macro              *models.Macro
	SelectedNode       string
	OnOpenActionDialog OnOpenActionDialogFunc
}

func NewMacroTree(m *models.Macro) *MacroTree {
	t := &MacroTree{}
	t.ExtendBaseWidget(t)
	t.Macro = m
	t.setTree()

	return t
}

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

	if up && index > 0 {
		psa[index-1], psa[index] = psa[index], psa[index-1]
		mt.Select(psa[index-1].GetUID())
	} else if !up && index < len(psa)-1 {
		psa[index], psa[index+1] = psa[index+1], psa[index]
		mt.Select(psa[index+1].GetUID())
	}
	mt.Refresh()
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
	// Tree row: Border with left=[actionIconButton, label], center=itemIconsScroll (fills space), right=removeButton
	const treeItemIconSize = 24
	mt.CreateNode = func(branch bool) fyne.CanvasObject {
		itemIconsBox := container.NewHBox()
		itemIconsScroll := container.NewHScroll(itemIconsBox)
		itemIconsScroll.SetMinSize(fyne.NewSize(0, treeItemIconSize))
		actionIconBtn := ttwidget.NewButtonWithIcon("", theme.ErrorIcon(), nil)
		actionIconBtn.Importance = widget.LowImportance
		leftSide := container.NewHBox(
			actionIconBtn,
			widget.NewLabel("Template"),
		)
		removeBtn := &widget.Button{Icon: theme.CancelIcon(), Importance: widget.LowImportance}
		return container.NewBorder(nil, nil, leftSide, removeBtn, itemIconsScroll)
	}
	mt.UpdateNode = func(uid string, branch bool, obj fyne.CanvasObject) {
		node := mt.Macro.Root.GetAction(uid)

		c := obj.(*fyne.Container)
		// Border with nil top/bottom: Objects = [left, right, center]
		leftSide := c.Objects[1].(*fyne.Container)
		actionIconBtn := leftSide.Objects[0].(*ttwidget.Button)
		label := leftSide.Objects[1].(*widget.Label)
		removeButton := c.Objects[2].(*widget.Button)
		itemIconsScroll := c.Objects[0].(*container.Scroll)

		label.SetText(node.String())
		actionIconBtn.SetIcon(node.Icon())
		actionIconBtn.SetToolTip(node.GetType())
		actionIconBtn.Importance = widget.MediumImportance
		actionIconBtn.OnTapped = nil
		if mt.OnOpenActionDialog != nil {
			action := node
			actionIconBtn.OnTapped = func() { mt.OnOpenActionDialog(action) }
		}

		// For image search actions, show selected item icons; for wait-for-pixel, show target color
		itemIconsBox := itemIconsScroll.Content.(*fyne.Container)
		itemIconsBox.Objects = itemIconsBox.Objects[:0]
		if is, ok := node.(*actions.ImageSearch); ok && len(is.Targets) > 0 {
			previewSize := fyne.NewSize(treeItemIconSize, treeItemIconSize)
			for _, target := range is.Targets {
				if path := getIconPathForTarget(target); path != "" {
					if res := assets.GetFyneResource(path); res != nil {
						img := canvas.NewImageFromResource(res)
						img.SetMinSize(previewSize)
						img.FillMode = canvas.ImageFillContain
						itemIconsBox.Add(img)
					}
				}
			}
			itemIconsScroll.Show()
		} else if wfp, ok := node.(*actions.WaitForPixel); ok {
			if c, ok := hexToColor(wfp.TargetColor); ok {
				swatch := canvas.NewRectangle(c)
				swatch.SetMinSize(fyne.NewSize(treeItemIconSize, treeItemIconSize))
				itemIconsBox.Add(swatch)
			}
			itemIconsScroll.Show()
		} else {
			itemIconsScroll.Hide()
		}
		itemIconsBox.Refresh()

		removeButton.OnTapped = func() {
			node.GetParent().RemoveSubAction(node)
			mt.RefreshItem(uid)
			if len(mt.Macro.Root.SubActions) == 0 || mt.SelectedNode == node.GetUID() {
				mt.SelectedNode = ""
			}
		}
		removeButton.Show()
	}
}

// PasteNode creates a copy of the action from clipboardMap and inserts it into
// the current selection: if the selected node is an advanced action (has children),
// the pasted node is added as its last child; otherwise it is inserted below the
// selected node as a sibling. With no selection, pastes at the end of root.
// Returns true if paste succeeded.
func (mt *MacroTree) PasteNode(clipboardMap map[string]any) bool {
	if clipboardMap == nil {
		return false
	}
	var parent actions.AdvancedActionInterface
	insertIndex := 0
	if mt.SelectedNode != "" {
		selected := mt.Macro.Root.GetAction(mt.SelectedNode)
		if selected == nil {
			return false
		}
		if adv, ok := selected.(actions.AdvancedActionInterface); ok {
			// Paste into the selected advanced action as its last child
			parent = adv
			insertIndex = len(parent.GetSubActions())
		} else {
			// Paste below the selected node as a sibling
			parent = selected.GetParent()
			if parent == nil {
				return false
			}
			psa := parent.GetSubActions()
			for i, c := range psa {
				if c.GetUID() == mt.SelectedNode {
					insertIndex = i + 1
					break
				}
			}
		}
	} else {
		parent = mt.Macro.Root
		insertIndex = len(parent.GetSubActions())
	}
	newAction, err := serialize.ViperSerializer.CreateActionFromMap(clipboardMap, parent)
	if err != nil {
		return false
	}
	subActions := parent.GetSubActions()
	newSubs := make([]actions.ActionInterface, 0, len(subActions)+1)
	newSubs = append(newSubs, subActions[:insertIndex]...)
	newSubs = append(newSubs, newAction)
	newSubs = append(newSubs, subActions[insertIndex:]...)
	parent.SetSubActions(newSubs)
	mt.Select(newAction.GetUID())
	mt.SelectedNode = newAction.GetUID()
	mt.Refresh()
	return true
}
