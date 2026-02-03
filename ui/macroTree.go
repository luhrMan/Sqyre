package ui

import (
	"Squire/internal/models"
	"Squire/internal/models/actions"
	"Squire/internal/models/serialize"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/container"
	"fyne.io/fyne/v2/layout"
	"fyne.io/fyne/v2/theme"
	"fyne.io/fyne/v2/widget"
	ttwidget "github.com/dweymouth/fyne-tooltip/widget"
)

// var selectedTreeItem = ""

type MacroTree struct {
	widget.Tree
	Macro        *models.Macro
	SelectedNode string
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
	mt.CreateNode = func(branch bool) fyne.CanvasObject {
		return container.NewHBox(ttwidget.NewIcon(theme.ErrorIcon()), widget.NewLabel("Template"), layout.NewSpacer(), &widget.Button{Icon: theme.CancelIcon(), Importance: widget.LowImportance})
	}
	mt.UpdateNode = func(uid string, branch bool, obj fyne.CanvasObject) {
		node := mt.Macro.Root.GetAction(uid)

		c := obj.(*fyne.Container)
		icon := c.Objects[0].(*ttwidget.Icon)
		label := c.Objects[1].(*widget.Label)
		removeButton := c.Objects[3].(*widget.Button)

		label.SetText(node.String())
		icon.SetResource(node.Icon())
		icon.SetToolTip(node.GetType())

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
