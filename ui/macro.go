package ui

import (
	"Squire/encoding"
	"Squire/internal"
	"Squire/internal/actions"
	"Squire/internal/data"
	"fmt"
	"log"
	"slices"

	"fyne.io/fyne/v2/data/binding"
	"fyne.io/fyne/v2/dialog"
	"github.com/go-vgo/robotgo"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/container"
	"fyne.io/fyne/v2/layout"
	"fyne.io/fyne/v2/theme"
	"fyne.io/fyne/v2/widget"
)

// func newProgram() {
// 	ss := make(map[[2]int]ScreenSize)
// 	pm := make(map[string]data.Point)
// 	sam := make(map[string]data.SearchArea)
// 	ss[[2]int{10, 10}] = ScreenSize{
// 		Points:      &pm,
// 		SearchAreas: &sam,
// 	}

// 	p := &Program{
// 		Macros:     &[]Macro{},
// 		Items:      &map[string]data.Item{},
// 		ScreenSize: &ss,
// 	}
// 	programs["Dark and Darker"] = *p
// 	log.Println(programs["Dark and Darker"].Macros)
// }

type MacroTree struct {
	Macro *internal.Macro
	Tree  *widget.Tree

	boundMacroName binding.String
}

func NewMacroTree() MacroTree {
	m := MacroTree{}
	m.createTree()
	return m
}

func (m *MacroTree) moveNodeUp(selectedUID string) {
	node := m.findNode(m.Macro.Root, selectedUID)
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
		m.Tree.Select(parent.GetSubActions()[index-1].GetUID())
		m.Tree.Refresh()
	}
}

func (m *MacroTree) moveNodeDown(selectedUID string) {
	node := m.findNode(m.Macro.Root, selectedUID)
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
		m.Tree.Select(parent.GetSubActions()[index+1].GetUID())

		m.Tree.Refresh()
	}
}

func (m *MacroTree) findNode(node actions.ActionInterface, uid string) actions.ActionInterface {
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

// func (m *MacroTree) ExecuteActionTree(ctx ...interface{}) { //error
// 	err := m.Macro.Root.Execute(ctx)
// 	if err != nil {
// 		log.Println(err)
// 		return
// 	}
// }

func (m *MacroTree) createTree() {
	// macro := NewMacro()
	m.Macro.Root = actions.NewLoop(1, "root", []actions.ActionInterface{})
	m.Macro.Root.SetUID("")

	m.Tree = &widget.Tree{}

	m.Tree.ChildUIDs = func(uid string) []string {
		node := m.findNode(m.Macro.Root, uid)
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
	}
	m.Tree.IsBranch = func(uid string) bool {
		node := m.findNode(m.Macro.Root, uid)
		_, ok := node.(actions.AdvancedActionInterface)
		return node != nil && ok
	}
	m.Tree.CreateNode = func(branch bool) fyne.CanvasObject {
		return container.NewHBox(widget.NewLabel("Template"), layout.NewSpacer(), &widget.Button{Icon: theme.CancelIcon(), Importance: widget.DangerImportance})
	}
	m.Tree.UpdateNode = func(uid string, branch bool, obj fyne.CanvasObject) {
		node := m.findNode(m.Macro.Root, uid)
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
				m.Macro.Root.RenameActions() //should figure out how to rename the whole tree from RemoveSubActions
				m.Tree.Refresh()
				if len(m.Macro.Root.SubActions) == 0 {
					selectedTreeItem = ""
				}
			}
			removeButton.Show()
		} else {
			removeButton.Hide()
		}
	}
}

func (m *MacroTree) addActionToTree(actionType actions.ActionInterface) {
	var (
		selectedNode = m.findNode(m.Macro.Root, selectedTreeItem)
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
			str = "Down"
		} else {
			str = "Up"
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
		action = actions.NewImageSearch(imageSearchName, []actions.ActionInterface{}, t, *data.GetSearchArea(searchArea))
	case *actions.Ocr:
		action = actions.NewOcr(ocrTarget, []actions.ActionInterface{}, ocrTarget, *data.GetSearchArea(ocrSearchBox))
	}

	if selectedNode == nil {
		selectedNode = m.Macro.Root
	}
	if s, ok := selectedNode.(actions.AdvancedActionInterface); ok {
		s.AddSubAction(action)
	} else {
		selectedNode.GetParent().AddSubAction(action)
	}
	m.Tree.Refresh()
}

