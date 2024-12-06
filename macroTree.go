package main

import (
        "Dark-And-Darker/internal/structs"
        "fmt"
        "fyne.io/fyne/v2/data/binding"
        "log"

        "fyne.io/fyne/v2"
        "fyne.io/fyne/v2/container"
        "fyne.io/fyne/v2/layout"
        "fyne.io/fyne/v2/theme"
        "fyne.io/fyne/v2/widget"
)

type macroTree struct {
        tree           *widget.Tree
        root           *structs.LoopAction
        boundMacroName binding.String
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

func (m *macroTree) executeActionTree() { //error
        var context interface{}
        err := m.root.Execute(context)
        if err != nil {
                log.Println(err)
                return
        }
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

                        if aa, ok := node.(structs.AdvancedActionInterface); ok {
                                sa := aa.GetSubActions()
                                childIDs := make([]string, len(sa))
                                for i, child := range sa {
                                        childIDs[i] = child.GetUID()
                                }
                                return childIDs
                        }

                        return []string{}
                },
                func(uid string) bool {
                        node := m.findNode(m.root, uid)
                        _, ok := node.(structs.AdvancedActionInterface)
                        return node != nil && ok
                },
                func(branch bool) fyne.CanvasObject {
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
                                        m.tree.Refresh()
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

}

func (u *ui) addActionToTree(actionType structs.ActionInterface) {
        var (
                selectedNode = u.mt.findNode(u.mt.root, selectedTreeItem)
                action       structs.ActionInterface
        )
        switch actionType.(type) {
        case *structs.WaitAction:
                action = structs.NewWaitAction(time)
        case *structs.MoveAction:
                action = structs.NewMoveAction(moveX, moveY)
        case *structs.ClickAction:
                str := ""
                if !button {
                        str = "left"
                } else {
                        str = "right"
                }
                action = structs.NewClickAction(str)
        case *structs.KeyAction:
                str := ""
                if !state {
                        str = "down"
                } else {
                        str = "up"
                }
                action = structs.NewKeyAction(key, str)
        case *structs.LoopAction:
                action = structs.NewLoopAction(int(count), loopName, []structs.ActionInterface{})
        case *structs.ImageSearchAction:
                var t []string
                for i, item := range imageSearchTargets {
                        if item == true {
                                t = append(t, i)
                        }
                }
                action = structs.NewImageSearchAction(imageSearchName, []structs.ActionInterface{}, t, *structs.GetSearchBox(searchArea))
        case *structs.OcrAction:
                // n, _ := boundAdvancedActionName.Get()
                // t, _ := boundOcrTarget.Get()
                // s, _ := boundSearchArea.Get()
                // action = &structs.OcrAction{
                // 	SearchBox: *structs.GetSearchBox(s),
                // 	Target:    t,
                // 	AdvancedAction: structs.AdvancedAction{
                // 		baseAction: structs.newBaseAction(),
                // 		Name:       n,
                // 	},
                // }

        }

        if selectedNode == nil {
                selectedNode = u.mt.root
        }
        if s, ok := selectedNode.(structs.AdvancedActionInterface); ok {
                s.AddSubAction(action)
        } else {
                selectedNode.GetParent().AddSubAction(action)
        }
        u.mt.tree.Refresh()
}

func (u *ui) createUpdateButton() *widget.Button {
        return widget.NewButton("Update", func() {
                node := u.mt.findNode(u.mt.root, selectedTreeItem)
                if selectedTreeItem == "" {
                        log.Println("No node selected")
                        return
                }
                og := node.String()
                switch node := node.(type) {
                case *structs.WaitAction:
                        node.Time = time
                case *structs.MoveAction:
                        node.X = moveX
                        node.Y = moveY
                case *structs.ClickAction:
                        if !button {
                                node.Button = "left"
                        } else {
                                node.Button = "right"
                        }
                case *structs.KeyAction:
                        node.Key = key
                        if !state {
                                node.State = "down"
                        } else {
                                node.State = "up"
                        }
                case *structs.LoopAction:
                        node.Name = loopName
                        node.Count = int(count)
                case *structs.ImageSearchAction:
                        var t []string
                        for i, item := range imageSearchTargets {
                                if item == true {
                                        t = append(t, i)
                                }
                        }
                        //                        t := boundSelectedItemsMap.Keys()
                        node.Name = imageSearchName
                        node.SearchBox = *structs.GetSearchBox(searchArea)
                        node.Targets = t
                }

                fmt.Printf("Updated node: %+v from '%v' to '%v' \n", node.GetUID(), og, node)

                u.mt.tree.Refresh()
        })
}

func (u *ui) updateTreeOnselect() {
        //Set here, Get @ addActionToTree
        u.mt.tree.OnSelected = func(uid widget.TreeNodeID) {
                selectedTreeItem = uid
                switch node := u.mt.findNode(u.mt.root, uid).(type) {
                case *structs.WaitAction:
                        u.st.boundTime.Set(node.Time)
                        u.st.tabs.SelectIndex(0)
                case *structs.MoveAction:
                        u.st.boundMoveX.Set(node.X)
                        u.st.boundMoveY.Set(node.Y)
                        u.st.tabs.SelectIndex(1)
                case *structs.ClickAction:
                        if node.Button == "left" {
                                u.st.boundButton.Set(false)
                        } else {
                                u.st.boundButton.Set(true)
                        }
                        u.st.tabs.SelectIndex(2)
                case *structs.KeyAction:
                        key = node.Key
                        u.st.tabs.Items[3].
                                Content.(*fyne.Container).
                                Objects[0].(*fyne.Container).
                                Objects[1].(*widget.Select).SetSelected(node.Key)
                        //                                                boundKeySelect.SetSelected(node.Key)
                        if node.State == "down" {
                                u.st.boundState.Set(false)
                        } else {
                                u.st.boundState.Set(true)
                        }
                        u.st.tabs.SelectIndex(3)

                case *structs.LoopAction:
                        u.st.boundLoopName.Set(node.Name)
                        u.st.boundCount.Set(float64(node.Count))
                        u.st.tabs.SelectIndex(4)
                case *structs.ImageSearchAction:
                        u.st.boundImageSearchName.Set(node.Name)
                        for t := range imageSearchTargets {
                                imageSearchTargets[t] = false
                        }
                        for _, t := range node.Targets {
                                imageSearchTargets[t] = true
                        }
                        u.mt.tree.Refresh()
                        u.st.tabs.Items[5]. //image search tab
                                Content.(*fyne.Container). //settings border
                                Objects[1].(*fyne.Container). //2nd grid with columns
                                Objects[1].(*fyne.Container). //vbox
                                Objects[1].(*widget.Select).SetSelected(node.SearchBox.Name)

                        u.st.tabs.SelectIndex(5)
                }
        }
}
