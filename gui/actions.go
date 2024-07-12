package gui

import (
	"Dark-And-Darker/structs"
	"Dark-And-Darker/utils"
	"log"
	"strconv"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/container"
	"fyne.io/fyne/v2/layout"
	"fyne.io/fyne/v2/theme"
	"fyne.io/fyne/v2/widget"
)

// ***************************************************************************************Wait
func createWaitActionSettings() *fyne.Container {
	millisecondsWaitEntry := widget.NewEntry()
	addWaitActionButton := &widget.Button{
		Text: utils.GetEmoji("Wait") + "Add Wait",
		OnTapped: func() {
			selectedNode := findNode(root, selectedTreeItem)
			if selectedNode != nil {
				wait, _ := strconv.Atoi(millisecondsWaitEntry.Text)
				wa := &structs.WaitAction{Time: wait, BaseAction: structs.BaseAction{UID: "ZZZ"}}

				if s, ok := selectedNode.(structs.ActionWithSubActionsInterface); ok {
					s.AddSubAction(wa, wa.String())
				} else {
					selectedNode.GetParent().AddSubAction(wa, wa.String())
				}
			}
			updateTree(&tree, root)
		},
		IconPlacement: widget.ButtonIconPlacement(widget.ButtonAlignTrailing),
		Icon:          theme.NavigateNextIcon(),
		Importance:    widget.HighImportance,
	}
	return container.NewGridWithColumns(4,
		layout.NewSpacer(),
		widget.NewLabel("Wait in ms"),
		millisecondsWaitEntry,
		addWaitActionButton,
	)
}

// ***************************************************************************************Move
func createMouseMoveSettings() *fyne.Container {
	mouseMoveXEntry := widget.NewEntry()
	mouseMoveYEntry := widget.NewEntry()
	spotSelector := &widget.Select{Options: *structs.GetSpotMapKeys(*structs.GetSpotMap())}
	spotSelector.OnChanged = func(s string) {
		structs.GetSpot("Search Area Selector Info:")
		log.Println(*structs.GetSpot(s))
		mouseMoveXEntry.SetText(strconv.FormatInt(int64(structs.GetSpot(s).Coordinates.X), 10))
		mouseMoveYEntry.SetText(strconv.FormatInt(int64(structs.GetSpot(s).Coordinates.Y), 10))
	}
	spotSelector.SetSelected(spotSelector.Options[0])

	addMouseMoveActionButton := &widget.Button{
		Text: utils.GetEmoji("Move") + "Add Move",
		OnTapped: func() {
			selectedNode := findNode(root, selectedTreeItem)
			x, _ := strconv.Atoi(mouseMoveXEntry.Text)
			y, _ := strconv.Atoi(mouseMoveYEntry.Text)
			if selectedNode != nil {
				mma := &structs.MouseMoveAction{X: x, Y: y, BaseAction: structs.BaseAction{UID: "ZZZ"}}
				if s, ok := selectedNode.(structs.ActionWithSubActionsInterface); ok {
					s.AddSubAction(mma, mma.String())
				} else {
					selectedNode.GetParent().AddSubAction(mma, mma.String())
				}
			}
			updateTree(&tree, root)
		},
		IconPlacement: widget.ButtonIconPlacement(widget.ButtonAlignTrailing),
		Icon:          theme.NavigateNextIcon(),
		Importance:    widget.HighImportance,
	}
	return container.NewVBox(
		container.NewGridWithColumns(4,
			spotSelector,
			container.NewGridWithColumns(2,
				container.NewHBox(layout.NewSpacer(), widget.NewLabel("X:")),
				mouseMoveXEntry,
			),
			container.NewGridWithColumns(2,
				container.NewHBox(layout.NewSpacer(), widget.NewLabel("Y:")),
				mouseMoveYEntry,
			),
			addMouseMoveActionButton,
		),
	)
}

// ***************************************************************************************Click
func createClickSettings() *fyne.Container {

	mouseButtonRadioGroup := &widget.RadioGroup{
		Horizontal: true,
		Required:   false,
		Options:    []string{"Left", "Right"},
		Selected:   "Left",
	}
	addClickActionButton := &widget.Button{
		Text: utils.GetEmoji("Click") + "Add Click",
		OnTapped: func() {
			selectedNode := findNode(root, selectedTreeItem)
			if selectedNode != nil {
				ca := &structs.ClickAction{Button: mouseButtonRadioGroup.Selected, BaseAction: structs.BaseAction{UID: "ZZZ"}}
				if s, ok := selectedNode.(structs.ActionWithSubActionsInterface); ok {
					s.AddSubAction(ca, ca.String())
				} else {
					selectedNode.GetParent().AddSubAction(ca, ca.String())
				}
			}
			updateTree(&tree, root)
		},
		IconPlacement: widget.ButtonIconPlacement(widget.ButtonAlignTrailing),
		Icon:          theme.NavigateNextIcon(),
		Importance:    widget.HighImportance,
	}
	return container.NewHBox(
		layout.NewSpacer(),
		mouseButtonRadioGroup,
		addClickActionButton,
	)
}