func (u *Ui) updateTreeOnselect() {
	//Set here, Get @ addActionToTree
	u.getCurrentTabMacro().Tree.OnSelected = func(uid widget.TreeNodeID) {
		selectedTreeItem = uid
		switch node := u.getCurrentTabMacro().findNode(u.getCurrentTabMacro().Macro.Root, uid).(type) {
		case *actions.Wait:
			u.st.boundTime.Set(node.Time)
			u.st.tabs.SelectIndex(waittab)
		case *actions.Move:
			u.st.boundMoveX.Set(node.X)
			u.st.boundMoveY.Set(node.Y)
			u.st.tabs.SelectIndex(movetab)
		case *actions.Click:
			if node.Button == "left" {
				u.st.boundButton.Set(false)
			} else {
				u.st.boundButton.Set(true)
			}
			u.st.tabs.SelectIndex(clicktab)
		case *actions.Key:
			key = node.Key
			u.st.boundKeySelect.SetSelected(node.Key)
			//			u.st.tabs.Items[3].
			//				Content.(*fyne.Container).
			//				Objects[0].(*fyne.Container).
			//				Objects[1].(*widget.Select).SetSelected(node.Key)

			//                                                boundKeySelect.SetSelected(node.Key)
			if node.State == "Down" {
				u.st.boundState.Set(false)
			} else {
				u.st.boundState.Set(true)
			}
			u.st.tabs.SelectIndex(keytab)

		case *actions.Loop:
			u.st.boundLoopName.Set(node.Name)
			u.st.boundCount.Set(node.Count)
			u.st.tabs.SelectIndex(looptab)
		case *actions.ImageSearch:
			u.st.boundImageSearchName.Set(node.Name)
			for t := range imageSearchTargets {
				imageSearchTargets[t] = false
			}
			for _, t := range node.Targets {
				imageSearchTargets[t] = true
			}
			u.getCurrentTabMacro().Tree.Refresh()
			u.st.boundImageSearchAreaSelect.SetSelected(node.SearchArea.Name)
			//			u.st.tabs.Items[5]. //image search tab
			//				Content.(*fyne.Container). //settings border
			//				Objects[1].(*fyne.Container). //2nd grid with columns
			//				Objects[1].(*fyne.Container). //vbox
			//				Objects[1].(*widget.Select).SetSelected(node.SearchBox.Name)

			u.st.tabs.SelectIndex(imagesearchtab)
		case *actions.Ocr:
			u.st.boundOCRTarget.Set(node.Target)
			u.st.boundOCRSearchBoxSelect.SetSelected(node.SearchArea.Name)
			u.st.tabs.SelectIndex(ocrtab)
		}
	}
}

