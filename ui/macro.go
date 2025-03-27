package ui

import (
	"Squire/internal/programs"
	"Squire/internal/programs/actions"
	"Squire/internal/programs/macro"
	"errors"
	"fmt"
	"log"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/container"
	"fyne.io/fyne/v2/layout"
	"fyne.io/fyne/v2/theme"
	"fyne.io/fyne/v2/widget"
)

type MacroTree struct {
	Macro *macro.Macro
	Tree  *widget.Tree

	// boundMacroName binding.String
}

func (u *Ui) GetMacroTabMacroTree() (*MacroTree, error) {
	mt, err := u.selectedMacroTab()
	if err != nil {
		return nil, err
	}
	if mt == nil {
		return nil, errors.New("macroTree is nil")
	}
	if mt.Tree == nil {
		return nil, errors.New("macroTree Tree is nil")
	}
	if mt.Macro == nil {
		return nil, errors.New("macroTree Macro is nil")
	}
	if mt.Macro.Root == nil {
		return nil, errors.New("macroTree Macro Root is nil")
	}
	return mt, nil
}

func (mt *MacroTree) moveNode(selectedUID string, up bool) {
	node := mt.Macro.Root.GetAction(selectedUID)
	if node == nil || node.GetParent() == nil {
		return
	}

	parent := node.GetParent()
	psa := parent.GetSubActions()
	index := -1
	for i, child := range psa {
		if child == node {
			index = i
			break
		}
	}

	if up && index > 0 {
		psa[index-1], psa[index] = psa[index], psa[index-1]
		mt.Tree.Select(psa[index-1].GetUID())
	} else if !up && index < len(psa)-1 {
		psa[index], psa[index+1] = psa[index+1], psa[index]
		mt.Tree.Select(psa[index+1].GetUID())
	}
	mt.Tree.Refresh()
}

func (mt *MacroTree) createTree() {
	log.Println("Creating tree")
	mt.Tree.ChildUIDs = func(uid string) []string {
		node := mt.Macro.Root.GetAction(uid)
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
	mt.Tree.IsBranch = func(uid string) bool {
		node := mt.Macro.Root.GetAction(uid)
		_, ok := node.(actions.AdvancedActionInterface)
		return node != nil && ok
	}
	mt.Tree.CreateNode = func(branch bool) fyne.CanvasObject {
		return container.NewHBox(widget.NewLabel("Template"), layout.NewSpacer(), &widget.Button{Icon: theme.CancelIcon(), Importance: widget.DangerImportance})
	}
	mt.Tree.UpdateNode = func(uid string, branch bool, obj fyne.CanvasObject) {
		node := mt.Macro.Root.GetAction(uid)
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
				mt.Tree.Refresh()
				if len(mt.Macro.Root.SubActions) == 0 {
					selectedTreeItem = ""
				}
			}
			removeButton.Show()
		} else {
			removeButton.Hide()
		}
	}
}
func (mt *MacroTree) updateTreeOnselect() {
	mt.Tree.OnSelected = func(uid widget.TreeNodeID) {
		selectedTreeItem = uid
		switch node := mt.Macro.Root.GetAction(uid).(type) {
		case *actions.Wait:
			GetUi().st.boundTime.Set(node.Time)
			GetUi().st.tabs.SelectIndex(waittab)
		case *actions.Move:
			GetUi().st.boundMoveX.Set(node.X)
			GetUi().st.boundMoveY.Set(node.Y)
			GetUi().st.tabs.SelectIndex(movetab)
		case *actions.Click:
			if node.Button == actions.LeftOrRight(false) {
				GetUi().st.boundButton.Set(false)
			} else {
				GetUi().st.boundButton.Set(true)
			}
			GetUi().st.tabs.SelectIndex(clicktab)
		case *actions.Key:
			key = node.Key
			GetUi().st.boundKeySelect.SetSelected(node.Key)
			if node.State == actions.UpOrDown(false) {
				GetUi().st.boundState.Set(false)
			} else {
				GetUi().st.boundState.Set(true)
			}
			GetUi().st.tabs.SelectIndex(keytab)

		case *actions.Loop:
			GetUi().st.boundLoopName.Set(node.Name)
			GetUi().st.boundCount.Set(node.Count)
			GetUi().st.tabs.SelectIndex(looptab)
		case *actions.ImageSearch:
			GetUi().st.boundImageSearchName.Set(node.Name)
			for t := range imageSearchTargets {
				imageSearchTargets[t] = false
			}
			for _, t := range node.Targets {
				imageSearchTargets[t] = true
			}
			mt.Tree.Refresh()
			GetUi().st.boundImageSearchAreaSelect.SetSelected(node.SearchArea.Name)
			GetUi().st.tabs.SelectIndex(imagesearchtab)
		case *actions.Ocr:
			GetUi().st.boundOCRTarget.Set(node.Target)
			GetUi().st.boundOCRSearchBoxSelect.SetSelected(node.SearchArea.Name)
			GetUi().st.tabs.SelectIndex(ocrtab)
		}
	}
}

