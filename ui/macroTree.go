package ui

import (
	"Squire/internal/programs/actions"
	"Squire/internal/programs/macro"
	"log"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/container"
	"fyne.io/fyne/v2/layout"
	"fyne.io/fyne/v2/theme"
	"fyne.io/fyne/v2/widget"
	hook "github.com/robotn/gohook"
)

type MacroTree struct {
	*widget.Tree
	Macro *macro.Macro
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
				mt.Refresh()
				if len(mt.Macro.Root.SubActions) == 0 {
					selectedTreeItem = ""
				}
			}
			removeButton.Show()
		} else {
			removeButton.Hide()
		}
	}
	mt.setUpdateTreeOnselect()
}

func (mt *MacroTree) setUpdateTreeOnselect() {
	mt.OnSelected = func(uid widget.TreeNodeID) {
		selectedTreeItem = uid
		switch node := mt.Macro.Root.GetAction(uid).(type) {
		case *actions.Wait:
			GetUi().at.wait.boundTime.Set(node.Time)
			GetUi().at.SelectIndex(waittab)
		case *actions.Move:
			GetUi().at.move.boundMoveX.Set(node.X)
			GetUi().at.move.boundMoveY.Set(node.Y)
			GetUi().at.SelectIndex(movetab)
		case *actions.Click:
			if node.Button == actions.LeftOrRight(false) {
				GetUi().at.click.boundButton.Set(false)
			} else {
				GetUi().at.click.boundButton.Set(true)
			}
			GetUi().at.SelectIndex(clicktab)
		case *actions.Key:
			key = node.Key
			GetUi().at.key.boundKeySelect.SetSelected(node.Key)
			if node.State == actions.UpOrDown(false) {
				GetUi().at.key.boundState.Set(false)
			} else {
				GetUi().at.key.boundState.Set(true)
			}
			GetUi().at.SelectIndex(keytab)

		case *actions.Loop:
			GetUi().at.loop.boundLoopName.Set(node.Name)
			GetUi().at.loop.boundCount.Set(node.Count)
			GetUi().at.SelectIndex(looptab)
		case *actions.ImageSearch:
			GetUi().at.imageSearch.boundImageSearchName.Set(node.Name)
			GetUi().at.imageSearch.boundImageSearchTargets.Set(node.Targets)
			GetUi().at.imageSearch.boundImageSearchAreaSelect.SetSelected(node.SearchArea.Name)
			GetUi().at.SelectIndex(imagesearchtab)
		case *actions.Ocr:
			GetUi().at.ocr.boundOCRTarget.Set(node.Target)
			GetUi().at.ocr.boundOCRSearchAreaSelect.SetSelected(node.SearchArea.Name)
			GetUi().at.SelectIndex(ocrtab)
		}
	}
}

func (mtree *MacroTree) RegisterHotkey() {
	hk := mtree.Macro.Hotkey
	log.Println("registering hotkey:", hk)
	hook.Register(hook.KeyDown, hk, func(e hook.Event) {
		log.Println("pressed", hk)
		mtree.Macro.ExecuteActionTree()
	})
}
func (mtree *MacroTree) UnregisterHotkey() {
	hk := mtree.Macro.Hotkey
	log.Println("unregistering hotkey:", hk)
	hook.Unregister(hook.KeyDown, hk)
}
