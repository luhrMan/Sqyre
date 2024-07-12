package gui

import (
	"Dark-And-Darker/structs"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/container"
	"fyne.io/fyne/v2/layout"
	"fyne.io/fyne/v2/theme"
	"fyne.io/fyne/v2/widget"
)

func newRootNode() *structs.LoopAction {
	root := &structs.LoopAction{Count: 1}
	root.SetName("root")
	root.SetUID("")
	root.SetParent(nil)
	return root
}

func moveNodeUp(root *structs.LoopAction, selectedUID string, tree *widget.Tree) {
	node := findNode(root, selectedUID)
	if node == nil || node.GetParent() == nil {
		return
	}

	parent := node.GetParent()
	index := -1
	for i, child := range parent.GetSubActions() {
		if child == node {
			index = i
			break
		}
	}

	if index > 0 {
		parent.GetSubActions()[index-1], parent.GetSubActions()[index] = parent.GetSubActions()[index], parent.GetSubActions()[index-1]
		parent.RenameActions(tree)
		tree.Select(parent.GetSubActions()[index-1].GetUID())
		updateTree(tree, root)
	}
}

func moveNodeDown(root *structs.LoopAction, selectedUID string, tree *widget.Tree) {
	node := findNode(root, selectedUID)
	if node == nil || node.GetParent() == nil {
		return
	}

	parent := node.GetParent()
	index := -1
	for i, child := range parent.GetSubActions() {
		if child == node {
			index = i
			break
		}
	}

	if index < len(parent.GetSubActions())-1 {
		parent.GetSubActions()[index], parent.GetSubActions()[index+1] = parent.GetSubActions()[index+1], parent.GetSubActions()[index]
		parent.RenameActions(tree)
		tree.Select(parent.GetSubActions()[index+1].GetUID())

		updateTree(tree, root)
	}
}

func createMoveButtons(root *structs.LoopAction, tree *widget.Tree) *fyne.Container {
	moveUpButton := &widget.Button{
		Text: "",
		OnTapped: func() {
			if selectedTreeItem != "" {
				moveNodeUp(root, selectedTreeItem, tree)
			}
		},
		Icon:       theme.MoveUpIcon(),
		Importance: widget.HighImportance,
	}

	moveDownButton := &widget.Button{
		Text: "",
		OnTapped: func() {
			if selectedTreeItem != "" {
				moveNodeDown(root, selectedTreeItem, tree)
			}
		},
		Icon:       theme.MoveDownIcon(),
		Importance: widget.HighImportance,
	}

	return container.NewHBox(layout.NewSpacer(), moveUpButton, moveDownButton)
}

func findNode(node structs.ActionInterface, uid string) structs.ActionInterface {
	if node.GetUID() == uid {
		return node
	}
	if parent, ok := node.(structs.AdvancedActionInterface); ok {
		for _, child := range parent.GetSubActions() {
			if found := findNode(child, uid); found != nil {
				return found
			}
		}
	}
	return nil
}

func updateTree(tree *widget.Tree, root *structs.LoopAction) {
	tree.Root = root.UID

	childCache := make(map[string][]string)
	tree.ChildUIDs = func(uid string) []string {
		if cachedChildren, ok := childCache[uid]; ok {
			return cachedChildren
		}
		node := findNode(root, uid)
		if node == nil {
			return []string{}
		}

		if awsa, ok := node.(structs.AdvancedActionInterface); ok {
			sa := awsa.GetSubActions()
			childIDs := make([]string, len(sa))
			for i, child := range sa {
				childIDs[i] = child.GetUID()
			}
			childCache[uid] = childIDs
			return childIDs
		}

		return []string{}
	}

	tree.IsBranch = func(uid string) bool {
		node := findNode(root, uid)
		_, ok := node.(structs.AdvancedActionInterface)
		return node != nil && ok
	}

	tree.CreateNode = func(branch bool) fyne.CanvasObject {
		return container.NewHBox(widget.NewLabel("Template"), layout.NewSpacer(), &widget.Button{Icon: theme.CancelIcon(), Importance: widget.DangerImportance})
	}

	tree.UpdateNode = func(uid string, branch bool, obj fyne.CanvasObject) {
		node := findNode(root, uid)
		if node == nil {
			return
		}
		container := obj.(*fyne.Container)
		label := container.Objects[0].(*widget.Label)
		removeButton := container.Objects[2].(*widget.Button)
		label.SetText(node.String())

		if node.GetParent() != nil {
			removeButton.OnTapped = func() {
				node.GetParent().RemoveSubAction(node, tree)
				updateTree(tree, root)
				if len(root.SubActions) == 0 {
					selectedTreeItem = ""
				}
			}
			removeButton.Show()
		} else {
			removeButton.Hide()
		}
	}
	tree.OnSelected = func(uid widget.TreeNodeID) {
		selectedTreeItem = uid
	}
	tree.Refresh()
}
