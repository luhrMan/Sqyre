package gui

import (
	"Dark-And-Darker/structs"
	"fmt"
	"log"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/container"
	"fyne.io/fyne/v2/layout"
	"fyne.io/fyne/v2/theme"
	"fyne.io/fyne/v2/widget"
)

type Node struct {
	Name     string
	UID      string
	Children []*Node
	Parent   *Node
	Action   structs.Action
}

func newActionNode(parent *Node, action structs.Action) *Node {
	actionNum := len(parent.Children) + 1
	uid := fmt.Sprintf("%s.%d", parent.UID, actionNum)
	node := &Node{
		UID:    uid,
		Parent: parent,
		Action: action,
	}
	//parent.Children = append(parent.Children, node)
	parent.addChild(node)
	log.Printf("New action: %s %s", uid, action)
	return node
}

func (n *Node) addChild(child *Node) {
	n.Children = append(n.Children, child)
	child.Parent = n
}

func (n *Node) removeChild(child *Node) {
	for i, c := range n.Children {
		if c == child {
			n.Children = append(n.Children[:i], n.Children[i+1:]...)
			log.Printf("Removing %s", child.UID)
			child.Parent = nil
			n.renameChildren()
			return
		}
	}
}

func (n *Node) renameChildren() {
	for i, child := range n.Children {
		child.UID = fmt.Sprintf("%s.%d", n.UID, i+1)
		child.renameChildren()
	}
	tree.OpenAllBranches()
}

func newRootNode(name string) *Node {
	rootAction := &structs.ContainerAction{
		Type: structs.ContainerType,
		Name: name,
	}
	return &Node{
		Name:   name,
		UID:    "root",
		Action: rootAction,
		Parent: nil,
	}
}
func findNode(node *Node, uid string) *Node {
	if node.UID == uid {
		return node
	}
	for _, child := range node.Children {
		if found := findNode(child, uid); found != nil {
			return found
		}
	}
	return nil
}

func moveNodeUp(root *Node, selectedUID string, tree *widget.Tree) {
	node := findNode(root, selectedUID)
	if node == nil || node.Parent == nil {
		return
	}

	parent := node.Parent
	index := -1
	for i, child := range parent.Children {
		if child == node {
			index = i
			break
		}
	}

	if index > 0 {
		parent.Children[index-1], parent.Children[index] = parent.Children[index], parent.Children[index-1]
		parent.renameChildren()
		tree.Select(parent.Children[index-1].UID)
		updateTree(tree, root)
	}
}

func moveNodeDown(root *Node, selectedUID string, tree *widget.Tree) {
	node := findNode(root, selectedUID)
	if node == nil || node.Parent == nil {
		return
	}

	parent := node.Parent
	index := -1
	for i, child := range parent.Children {
		if child == node {
			index = i
			break
		}
	}

	if index < len(parent.Children)-1 {
		parent.Children[index], parent.Children[index+1] = parent.Children[index+1], parent.Children[index]
		parent.renameChildren()
		tree.Select(parent.Children[index+1].UID)

		updateTree(tree, root)
	}
}

func createMoveButtons(root *Node, tree *widget.Tree) *fyne.Container {
	moveUpButton := widget.NewButton("Move Up", func() {
		if selectedTreeItem != "" {
			moveNodeUp(root, selectedTreeItem, tree)
		}
	})

	moveDownButton := widget.NewButton("Move Down", func() {
		if selectedTreeItem != "" {
			moveNodeDown(root, selectedTreeItem, tree)
		}
	})

	return container.NewHBox(moveUpButton, moveDownButton)
}

func updateTree(tree *widget.Tree, root *Node) {
	tree.Root = root.UID
	tree.ChildUIDs = func(uid string) []string {
		node := findNode(root, uid)
		if node == nil {
			return []string{}
		}
		childIDs := make([]string, len(node.Children))
		for i, child := range node.Children {
			childIDs[i] = child.UID
		}
		return childIDs
	}
	tree.IsBranch = func(uid string) bool {
		node := findNode(root, uid)
		return node != nil && (node.Action.GetType() == structs.ContainerType || node.Action.GetType() == structs.LoopType)
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
		label.SetText(node.Action.String())

		if node.Parent != nil {
			removeButton.OnTapped = func() {
				node.Parent.removeChild(node)
				updateTree(tree, root)
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
