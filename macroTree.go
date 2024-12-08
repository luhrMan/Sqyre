package main

import (
        "Dark-And-Darker/internal/actions"
        "Dark-And-Darker/internal/structs"
        "fmt"
        "fyne.io/fyne/v2/data/binding"
        xwidget "fyne.io/x/fyne/widget"
        "log"
        "os"
        "strings"

        "fyne.io/fyne/v2"
        "fyne.io/fyne/v2/container"
        "fyne.io/fyne/v2/layout"
        "fyne.io/fyne/v2/theme"
        "fyne.io/fyne/v2/widget"
)

type macro struct {
        tree *widget.Tree
        root *actions.Loop

        sel *xwidget.CompletionEntry
        dt  *container.DocTabs

        boundMacroName   binding.String
        boundGlobalDelay binding.Int
}

func (m *macro) moveNodeUp(selectedUID string) {
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

func (m *macro) moveNodeDown(selectedUID string) {
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

func (m *macro) findNode(node actions.ActionInterface, uid string) actions.ActionInterface {
        if node.GetUID() == uid {
                return node
        }
        if parent, ok := node.(actions.AdvancedActionInterface); ok {
                for _, child := range parent.GetSubActions() {
                        if found := m.findNode(child, uid); found != nil {
                                return found
                        }
                }
        }
        return nil
}

func (m *macro) executeActionTree() { //error
        var context interface{}
        err := m.root.Execute(context)
        if err != nil {
                log.Println(err)
                return
        }
}

func (m *macro) createMacroSelect() {
        files, err := os.ReadDir("./internal/saved-macros")
        if err != nil {
                log.Fatal(err)
        }
        var macroList []string
        for _, f := range files {
                macroList = append(macroList, strings.TrimSuffix(f.Name(), ".json"))
        }
        m.sel = xwidget.NewCompletionEntry(macroList)
        m.sel.OnSubmitted = func(s string) { m.loadTreeFromJsonFile(s + ".json") }
        //        m.sel.
        m.sel.OnChanged = func(s string) {
                //                if len(s) == 0 {
                //                        m.sel.HideCompletion()
                //                        return
                //                }
                var matches []string
                userPrefix := strings.ToLower(s)
                for _, listStr := range macroList {
                        if len(listStr) < len(s) {
                                continue
                        }
                        listPrefix := strings.ToLower(listStr[:len(s)])
                        if userPrefix == listPrefix {
                                matches = append(matches, listStr)
                        }
                }
                m.sel.SetOptions(matches)
                m.sel.ShowCompletion()
        }
        //        , func(s string) { m.loadTreeFromJsonFile(s + ".json") }
}

func (m *macro) createTree() {
        m.root = actions.NewLoop(1, "root", []actions.ActionInterface{})
        m.root.SetUID("")

        m.tree = widget.NewTree(
                func(uid string) []string {
                        node := m.findNode(m.root, uid)
                        if node == nil {
                                return []string{}
                        }

                        if aa, ok := node.(actions.AdvancedActionInterface); ok {
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
                        _, ok := node.(actions.AdvancedActionInterface)
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

func (u *ui) addActionToTree(actionType actions.ActionInterface) {
        var (
                selectedNode = u.m.findNode(u.m.root, selectedTreeItem)
                action       actions.ActionInterface
        )
        switch actionType.(type) {
        case *actions.Wait:
                action = actions.NewWait(time)
        case *actions.Move:
                action = actions.NewMove(moveX, moveY)
        case *actions.Click:
                str := ""
                if !button {
                        str = "left"
                } else {
                        str = "right"
                }
                action = actions.NewClick(str)
        case *actions.Key:
                str := ""
                if !state {
                        str = "down"
                } else {
                        str = "up"
                }
                action = actions.NewKey(key, str)
        case *actions.Loop:
                action = actions.NewLoop(int(count), loopName, []actions.ActionInterface{})
        case *actions.ImageSearch:
                var t []string
                for i, item := range imageSearchTargets {
                        if item == true {
                                t = append(t, i)
                        }
                }
                action = actions.NewImageSearch(imageSearchName, []actions.ActionInterface{}, t, *structs.GetSearchBox(searchArea))
        case *actions.Ocr:
                // n, _ := boundAdvancedActionName.Get()
                // t, _ := boundOcrTarget.Get()
                // s, _ := boundSearchArea.Get()
                // action = &actions.OcrAction{
                // 	SearchBox: *actions.GetSearchBox(s),
                // 	Target:    t,
                // 	advanced: actions.advancedAction{
                // 		base: actions.newBaseAction(),
                // 		Name:       n,
                // 	},
                // }

        }

        if selectedNode == nil {
                selectedNode = u.m.root
        }
        if s, ok := selectedNode.(actions.AdvancedActionInterface); ok {
                s.AddSubAction(action)
        } else {
                selectedNode.GetParent().AddSubAction(action)
        }
        u.m.tree.Refresh()
}

func (u *ui) createUpdateButton() *widget.Button {
        return widget.NewButton("Update", func() {
                node := u.m.findNode(u.m.root, selectedTreeItem)
                if selectedTreeItem == "" {
                        log.Println("No node selected")
                        return
                }
                og := node.String()
                switch node := node.(type) {
                case *actions.Wait:
                        node.Time = time
                case *actions.Move:
                        node.X = moveX
                        node.Y = moveY
                case *actions.Click:
                        if !button {
                                node.Button = "left"
                        } else {
                                node.Button = "right"
                        }
                case *actions.Key:
                        node.Key = key
                        if !state {
                                node.State = "down"
                        } else {
                                node.State = "up"
                        }
                case *actions.Loop:
                        node.Name = loopName
                        node.Count = int(count)
                case *actions.ImageSearch:
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

                u.m.tree.Refresh()
        })
}

func (u *ui) updateTreeOnselect() {
        //Set here, Get @ addActionToTree
        u.m.tree.OnSelected = func(uid widget.TreeNodeID) {
                selectedTreeItem = uid
                switch node := u.m.findNode(u.m.root, uid).(type) {
                case *actions.Wait:
                        u.st.boundTime.Set(node.Time)
                        u.st.tabs.SelectIndex(0)
                case *actions.Move:
                        u.st.boundMoveX.Set(node.X)
                        u.st.boundMoveY.Set(node.Y)
                        u.st.tabs.SelectIndex(1)
                case *actions.Click:
                        if node.Button == "left" {
                                u.st.boundButton.Set(false)
                        } else {
                                u.st.boundButton.Set(true)
                        }
                        u.st.tabs.SelectIndex(2)
                case *actions.Key:
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

                case *actions.Loop:
                        u.st.boundLoopName.Set(node.Name)
                        u.st.boundCount.Set(float64(node.Count))
                        u.st.tabs.SelectIndex(4)
                case *actions.ImageSearch:
                        u.st.boundImageSearchName.Set(node.Name)
                        for t := range imageSearchTargets {
                                imageSearchTargets[t] = false
                        }
                        for _, t := range node.Targets {
                                imageSearchTargets[t] = true
                        }
                        u.m.tree.Refresh()
                        u.st.tabs.Items[5]. //image search tab
                                Content.(*fyne.Container). //settings border
                                Objects[1].(*fyne.Container). //2nd grid with columns
                                Objects[1].(*fyne.Container). //vbox
                                Objects[1].(*widget.Select).SetSelected(node.SearchBox.Name)

                        u.st.tabs.SelectIndex(5)
                }
        }
}
