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

var advancedActionNameEntry = widget.NewEntry()

// ***************************************************************************************Wait
func createWaitActionSettings() *fyne.Container {
	millisecondsWaitEntry := widget.NewEntry()
	addWaitActionButton := &widget.Button{
		Text: utils.GetEmoji("Wait") + "Add Wait",
		OnTapped: func() {
			selectedNode := findNode(root, selectedTreeItem)
			if selectedNode == nil {
				selectedNode = root
			}
			wait, _ := strconv.Atoi(millisecondsWaitEntry.Text)
			wa := &structs.WaitAction{Time: wait, BaseAction: structs.NewBaseAction()}

			if s, ok := selectedNode.(structs.AdvancedActionInterface); ok {
				s.AddSubAction(wa)
			} else {
				selectedNode.GetParent().AddSubAction(wa)
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
		//structs.GetSpot(spotSelector.Selected)
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
			if selectedNode == nil {
				selectedNode = root
			}
			mma := &structs.MouseMoveAction{X: x, Y: y, BaseAction: structs.NewBaseAction()}
			if s, ok := selectedNode.(structs.AdvancedActionInterface); ok {
				s.AddSubAction(mma)
			} else {
				selectedNode.GetParent().AddSubAction(mma)
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
		Options:    []string{"left", "right"},
		Selected:   "left",
	}
	addClickActionButton := &widget.Button{
		Text: utils.GetEmoji("Click") + "Add Click",
		OnTapped: func() {
			selectedNode := findNode(root, selectedTreeItem)
			if selectedNode == nil {
				selectedNode = root
			}
			ca := &structs.ClickAction{Button: mouseButtonRadioGroup.Selected, BaseAction: structs.NewBaseAction()}
			if s, ok := selectedNode.(structs.AdvancedActionInterface); ok {
				s.AddSubAction(ca)
			} else {
				selectedNode.GetParent().AddSubAction(ca)
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

// ***************************************************************************************Key
func createKeySettings() *fyne.Container {
	keyUpDownRadioGroup := &widget.RadioGroup{
		Horizontal: true,
		Required:   true,
		Options:    []string{"Up", "Down"},
		Selected:   "Down",
	}
	keySelector := widget.NewSelect([]string{"ctrl", "alt", "shift"}, func(s string) {})
	addKeyPressActionButton := &widget.Button{
		Text: utils.GetEmoji("Key") + "Add Key",
		OnTapped: func() {
			selectedNode := findNode(root, selectedTreeItem)
			if selectedNode == nil {
				selectedNode = root
			}
			ka := &structs.KeyAction{Key: keySelector.Selected, State: keyUpDownRadioGroup.Selected, BaseAction: structs.NewBaseAction()}
			if s, ok := selectedNode.(structs.AdvancedActionInterface); ok {
				s.AddSubAction(ka)
			} else {
				selectedNode.GetParent().AddSubAction(ka)
			}
			updateTree(&tree, root)
		},
		IconPlacement: widget.ButtonIconPlacement(widget.ButtonAlignTrailing),
		Icon:          theme.NavigateNextIcon(),
		Importance:    widget.HighImportance,
	}
	return container.NewHBox(
		keySelector,
		layout.NewSpacer(),
		keyUpDownRadioGroup,
		addKeyPressActionButton,
	)
}

// ***************************************************************************************Advanced Actions

func createAdvancedActionSettings() *fyne.Container {
	return container.NewVBox(
		//container.NewGridWithColumns(2,
		container.NewGridWithColumns(2,
			widget.NewLabel("Search Area:"),
			searchAreaSelector,
		),
		container.NewGridWithColumns(2,
			widget.NewLabel("Name:"),
			advancedActionNameEntry,
		),
		//),
	)
}

// ***************************************************************************************Loop
func createLoopActionSettings() *fyne.Container {
	loops := widget.NewSlider(1, 10)

	addLoopActionButton := &widget.Button{
		Text: utils.GetEmoji("Loop") + "Add Loop",
		OnTapped: func() {
			selectedNode := findNode(root, selectedTreeItem)
			if selectedNode == nil {
				selectedNode = root
			}
			la := &structs.LoopAction{
				Count: int(loops.Value),
				AdvancedAction: structs.AdvancedAction{
					BaseAction: structs.NewBaseAction(),
					Name:       advancedActionNameEntry.Text,
				},
			}
			if s, ok := selectedNode.(structs.AdvancedActionInterface); ok {
				s.AddSubAction(la)
			} else {
				selectedNode.GetParent().AddSubAction(la)
			}
			updateTree(&tree, root)
		},
		IconPlacement: widget.ButtonIconPlacement(widget.ButtonAlignTrailing),
		Icon:          theme.NavigateNextIcon(),
		Importance:    widget.HighImportance,
	}

	return container.NewVBox(
		container.NewGridWithColumns(2,
			container.NewGridWithColumns(2,
				widget.NewLabel("Loops:"),
				loops,
			),
			container.NewGridWithColumns(2,
				layout.NewSpacer(),
				addLoopActionButton,
			),
		),
	)
}

// ***************************************************************************************Image Search
func createImageSearchSettings() *fyne.Container {
	addImageSearchActionButton := &widget.Button{
		Text: utils.GetEmoji("Image Search") + "Add Image Search",
		OnTapped: func() {
			selectedNode := findNode(root, selectedTreeItem)
			if selectedNode == nil {
				selectedNode = root
			}
			isa := &structs.ImageSearchAction{
				SearchBox: *structs.GetSearchBox(searchAreaSelector.Selected),
				Targets:   selectedItems(),
				AdvancedAction: structs.AdvancedAction{
					BaseAction: structs.NewBaseAction(),
					Name:       advancedActionNameEntry.Text,
				},
			}
			if s, ok := selectedNode.(structs.AdvancedActionInterface); ok {
				s.AddSubAction(isa)
			} else {
				selectedNode.GetParent().AddSubAction(isa)
			}

			updateTree(&tree, root)
		},
		IconPlacement: widget.ButtonIconPlacement(widget.ButtonAlignTrailing),
		Icon:          theme.NavigateNextIcon(),
		Importance:    widget.HighImportance,
	}
	return container.NewHBox(
		layout.NewSpacer(),
		addImageSearchActionButton,
	)
}

// ***************************************************************************************OCR
func createOCRSettings() *fyne.Container {
	textToSearch := widget.NewEntry()
	addOCRActionButton := &widget.Button{
		Text: utils.GetEmoji("OCR") + "Add OCR",
		OnTapped: func() {
			selectedNode := findNode(root, selectedTreeItem)
			if selectedNode == nil {
				selectedNode = root
			}
			ocra := &structs.OcrAction{
				SearchBox: *structs.GetSearchBox(searchAreaSelector.Selected),
				Target:    textToSearch.Text,
				AdvancedAction: structs.AdvancedAction{
					BaseAction: structs.NewBaseAction(),
					Name:       advancedActionNameEntry.Text,
				},
			}
			if s, ok := selectedNode.(structs.AdvancedActionInterface); ok {
				s.AddSubAction(ocra)
			} else {
				selectedNode.GetParent().AddSubAction(ocra)
			}
			updateTree(&tree, root)
		},
		IconPlacement: widget.ButtonIconPlacement(widget.ButtonAlignTrailing),
		Icon:          theme.NavigateNextIcon(),
		Importance:    widget.HighImportance,
	}
	return container.NewGridWithColumns(3,
		widget.NewLabel("Text to search:"),
		textToSearch,
		addOCRActionButton,
	)
}
