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

type MacroTree struct {
	Macro *internal.Macro
	Tree  *widget.Tree

	boundMacroName binding.String
}

func (u *Ui) GetMacroTabMacroTree() *MacroTree {
	mt, err := u.selectedMacroTab()
	if err != nil {
		log.Println(err)
		return nil
	}
	if mt == nil {
		log.Println("MacroTree is nil")
		return nil
	}
	return mt
}
func (u *Ui) GetMacroTabMacroTreeMacro() *internal.Macro {
	mt := u.GetMacroTabMacroTree()
	if mt == nil {
		return nil
	}
	if mt.Macro == nil {
		log.Println("MacroTree Macro is nil")
		return nil
	}
	return mt.Macro
}

func (mt *MacroTree) moveNode(selectedUID string, up bool) {

	if mt == nil {
		return
	}

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
func (u *Ui) updateTreeOnselect() {
	u.selectedMacroTab().Tree.OnSelected = func(uid widget.TreeNodeID) {
		selectedTreeItem = uid
		switch node := u.selectedMacroTab().Macro.Root.GetAction(uid).(type) {
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
			var (
				// selectedNode, err = u.selectedMacroTab().Macro.Root.GetAction(selectedTreeItem)
				action actions.ActionInterface
			)
			mt, err := u.selectedMacroTab()
			if err != nil {
				log.Println(err)
				return
			}
			if mt.Macro.Root.GetAction(selectedTreeItem) == nil {

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
				action = actions.NewImageSearch(imageSearchName, []actions.ActionInterface{}, t, *data.GetSearchArea(searchArea))
			case "OCR":
				action = actions.NewOcr(ocrTarget, []actions.ActionInterface{}, ocrTarget, *data.GetSearchArea(ocrSearchBox))
			}

			if selectedNode == nil {
				selectedNode = u.selectedMacroTab().Macro.Root
			}
			if s, ok := selectedNode.(actions.AdvancedActionInterface); ok {
				s.AddSubAction(action)
			} else {
				selectedNode.GetParent().AddSubAction(action)
			}
			u.selectedMacroTab().Tree.Refresh()
		}),
		widget.NewToolbarAction(theme.ViewRefreshIcon(), func() {
			node := u.selectedMacroTab().Macro.Root.GetAction(selectedTreeItem)
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
			if u.selectedMacroTab() == nil {
				return
			}
			u.selectedMacroTab().Tree.UnselectAll()
			selectedTreeItem = ""
		}),
		widget.NewToolbarAction(theme.MoveDownIcon(), func() {
			u.selectedMacroTab().moveNode(selectedTreeItem, false)
		}),
		widget.NewToolbarAction(theme.MoveUpIcon(), func() {
			u.selectedMacroTab().moveNode(selectedTreeItem, true)

		}),
		widget.NewToolbarSeparator(),
		widget.NewToolbarSpacer(),
		widget.NewToolbarAction(theme.MediaPlayIcon(), func() {
			robotgo.ActiveName("Dark And Darker")
			u.selectedMacroTab().Macro.ExecuteActionTree()
		}),
		widget.NewToolbarAction(theme.DocumentSaveIcon(), func() {
			save := func() {
				err := encoding.GobSerializer.Encode(u.sel.Text, u.selectedMacroTab())
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