func (u *Ui) createMacroToolbar() *widget.Toolbar {
	tb := widget.NewToolbar(
		widget.NewToolbarAction(theme.ContentAddIcon(), func() {
			switch u.st.tabs.Selected().Text {
			case "Wait":
				u.getCurrentTabMacro().addActionToTree(&actions.Wait{})
			case "Move":
				u.getCurrentTabMacro().addActionToTree(&actions.Move{})
			case "Click":
				u.getCurrentTabMacro().addActionToTree(&actions.Click{})
			case "Key":
				u.getCurrentTabMacro().addActionToTree(&actions.Key{})
			case "Loop":
				u.getCurrentTabMacro().addActionToTree(&actions.Loop{})
			case "Image":
				u.getCurrentTabMacro().addActionToTree(&actions.ImageSearch{})
			case "OCR":
				u.getCurrentTabMacro().addActionToTree(&actions.Ocr{})
			}
		}),
		widget.NewToolbarAction(theme.ViewRefreshIcon(), func() {
			node := u.getCurrentTabMacro().findNode(u.getCurrentTabMacro().Macro.Root, selectedTreeItem)
			if selectedTreeItem == "" {
				log.Println("No node selected")
				return
			}
			og := node.String()
			switch node := node.(type) {
			//			case *actions.Wait:
			//				node.Time = time
			//			case *actions.Move:
			//				node.X = moveX
			//				node.Y = moveY
			//			case *actions.Click:
			//				if !button {
			//					node.Button = "left"
			//				} else {
			//					node.Button = "right"
			//				}
			//			case *actions.Key:
			//				node.Key = key
			//				if !state {
			//					node.State = "down"
			//				} else {
			//					node.State = "up"
			//				}
			//			case *actions.Loop:
			//				node.Name = loopName
			//				node.Count = count
			case *actions.ImageSearch:
				var t []string
				for i, item := range imageSearchTargets {
					if item {
						t = append(t, i)
					}
				}
				node.Name = imageSearchName
				node.SearchArea = *data.GetSearchArea(searchArea)
				node.Targets = t
			}

			fmt.Printf("Updated node: %+v from '%v' to '%v' \n", node.GetUID(), og, node)

			u.getCurrentTabMacro().Tree.Refresh()
		}),
		widget.NewToolbarSpacer(),
		widget.NewToolbarSeparator(),
		widget.NewToolbarAction(theme.RadioButtonIcon(), func() {
			u.getCurrentTabMacro().Tree.UnselectAll()
			selectedTreeItem = ""
		}),
		widget.NewToolbarAction(theme.MoveDownIcon(), func() {
			u.getCurrentTabMacro().moveNodeDown(selectedTreeItem)
		}),
		widget.NewToolbarAction(theme.MoveUpIcon(), func() {
			u.getCurrentTabMacro().moveNodeUp(selectedTreeItem)
		}),
		widget.NewToolbarSeparator(),
		widget.NewToolbarSpacer(),
		widget.NewToolbarAction(theme.MediaPlayIcon(), func() {
			robotgo.ActiveName("Dark and Darker")
			u.getCurrentTabMacro().Macro.ExecuteActionTree()
		}),
		widget.NewToolbarAction(theme.DocumentSaveIcon(), func() {
			save := func() {
				err := encoding.GobSerializer.Encode(u.getCurrentTabMacro(), u.sel.Text)
				// err := u.getCurrentTabMacro().saveTreeToJsonFile(u.sel.Text)
				if err != nil {
					dialog.ShowError(err, u.win)
					log.Printf("encode tree to json: %v", err)
				} else {
					dialog.ShowInformation("File Saved Successfully", u.sel.Text+".json"+"\nPlease refresh the list.", u.win)
				}
			}
			if slices.Contains(u.sel.Options, u.sel.Text) {
				dialog.ShowConfirm("Overwrite existing file", "Overwrite "+u.sel.Text+"?", func(b bool) {
					if !b {
						return
					}
					save()
				}, u.win)
			} else {
				save()
			}
		}),
	)
	return tb
}

func (u *Ui) getCurrentTabMacro() *MacroTree {
	return u.mtm[u.dt.Selected().Text]
}

func (u *Ui) LoadMainContent() *fyne.Container {
	data.CreateItemMaps()
	u.createDocTabs()
	u.addMacroDocTab("Currency Testing")
	u.dt.SelectIndex(0)
	u.createSelect()
	u.dt.OnClosed = func(ti *container.TabItem) {
		delete(u.mtm, ti.Text)
	}
	u.win.SetMainMenu(u.createMainMenu())
	u.actionSettingsTabs()

	macroLayout := container.NewBorder(
		container.NewGridWithColumns(2,
			container.NewHBox(
				u.createMacroToolbar(),
				layout.NewSpacer(),
				widget.NewLabel("Macro Name:"),
			),
			container.NewBorder(nil, nil, nil, widget.NewButtonWithIcon("", theme.LoginIcon(), func() { u.addMacroDocTab(u.sel.Text) }), u.sel),
		),
		nil,
		widget.NewSeparator(),
		nil,
		u.dt,
	)
	mainLayout := container.NewBorder(nil, nil, u.st.tabs, nil, macroLayout)

	return mainLayout
}

func (u *Ui) addMacroDocTab(name string) {
	fp := savedMacrosPath + name
	if _, ok := u.mtm[name]; ok {
		return
	}
	m := &MacroTree{Macro: internal.NewMacro("", &actions.Loop{}, 30, "")}
	m.createTree()
	s, err := encoding.JsonSerializer.Decode(fp)
	if err != nil {
		dialog.ShowError(err, u.win)
		return
	}
	log.Println(s)
	result, err := encoding.JsonSerializer.CreateActionFromMap(s.(map[string]any), nil)
	// var result actions.ActionInterface
	log.Println(result)
	m.Macro.Root.SubActions = []actions.ActionInterface{}
	if s, ok := result.(*actions.Loop); ok { // fill Macro.Root / tree
		for _, sa := range s.SubActions {
			m.Macro.Root.AddSubAction(sa)
		}
	}
	if err != nil {
		fmt.Errorf("error unmarshalling tree: %v", err)
	}
	m.Tree.Refresh()
	u.mtm[name] = m

	t := container.NewTabItem(name, m.Tree)
	u.dt.Append(t)
	u.dt.Select(t)
	u.updateTreeOnselect()
}