func (u *Ui) createMacroToolbar() *widget.Toolbar {
	tb := widget.NewToolbar(
		widget.NewToolbarAction(theme.ContentAddIcon(), func() {
			var (
				action actions.ActionInterface
			)
			mt, err := u.GetMacroTabMacroTree()
			if err != nil {
				log.Println(err)
				return
			}
			selectedNode := mt.Macro.Root.GetAction(selectedTreeItem)
			if selectedNode == nil {
				log.Println("Could not find action: ", selectedTreeItem)
				return
			}
			switch u.st.tabs.Selected().Text {
			case "Wait":
				action = actions.NewWait(time)
			case "Move":
				action = actions.NewMove(moveX, moveY)
			case "Click":
				action = actions.NewClick(actions.LeftOrRight(button))
			case "Key":
				action = actions.NewKey(key, actions.UpOrDown(state))
			case "Loop":
				action = actions.NewLoop(int(count), loopName, []actions.ActionInterface{})
			case "Image":
				var t []string
				for i, item := range imageSearchTargets {
					if item {
						t = append(t, i)
					}
				}
				action = actions.NewImageSearch(imageSearchName, []actions.ActionInterface{}, t, programs.CurrentProgramAndScreenSizeCoordinates().GetSearchArea(searchArea))
			case "OCR":
				action = actions.NewOcr(ocrTarget, []actions.ActionInterface{}, ocrTarget, programs.CurrentProgramAndScreenSizeCoordinates().GetSearchArea(ocrSearchBox))
			}

			if selectedNode == nil {
				selectedNode = mt.Macro.Root
			}
			if s, ok := selectedNode.(actions.AdvancedActionInterface); ok {
				s.AddSubAction(action)
			} else {
				selectedNode.GetParent().AddSubAction(action)
			}

			mt.Tree.Refresh()
		}),
		widget.NewToolbarAction(theme.ViewRefreshIcon(), func() {
			mt, err := u.GetMacroTabMacroTree()
			if err != nil {
				log.Println(err)
				return
			}
			if selectedTreeItem == "" {
				log.Println("No node selected")
				return
			}
			node := mt.Macro.Root.GetAction(selectedTreeItem)
			if node == nil {
				log.Println("could not find action:", selectedTreeItem)
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
				node.SearchArea = programs.CurrentProgramAndScreenSizeCoordinates().GetSearchArea(searchArea)
				node.Targets = t
			}

			fmt.Printf("Updated node: %+v from '%v' to '%v' \n", node.GetUID(), og, node)

			mt.Tree.Refresh()
		}),
		widget.NewToolbarSpacer(),
		widget.NewToolbarSeparator(),
		widget.NewToolbarAction(theme.RadioButtonIcon(), func() {
			t, err := u.GetMacroTabMacroTree()
			if err != nil {
				log.Println(err)
				return
			}
			t.Tree.UnselectAll()
			selectedTreeItem = ""
		}),
		widget.NewToolbarAction(theme.MoveDownIcon(), func() {
			t, err := u.GetMacroTabMacroTree()
			if err != nil {
				log.Println(err)
				return
			}
			t.moveNode(selectedTreeItem, false)
		}),
		widget.NewToolbarAction(theme.MoveUpIcon(), func() {
			t, err := u.GetMacroTabMacroTree()
			if err != nil {
				log.Println(err)
				return
			}
			t.moveNode(selectedTreeItem, true)

		}),
		widget.NewToolbarSeparator(),
		widget.NewToolbarSpacer(),
		widget.NewToolbarAction(theme.MediaPlayIcon(), func() {
			t, err := u.GetMacroTabMacroTree()
			if err != nil {
				log.Println(err)
				return
			}
			t.Macro.ExecuteActionTree()
		}),
		// widget.NewToolbarAction(theme.DocumentSaveIcon(), func() {
		// 	save := func() {
		// 		t, err := u.GetMacroTabMacroTree()
		// 		if err != nil {
		// 			log.Println(err)
		// 			return
		// 		}
		// 		str := "programs" + "." + config.DarkAndDarker + "." + "macros"
		// 		for _, m := range config.ViperConfig.Get(str).([]any) {
		// 			log.Println("string slice text", m)
		// 			log.Println("select text", u.sel.Text)
		// 			if m.(map[string]any)["name"] == u.sel.Text {
		// 				config.ViperConfig.Set(str+"."+"0", t.Macro)
		// 				config.ViperConfig.WriteConfig()
		// 				dialog.ShowInformation("Macro Saved Successfully", u.sel.Text+"\nPlease refresh the list.", u.win)
		// 				return
		// 			}
		// 		}
		// 		dialog.ShowInformation("Macro failed to save", "Macro not found in config", u.win)
		// 	}
		// 	if slices.Contains(u.sel.Options, u.sel.Text) {
		// 		dialog.ShowConfirm("Overwrite existing file", "Overwrite "+u.sel.Text+"?", func(b bool) {
		// 			if !b {
		// 				return
		// 			}
		// 			save()
		// 		}, u.win)
		// 	} else {
		// 		save()
		// 	}
		// }),
	)
	return tb
}
