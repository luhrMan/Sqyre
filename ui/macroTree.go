package ui

import (
	"Squire/internal/actions"
	"Squire/internal/structs"
	"log"

	"fyne.io/fyne/v2/data/binding"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/container"
	"fyne.io/fyne/v2/layout"
	"fyne.io/fyne/v2/theme"
	"fyne.io/fyne/v2/widget"
)

type Program struct {
	Macros      *[]Macro
	Items       *map[string]structs.Item
	Coordinates *map[[2]int]ScreenSize
}

// func newProgram() {
// 	ss := make(map[[2]int]ScreenSize)
// 	pm := make(map[string]structs.Point)
// 	sam := make(map[string]structs.SearchArea)
// 	ss[[2]int{10, 10}] = ScreenSize{
// 		Points:      &pm,
// 		SearchAreas: &sam,
// 	}

// 	p := &Program{
// 		Macros:     &[]Macro{},
// 		Items:      &map[string]structs.Item{},
// 		ScreenSize: &ss,
// 	}
// 	programs["Dark and Darker"] = *p
// 	log.Println(programs["Dark and Darker"].Macros)
// }

type ScreenSize struct {
	Points      *map[string]structs.Point
	SearchAreas *map[string]structs.SearchArea
}

type Macro struct {
	name   string
	Tree   *widget.Tree
	Root   *actions.Loop
	Hotkey string

	boundMacroName binding.String
}

func NewMacro() Macro {
	m := Macro{}
	m.createTree()
	return m
}

func (m *Macro) moveNodeUp(selectedUID string) {
	node := m.findNode(m.Root, selectedUID)
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

func (m *Macro) moveNodeDown(selectedUID string) {
	node := m.findNode(m.Root, selectedUID)
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

func (m *Macro) findNode(node actions.ActionInterface, uid string) actions.ActionInterface {
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

func (m *Macro) executeActionTree(ctx ...interface{}) { //error
	err := m.Root.Execute(ctx)
	if err != nil {
		log.Println(err)
		return
	}
}

func (m *Macro) createTree() {
	// macro := NewMacro()
	m.Root = actions.NewLoop(1, "root", []actions.ActionInterface{})
	m.Root.SetUID("")

	m.Tree = &widget.Tree{}

	m.Tree.ChildUIDs = func(uid string) []string {
		node := m.findNode(m.Root, uid)
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
		node := m.findNode(m.Root, uid)
		_, ok := node.(actions.AdvancedActionInterface)
		return node != nil && ok
	}
	m.Tree.CreateNode = func(branch bool) fyne.CanvasObject {
		return container.NewHBox(widget.NewLabel("Template"), layout.NewSpacer(), &widget.Button{Icon: theme.CancelIcon(), Importance: widget.DangerImportance})
	}
	m.Tree.UpdateNode = func(uid string, branch bool, obj fyne.CanvasObject) {
		node := m.findNode(m.Root, uid)
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
				m.Root.RenameActions() //should figure out how to rename the whole tree from RemoveSubActions
				m.Tree.Refresh()
				if len(m.Root.SubActions) == 0 {
					selectedTreeItem = ""
				}
			}
			removeButton.Show()
		} else {
			removeButton.Hide()
		}
	}
}

func (m *Macro) addActionToTree(actionType actions.ActionInterface) {
	var (
		selectedNode = m.findNode(m.Root, selectedTreeItem)
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
		action = actions.NewImageSearch(imageSearchName, []actions.ActionInterface{}, t, *structs.GetSearchArea(searchArea))
	case *actions.Ocr:
		action = actions.NewOcr(ocrTarget, []actions.ActionInterface{}, ocrTarget, *structs.GetSearchArea(ocrSearchBox))
	}

	if selectedNode == nil {
		selectedNode = m.Root
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
		switch node := u.getCurrentTabMacro().findNode(u.getCurrentTabMacro().Root, uid).(type) {
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
			u.st.tabs.SelectIndex(3)

		case *actions.Loop:
			u.st.boundLoopName.Set(node.Name)
			u.st.boundCount.Set(node.Count)
			u.st.tabs.SelectIndex(4)
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

			u.st.tabs.SelectIndex(5)
		case *actions.Ocr:
			u.st.boundOCRTarget.Set(node.Target)
			u.st.boundOCRSearchBoxSelect.SetSelected(node.SearchArea.Name)
			u.st.tabs.SelectIndex(6)
		}
	}
}

// func (m *Macro) saveTreeToJsonFile(filename string) error {
// 	if filename == "" {
// 		return fmt.Errorf("cannot save empty filename")
// 	}
// 	jsonData, err := json.MarshalIndent(m.root, "", "\t")
// 	if err != nil {
// 		return fmt.Errorf("error marshalling tree: %v", err)
// 	}
// 	filepath := path + filename + ".json"
// 	err = os.WriteFile(filepath, jsonData, 0644)
// 	if err != nil {
// 		return fmt.Errorf("error writing to file: %v", err)
// 	}
// 	return nil
// }

// func (m *Macro) loadTreeFromJsonFile(filename string) error {
// 	log.Printf("loadTreeFromJsonFile: attempting to read file %v", filename)
// 	jsonData, err := os.ReadFile(path + filename)
// 	if err != nil {
// 		return fmt.Errorf("error reading file: %v", err)
// 	}
// 	var result actions.ActionInterface
// 	//err = json.Unmarshal(jsonData, root)
// 	m.root.SubActions = []actions.ActionInterface{}
// 	result, err = UnmarshalJSON(jsonData)
// 	if s, ok := result.(*actions.Loop); ok { // fill root / tree
// 		for _, sa := range s.SubActions {
// 			m.root.AddSubAction(sa)
// 		}
// 	}
// 	if err != nil {
// 		return fmt.Errorf("error unmarshalling tree: %v", err)
// 	}
// 	m.tree.Refresh()
// 	return err
// }
