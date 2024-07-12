package gui

import (
	"Dark-And-Darker/structs"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/container"
	"fyne.io/fyne/v2/layout"
	"fyne.io/fyne/v2/theme"
	"fyne.io/fyne/v2/widget"
)

// type NodeInterface interface {
// 	GetName() string
// 	SetName(string)
// 	GetUID() string
// 	SetUID(string)
// 	GetParent() *Node
// 	SetParent(*Node)
// }

// type Node struct {
// 	Name     string
// 	UID      string
// 	Parent   *Node
// 	Children []*Node
// 	Action   structs.Action
// }

// func (n *Node) GetName() string {
// 	return n.Name
// }

// func (n *Node) SetName(name string) {
// 	n.Name = name
// }

// func (n *Node) GetUID() string {
// 	return n.UID
// }

// func (n *Node) SetUID(uid string) {
// 	n.UID = uid
// }

// func (n *Node) SetParent(parent *Node) {
// 	n.Parent = parent
// }

// func (n *Node) GetParent() *Node {
// 	return n.Parent
// }

//	type SearchContainerNode struct {
//		ContainerNode
//		Context map[string]interface{}
//	}
// func newAction(parent structs.ActionWithSubActionsInterface, newAction structs.ActionInterface, name string) structs.ActionInterface {
// 	actionNum := len(parent.GetSubActions()) + 1
// 	uid := fmt.Sprintf("%s.%d", parent.GetUID(), actionNum)
// 	action := &structs.BaseAction{
// 		UID:  uid,
// 		Name: name + " | " + utils.GetEmoji("Container"),
// 	}
// 	parent.AddSubAction(action)
// 	log.Printf("New container: %s", uid)
// 	return action
// }

// func newActionNode(parent *Node, action structs.Action) *Node {
// 	if parent == nil {
// 		parent = root
// 	}
// 	actionNum := len(parent.Children) + 1
// 	uid := fmt.Sprintf("%s.%d", parent.GetUID(), actionNum)
// 	actionNode := &Node{
// 		UID:    uid,
// 		Parent: parent,
// 	}

// 	parent.addChild(actionNode)
// 	log.Printf("New action: %s %s", uid, action)
// 	return actionNode
// }

func newRootNode() *structs.LoopAction {
	root := &structs.LoopAction{}
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
	moveUpButton := widget.NewButtonWithIcon("", theme.MoveUpIcon(), func() {
		if selectedTreeItem != "" {
			moveNodeUp(root, selectedTreeItem, tree)
		}
	})

	moveDownButton := widget.NewButtonWithIcon("", theme.MoveDownIcon(), func() {
		if selectedTreeItem != "" {
			moveNodeDown(root, selectedTreeItem, tree)
		}
	})

	return container.NewHBox(layout.NewSpacer(), moveUpButton, moveDownButton)
}

func findNode(node structs.ActionInterface, uid string) structs.ActionInterface {
	if node.GetUID() == uid {
		return node
	}
	if parent, ok := node.(structs.ActionWithSubActionsInterface); ok {
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

		if awsa, ok := node.(structs.ActionWithSubActionsInterface); ok {
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
		_, ok := node.(structs.ActionWithSubActionsInterface)
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
