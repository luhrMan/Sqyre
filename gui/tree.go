package gui

import (
	"Dark-And-Darker/structs"
	"Dark-And-Darker/utils"
	"fmt"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/container"
	"fyne.io/fyne/v2/layout"
	"fyne.io/fyne/v2/theme"
	"fyne.io/fyne/v2/widget"
)

func createSampleTree() *Node {
	seq1 := NewSequence(&macro, "preset x2")
	NewAction(seq1, &structs.MouseMoveAction{X: 100, Y: 100})
	NewAction(seq1, &structs.ClickAction{Button: "Left"})

	seq2 := NewSequence(&macro, "preset x1")
	NewAction(seq2, &structs.MouseMoveAction{X: 2000, Y: 200})
	NewAction(seq2, &structs.ClickAction{Button: "Right"})
	return &macro
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
		return node != nil && (node.Type == MacroType || node.Type == SequenceType)
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

		switch node.Type {
		case MacroType:
			label.SetText(fmt.Sprintf("📁 %s", node.UID))
		case SequenceType:
			label.SetText(fmt.Sprintf("%s %s %s", utils.GetEmoji("Sequence"), node.UID, node.Name))
		case ActionType:
			label.SetText(node.Action.String())
		}

		if node.Parent != nil {
			removeButton.OnTapped = func() {
				node.Parent.RemoveChild(node)
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
