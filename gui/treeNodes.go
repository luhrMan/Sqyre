package gui

import (
	"Dark-And-Darker/structs"
	"Dark-And-Darker/utils"
	"fmt"
	"log"
	"strconv"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/container"
	"fyne.io/fyne/v2/layout"
	"fyne.io/fyne/v2/theme"
	"fyne.io/fyne/v2/widget"
)

type NodeInterface interface {
	GetName() string
	SetName(string)
	GetUID() string
	SetUID(string)
	GetParent() *ContainerNode
	SetParent(*ContainerNode)
}

type BaseNode struct {
	Name   string
	UID    string
	Parent *ContainerNode
}

func (n *BaseNode) GetName() string {
	return n.Name
}

func (n *BaseNode) SetName(name string) {
	n.Name = name
}

func (n *BaseNode) GetUID() string {
	return n.UID
}

func (n *BaseNode) SetUID(uid string) {
	n.UID = uid
}

func (n *BaseNode) SetParent(parent *ContainerNode) {
	n.Parent = parent
}

func (n *BaseNode) GetParent() *ContainerNode {
	return n.Parent
}

type ContainerNode struct {
	BaseNode
	Children   []NodeInterface
	Iterations int
}

func newContainerNode(parent *ContainerNode, iterations int, name string) *ContainerNode {
	actionNum := len(parent.Children) + 1
	uid := fmt.Sprintf("%s.%d", parent.UID, actionNum)
	node := &ContainerNode{
		BaseNode: BaseNode{
			UID:    uid,
			Parent: parent,
			Name:   name + " | " + utils.GetEmoji("Container") + strconv.FormatInt(int64(iterations), 10),
		},
		Iterations: iterations,
	}
	parent.addChild(node)
	log.Printf("New container: %s", uid)
	return node
}

func (n *ContainerNode) addChild(child NodeInterface) {
	n.Children = append(n.Children, child)
	child.SetParent(n)
}

func (n *ContainerNode) removeChild(child NodeInterface) {
	for i, c := range n.Children {
		if c == child {
			n.Children = append(n.Children[:i], n.Children[i+1:]...)
			log.Printf("Removing %s", child.GetUID())
			child.SetParent(nil)
			n.renameChildren()
			return
		}
	}
}

func (n *ContainerNode) renameChildren() {
	for i, child := range n.Children {
		child.SetUID(fmt.Sprintf("%s.%d", n.UID, i+1))
		if c, ok := child.(*ContainerNode); ok {
			c.renameChildren()
		}
	}
	tree.OpenAllBranches()
}

type ActionNode struct {
	BaseNode
	Action structs.Action
}

func newActionNode(parent *ContainerNode, action structs.Action) *ActionNode {
	if parent == nil {
		parent = root
	}
	actionNum := len(parent.Children) + 1
	uid := fmt.Sprintf("%s.%d", parent.GetUID(), actionNum)
	actionNode := &ActionNode{
		BaseNode: BaseNode{
			UID:    uid,
			Parent: parent,
		},
		Action: action,
	}
	parent.addChild(actionNode)
	log.Printf("New action: %s %s", uid, action)
	return actionNode
}

func newRootNode() *ContainerNode {
	return &ContainerNode{
		BaseNode: BaseNode{
			Name: "root",
			UID:  "",
		},
		Iterations: 1,
	}
}
func findNode(node NodeInterface, uid string) NodeInterface {
	if node.GetUID() == uid {
		return node
	}
	if n, ok := node.(*ContainerNode); ok {
		for _, child := range n.Children {
			if found := findNode(child, uid); found != nil {
				return found
			}
		}
	}
	return nil
}

func moveNodeUp(root *ContainerNode, selectedUID string, tree *widget.Tree) {
	node := findNode(root, selectedUID)
	if node == nil || node.GetParent() == nil {
		return
	}

	parent := node.GetParent()
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
		tree.Select(parent.Children[index-1].GetUID())
		updateTree(tree, root)
	}
}

func moveNodeDown(root *ContainerNode, selectedUID string, tree *widget.Tree) {
	node := findNode(root, selectedUID)
	if node == nil || node.GetParent() == nil {
		return
	}

	parent := node.GetParent()
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
		tree.Select(parent.Children[index+1].GetUID())

		updateTree(tree, root)
	}
}

func createMoveButtons(root *ContainerNode, tree *widget.Tree) *fyne.Container {
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

func updateTree(tree *widget.Tree, root *ContainerNode) {
	tree.Root = root.UID
	tree.ChildUIDs = func(uid string) []string {
		node := findNode(root, uid)
		if node == nil {
			return []string{}
		}
		childIDs := make([]string, len(node.(*ContainerNode).Children))
		for i, child := range node.(*ContainerNode).Children {
			childIDs[i] = child.GetUID()
		}

		return childIDs
	}
	tree.IsBranch = func(uid string) bool {
		var b bool
		node := findNode(root, uid)
		if _, ok := node.(*ContainerNode); ok {
			b = true
		}
		return node != nil && b
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

		switch node := node.(type) {
		case *ActionNode:
			label.SetText(node.Action.String())
		case *ContainerNode:
			label.SetText(node.Name)
		default:
			label.SetText("node type fukced up")
		}

		if node.GetParent() != nil {
			removeButton.OnTapped = func() {
				node.GetParent().removeChild(node)
				updateTree(tree, root)
				if len(root.Children) == 0 {
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
