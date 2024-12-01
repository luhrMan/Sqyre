package gui

import (
	"Dark-And-Darker/structs"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/container"
	"fyne.io/fyne/v2/layout"
	"fyne.io/fyne/v2/theme"
	"fyne.io/fyne/v2/widget"
)

type macroTree struct {
	tree *widget.Tree
	root *structs.LoopAction
}

func getRoot() *structs.LoopAction {
	return macro.root
}

func (m *macroTree) moveNodeUp(selectedUID string) {
	node := m.findNode(m.root, selectedUID)
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
		parent.RenameActions()
		m.tree.Select(parent.GetSubActions()[index-1].GetUID())
		m.tree.Refresh()
	}
}

func (m *macroTree) moveNodeDown(selectedUID string) {
	node := m.findNode(m.root, selectedUID)
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
		parent.RenameActions()
		m.tree.Select(parent.GetSubActions()[index+1].GetUID())

		m.tree.Refresh()
	}
}

func (m *macroTree) createMoveButtons() *fyne.Container {
	moveUpButton := &widget.Button{
		Text: "",
		OnTapped: func() {
			if selectedTreeItem != "" {
				m.moveNodeUp(selectedTreeItem)
			}
		},
		Icon:       theme.MoveUpIcon(),
		Importance: widget.HighImportance,
	}

	moveDownButton := &widget.Button{
		Text: "",
		OnTapped: func() {
			if selectedTreeItem != "" {
				m.moveNodeDown(selectedTreeItem)
			}
		},
		Icon:       theme.MoveDownIcon(),
		Importance: widget.HighImportance,
	}

	return container.NewHBox(layout.NewSpacer(), moveUpButton, moveDownButton)
}

func (m *macroTree) findNode(node structs.ActionInterface, uid string) structs.ActionInterface {
	if node.GetUID() == uid {
		return node
	}
	if parent, ok := node.(structs.AdvancedActionInterface); ok {
		for _, child := range parent.GetSubActions() {
			if found := m.findNode(child, uid); found != nil {
				return found
			}
		}
	}
	return nil
}

func (m *macroTree) createTree() {
	m.root = structs.NewLoopAction(1, "root", []structs.ActionInterface{})
	m.root.SetUID("")

	m.tree = widget.NewTree(
		func(uid string) []string {
			node := m.findNode(m.root, uid)
			if node == nil {
				return []string{}
			}

			if awsa, ok := node.(structs.AdvancedActionInterface); ok {
				sa := awsa.GetSubActions()
				childIDs := make([]string, len(sa))
				for i, child := range sa {
					childIDs[i] = child.GetUID()
				}
				return childIDs
			}

			return []string{}
		},
		func(uid string) bool {
			//			log.Printf("Create Branch: %v", uid)
			node := m.findNode(m.root, uid)
			_, ok := node.(structs.AdvancedActionInterface)
			return node != nil && ok
		},
		func(branch bool) fyne.CanvasObject {
			//			log.Printf("Create Template")
			return container.NewHBox(widget.NewLabel("Template"), layout.NewSpacer(), &widget.Button{Icon: theme.CancelIcon(), Importance: widget.DangerImportance})
		},
		func(uid string, branch bool, obj fyne.CanvasObject) {
			node := m.findNode(m.root, uid)
			if node == nil {
				return
			}
			c := obj.(*fyne.Container)
			label := c.Objects[0].(*widget.Label)
			removeButton := c.Objects[2].(*widget.Button)
			label.SetText(node.String())

			if node.GetParent() != nil {
				removeButton.OnTapped = func() {
					node.GetParent().RemoveSubAction(node)
					m.root.RenameActions() //should figure out how to rename the whole tree from RemoveSubActions
					macro.tree.Refresh()
					if len(m.root.SubActions) == 0 {
						selectedTreeItem = ""
					}
				}
				removeButton.Show()
			} else {
				removeButton.Hide()
			}
		},
	)
	//Set here, Get @ addActionToTree in content.go
	m.tree.OnSelected = func(uid widget.TreeNodeID) {
		selectedTreeItem = uid
		switch node := m.findNode(m.root, uid).(type) {
		case *structs.WaitAction:
			boundTime.Set(float64(node.Time))
			settingsAccordion.Open(0)
		case *structs.MoveAction:
			boundMoveX.Set(float64(node.X))
			boundMoveY.Set(float64(node.Y))
			settingsAccordion.Open(1)
		case *structs.ClickAction:
			if node.Button == "left" {
				boundButton.Set(false)
			} else {
				boundButton.Set(true)
			}
			settingsAccordion.Open(2)

		case *structs.KeyAction:
			boundKeySelect.SetSelected(node.Key)
			if node.State == "down" {
				boundState.Set(false)
			} else {
				boundState.Set(true)
			}
			settingsAccordion.Open(3)

		case *structs.LoopAction:
			boundAdvancedActionName.Set(node.Name)
			boundCount.Set(float64(node.Count))
			settingsAccordion.Open(4)

		case *structs.ImageSearchAction:
			boundAdvancedActionName.Set(node.Name)
			boundSelectedItemsMap.Set(map[string]any{})
			for _, t := range node.Targets {
				boundSelectedItemsMap.SetValue(t, true)
			}
			boundSearchAreaSelect.SetSelected(node.SearchBox.Name)
			settingsAccordion.Open(5)
		}
	}
}

func (m *macroTree) addActionToTree(actionType structs.ActionInterface) {
	var (
		selectedNode = m.findNode(m.root, selectedTreeItem)
		action       structs.ActionInterface
	)
	switch actionType.(type) {
	case *structs.WaitAction:
		t, _ := boundTime.Get()
		action = structs.NewWaitAction(int(t))
	case *structs.MoveAction:
		x, _ := boundMoveX.Get()
		y, _ := boundMoveY.Get()
		action = structs.NewMoveAction(int(x), int(y))
	case *structs.ClickAction:
		str := ""
		b, _ := boundButton.Get()
		if !b {
			str = "left"
		} else {
			str = "right"
		}
		action = structs.NewClickAction(str)
	case *structs.KeyAction:
		str := ""
		k, _ := boundKey.Get()
		s, _ := boundState.Get()
		if !s {
			str = "down"
		} else {
			str = "up"
		}
		action = structs.NewKeyAction(k, str)
	case *structs.LoopAction:
		n, _ := boundAdvancedActionName.Get()
		c, _ := boundCount.Get()
		action = structs.NewLoopAction(int(c), n, []structs.ActionInterface{})
	case *structs.ImageSearchAction:
		n, _ := boundAdvancedActionName.Get()
		s, _ := boundSearchArea.Get()
		t := boundSelectedItemsMap.Keys()
		action = structs.NewImageSearchAction(n, []structs.ActionInterface{}, t, *structs.GetSearchBox(s))
	case *structs.OcrAction:
		// n, _ := boundAdvancedActionName.Get()
		// t, _ := boundOcrTarget.Get()
		// s, _ := boundSearchArea.Get()
		// action = &structs.OcrAction{
		// 	SearchBox: *structs.GetSearchBox(s),
		// 	Target:    t,
		// 	AdvancedAction: structs.AdvancedAction{
		// 		BaseAction: structs.NewBaseAction(),
		// 		Name:       n,
		// 	},
		// }

	}

	if selectedNode == nil {
		selectedNode = getRoot()
	}
	if s, ok := selectedNode.(structs.AdvancedActionInterface); ok {
		s.AddSubAction(action)
	} else {
		selectedNode.GetParent().AddSubAction(action)
	}
	m.tree.Refresh()
}
