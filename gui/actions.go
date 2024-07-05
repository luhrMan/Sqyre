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
			if selectedTreeItem == "" {
				return
			}
			selectedNode := findNode(root, selectedTreeItem)
			if selectedNode != nil && selectedNode.Type == SequenceType {
				wait, _ := strconv.Atoi(millisecondsWaitEntry.Text)
				NewAction(selectedNode, &structs.WaitAction{Time: wait})
				updateTree(&tree, root)
			}
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
			if selectedTreeItem == "" {
				return
			}
			selectedNode := findNode(root, selectedTreeItem)
			if selectedNode != nil && selectedNode.Type == SequenceType {
				x, _ := strconv.Atoi(mouseMoveXEntry.Text)
				y, _ := strconv.Atoi(mouseMoveYEntry.Text)
				NewAction(selectedNode, &structs.MouseMoveAction{X: x, Y: y})
				updateTree(&tree, root)
			}
		},
		IconPlacement: widget.ButtonIconPlacement(widget.ButtonAlignTrailing),
		Icon:          theme.NavigateNextIcon(),
		Importance:    widget.HighImportance,
	}
	return container.NewVBox(
		spotSelector,
		container.NewGridWithColumns(4,
			widget.NewLabel(""),
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
			if selectedTreeItem == "" {
				return
			}
			selectedNode := findNode(root, selectedTreeItem)
			if selectedNode != nil && selectedNode.Type == SequenceType {

				NewAction(selectedNode, &structs.ClickAction{Button: mouseButtonRadioGroup.Selected})
				updateTree(&tree, root)
			}
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

// ***************************************************************************************Key
func createKeySettings() *fyne.Container {

	keyUpDownRadioGroup := &widget.RadioGroup{
		Horizontal: true,
		Required:   true,
		Options:    []string{"Up", "Down"},
		Selected:   "Down",
	}

	addKeyPressActionButton := &widget.Button{
		Text: utils.GetEmoji("Key") + "Add Key",
		OnTapped: func() {
			if selectedTreeItem == "" {
				return
			}
			selectedNode := findNode(root, selectedTreeItem)
			if selectedNode != nil && selectedNode.Type == SequenceType {
				NewAction(selectedNode, &structs.KeyAction{Key: "Enter", State: keyUpDownRadioGroup.Selected})
				updateTree(&tree, root)
			}
		},
		IconPlacement: widget.ButtonIconPlacement(widget.ButtonAlignTrailing),
		Icon:          theme.NavigateNextIcon(),
		Importance:    widget.HighImportance,
	}
	return container.NewHBox(
		layout.NewSpacer(),
		widget.NewSelect([]string{"ctrl", "alt"}, func(s string) {}),
		keyUpDownRadioGroup,
		addKeyPressActionButton,
	)
}

// ***************************************************************************************Search settings
func createSearchAreaSelector() *fyne.Container {
	searchAreaSelector := &widget.Select{Options: *structs.GetSearchBoxMapKeys(*structs.GetSearchBoxMap())}
	searchAreaSelector.SetSelected(searchAreaSelector.Options[0])

	return container.NewWithoutLayout(searchAreaSelector)
}

// ***************************************************************************************Image Search
func createImageSearchSettings() *fyne.Container {
	addImageSearchActionButton := &widget.Button{
		Text: utils.GetEmoji("Image Search") + "Add Image Search",
		OnTapped: func() {
			if selectedTreeItem == "" {
				return
			}
			selectedNode := findNode(root, selectedTreeItem)
			if selectedNode != nil && selectedNode.Type == SequenceType {
				NewAction(selectedNode, &structs.ImageSearchAction{})
				updateTree(&tree, root)
			}
		},
		IconPlacement: widget.ButtonIconPlacement(widget.ButtonAlignTrailing),
		Icon:          theme.NavigateNextIcon(),
		Importance:    widget.HighImportance,
	}
	return container.NewHBox(
		addImageSearchActionButton,
	)
}

// ***************************************************************************************OCR
func createOCRSettings() *fyne.Container {

	addOCRActionButton := &widget.Button{
		Text: utils.GetEmoji("OCR") + "Add OCR",
		OnTapped: func() {
			if selectedTreeItem == "" {
				return
			}
			selectedNode := findNode(root, selectedTreeItem)
			if selectedNode != nil && selectedNode.Type == SequenceType {
				NewAction(selectedNode, &structs.OcrAction{})
				updateTree(&tree, root)
			}
		},
		IconPlacement: widget.ButtonIconPlacement(widget.ButtonAlignTrailing),
		Icon:          theme.NavigateNextIcon(),
		Importance:    widget.HighImportance,
	}
	return container.NewHBox(
		addOCRActionButton,
	)
}
