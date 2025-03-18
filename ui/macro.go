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

// func NewMacroTree() MacroTree {
// 	m := MacroTree{}
// 	m.createTree()
// 	return m
// }

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
	// m.Macro.Root = actions.NewLoop(1, "root", []actions.ActionInterface{})
	// m.Macro.Root.SetUID("")

	// m.Tree = &widget.Tree{}

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
		action = actions.NewClick(actions.LeftOrRight(button))
	case *actions.Key:
		action = actions.NewKey(key, actions.UpOrDown(state))
	case *actions.Loop:
		action = actions.NewLoop(int(count), loopName, []actions.ActionInterface{})
	case *actions.ImageSearch:
		var t []string
		for i, item := range imageSearchTargets {
			if item {
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
	u.selectedMacroTab().Tree.OnSelected = func(uid widget.TreeNodeID) {
		selectedTreeItem = uid
		switch node := u.selectedMacroTab().findNode(u.selectedMacroTab().Macro.Root, uid).(type) {
		case *actions.Wait:
			u.st.boundTime.Set(node.Time)
			u.st.tabs.SelectIndex(waittab)
		case *actions.Move:
			u.st.boundMoveX.Set(node.X)
			u.st.boundMoveY.Set(node.Y)
			u.st.tabs.SelectIndex(movetab)
		case *actions.Click:
			if node.Button == actions.LeftOrRight(false) {
				u.st.boundButton.Set(false)
			} else {
				u.st.boundButton.Set(true)
			}
			u.st.tabs.SelectIndex(clicktab)
		case *actions.Key:
			key = node.Key
			u.st.boundKeySelect.SetSelected(node.Key)
			if node.State == actions.UpOrDown(false) {
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
			u.selectedMacroTab().Tree.Refresh()
			u.st.boundImageSearchAreaSelect.SetSelected(node.SearchArea.Name)
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
				u.selectedMacroTab().addActionToTree(&actions.Wait{})
			case "Move":
				u.selectedMacroTab().addActionToTree(&actions.Move{})
			case "Click":
				u.selectedMacroTab().addActionToTree(&actions.Click{})
			case "Key":
				u.selectedMacroTab().addActionToTree(&actions.Key{})
			case "Loop":
				u.selectedMacroTab().addActionToTree(&actions.Loop{})
			case "Image":
				u.selectedMacroTab().addActionToTree(&actions.ImageSearch{})
			case "OCR":
				u.selectedMacroTab().addActionToTree(&actions.Ocr{})
			}
		}),
		widget.NewToolbarAction(theme.ViewRefreshIcon(), func() {
			node := u.selectedMacroTab().findNode(u.selectedMacroTab().Macro.Root, selectedTreeItem)
			if selectedTreeItem == "" {
				log.Println("No node selected")
				return
			}
			og := node.String()
			switch node := node.(type) {
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

			u.selectedMacroTab().Tree.Refresh()
		}),
		widget.NewToolbarSpacer(),
		widget.NewToolbarSeparator(),
		widget.NewToolbarAction(theme.RadioButtonIcon(), func() {
			u.selectedMacroTab().Tree.UnselectAll()
			selectedTreeItem = ""
		}),
		widget.NewToolbarAction(theme.MoveDownIcon(), func() {
			u.selectedMacroTab().moveNodeDown(selectedTreeItem)
		}),
		widget.NewToolbarAction(theme.MoveUpIcon(), func() {
			u.selectedMacroTab().moveNodeUp(selectedTreeItem)
		}),
		widget.NewToolbarSeparator(),
		widget.NewToolbarSpacer(),
		widget.NewToolbarAction(theme.MediaPlayIcon(), func() {
			robotgo.ActiveName("Dark And Darker")
			u.selectedMacroTab().Macro.ExecuteActionTree()
		}),
		widget.NewToolbarAction(theme.DocumentSaveIcon(), func() {
			save := func() {
				err := encoding.GobSerializer.Encode(u.selectedMacroTab(), u.sel.Text)
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

func (u *Ui) selectedMacroTab() *MacroTree {
	return u.mtm[u.dt.Selected().Text]
}

func (u *Ui) addMacroDocTab(macro internal.Macro) {
	// fp := savedMacrosPath + name
	log.Println("macro: ", macro)
	log.Println("macro tree map: ", u.mtm)
	log.Println("macro tree: ", u.mtm[macro.Name])
	if _, ok := u.mtm[macro.Name]; !ok {
		return
	}
	mt := u.mtm[macro.Name]
	// m := &MacroTree{Macro: &macro}
	mt.createTree()

	// s, err := encoding.JsonSerializer.Decode(fp)
	// if err != nil {
	// 	dialog.ShowError(err, u.win)
	// 	return
	// }
	// result, err := encoding.JsonSerializer.CreateActionFromMap(s.(map[string]any), nil)

	// m.Macro.Root.SubActions = []actions.ActionInterface{}
	// if s, ok := result.(*actions.Loop); ok { // fill Macro.Root / tree
	// 	for _, sa := range s.SubActions {
	// 		m.Macro.Root.AddSubAction(sa)
	// 	}
	// }
	// if err != nil {
	// 	fmt.Errorf("error unmarshalling tree: %v", err)
	// }
	// u.mtm[name] = m

	t := container.NewTabItem(macro.Name, mt.Tree)
	u.dt.Append(t)
	u.dt.Select(t)
	u.updateTreeOnselect()
	mt.Tree.Refresh()
}