// // ***************************************************************************************Key
// func createKeySettings() *fyne.Container {
// 	keyUpDownRadioGroup := &widget.RadioGroup{
// 		Horizontal: true,
// 		Required:   true,
// 		Options:    []string{"Up", "Down"},
// 		Selected:   "Down",
// 	}
// 	addKeyPressActionButton := &widget.Button{
// 		Text: utils.GetEmoji("Key") + "Add Key",
// 		OnTapped: func() {
// 			selectedNode := findNode(root, selectedTreeItem)
// 			if selectedNode != nil {
// 				if _, ok := selectedNode.(*Node); ok {
// 					newActionNode(selectedNode.(*Node), &structs.KeyAction{Key: "Enter", State: keyUpDownRadioGroup.Selected})
// 				} else {
// 					newActionNode(selectedNode.GetParent(), &structs.KeyAction{Key: "Enter", State: keyUpDownRadioGroup.Selected})
// 				}
// 			}
// 			updateTree(&tree, root)
// 		},
// 		IconPlacement: widget.ButtonIconPlacement(widget.ButtonAlignTrailing),
// 		Icon:          theme.NavigateNextIcon(),
// 		Importance:    widget.HighImportance,
// 	}
// 	return container.NewHBox(
// 		widget.NewSelect([]string{"ctrl", "alt", "shift"}, func(s string) {}),
// 		layout.NewSpacer(),
// 		keyUpDownRadioGroup,
// 		addKeyPressActionButton,
// 	)
// }

// // ***************************************************************************************Search settings

// // ***************************************************************************************Image Search
// func createImageSearchSettings() *fyne.Container {
// 	addImageSearchActionButton := &widget.Button{
// 		Text: utils.GetEmoji("Image Search") + "Add Image Search",
// 		OnTapped: func() {
// 			selectedNode := findNode(root, selectedTreeItem)
// 			if selectedNode != nil {
// 				if _, ok := selectedNode.(*Node); ok {
// 					newActionNode(selectedNode.(*Node), &structs.ImageSearchAction{SearchBox: *structs.GetSearchBox(searchAreaSelector.Selected), Targets: selectedItems()})
// 				} else {
// 					newActionNode(selectedNode.GetParent(), &structs.ImageSearchAction{SearchBox: *structs.GetSearchBox(searchAreaSelector.Selected), Targets: selectedItems()})
// 				}
// 			}
// 			updateTree(&tree, root)
// 		},
// 		IconPlacement: widget.ButtonIconPlacement(widget.ButtonAlignTrailing),
// 		Icon:          theme.NavigateNextIcon(),
// 		Importance:    widget.HighImportance,
// 	}
// 	return container.NewHBox(
// 		addImageSearchActionButton,
// 	)
// }

// // ***************************************************************************************OCR
// func createOCRSettings() *fyne.Container {
// 	addOCRActionButton := &widget.Button{
// 		Text: utils.GetEmoji("OCR") + "Add OCR",
// 		OnTapped: func() {
// 			selectedNode := findNode(root, selectedTreeItem)
// 			if selectedNode != nil {
// 				if _, ok := selectedNode.(*Node); ok {
// 					newActionNode(selectedNode.(*Node), &structs.OcrAction{SearchBox: *structs.GetSearchBox(searchAreaSelector.Selected)})
// 				} else {
// 					newActionNode(selectedNode.GetParent(), &structs.OcrAction{SearchBox: *structs.GetSearchBox(searchAreaSelector.Selected)})
// 				}
// 			}
// 			updateTree(&tree, root)
// 		},
// 		IconPlacement: widget.ButtonIconPlacement(widget.ButtonAlignTrailing),
// 		Icon:          theme.NavigateNextIcon(),
// 		Importance:    widget.HighImportance,
// 	}
// 	return container.NewHBox(
// 		addOCRActionButton,
// 	)
// }
