package gui

import (
	"Dark-And-Darker/structs"
	"log"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/canvas"
	"fyne.io/fyne/v2/container"
	"fyne.io/fyne/v2/theme"
	"fyne.io/fyne/v2/widget"
	"github.com/go-vgo/robotgo"
)

var (
	root               = &structs.LoopAction{}
	tree               = widget.Tree{}
	selectedTreeItem   string
	selectedItemsMap   = make(map[string]bool)
	searchAreaSelector = &widget.Select{Options: *structs.GetSearchBoxMapKeys(*structs.GetSearchBoxMap())}
)

func LoadMainContent() *container.Split {
	log.Println("Screen Size")
	log.Println(robotgo.GetScreenSize())
	log.Println("Monitor 1 size")
	log.Println(robotgo.GetDisplayBounds(0))
	log.Println("Monitor 2 size")
	log.Println(robotgo.GetDisplayBounds(1))
	root = newRootNode()
	updateTree(&tree, root)
	// loopAction := &structs.LoopAction{
	// 	ActionWithSubActions: structs.ActionWithSubActions{
	// 		BaseAction: structs.BaseAction{
	// 			UID: "1",
	// 			//Name: "first action",
	// 		},
	// 	},
	// 	Count: 1,
	// }

	//click merchants tab, click merchant
	root.AddSubAction(&structs.MouseMoveAction{BaseAction: structs.NewBaseAction(), X: structs.GetSpot("Merchants Tab").Coordinates.X, Y: structs.GetSpot("Merchants Tab").Coordinates.Y})
	root.AddSubAction(&structs.ClickAction{BaseAction: structs.NewBaseAction(), Button: "left"})
	root.AddSubAction(&structs.WaitAction{BaseAction: structs.NewBaseAction(), Time: 500})
	root.AddSubAction(&structs.MouseMoveAction{BaseAction: structs.NewBaseAction(), X: structs.GetSpot("Collector").Coordinates.X, Y: structs.GetSpot("Collector").Coordinates.Y})
	root.AddSubAction(&structs.ClickAction{BaseAction: structs.NewBaseAction(), Button: "left"})

	//image search for treasures
	imageSearch := &structs.ImageSearchAction{
		AdvancedAction: structs.AdvancedAction{
			BaseAction: structs.NewBaseAction(),
			Name:       "Search for treasures",
			SubActions: []structs.ActionInterface{},
		},
		SearchBox: *structs.GetSearchBox("Player Inventory Merchant"),
		Targets:   *structs.GetItemsMapCategory("treasures"),
	}
	// ocrSearch := &structs.OcrAction{
	// 	AdvancedAction: structs.AdvancedAction{
	// 		BaseAction: structs.NewBaseAction(),
	// 		Name:       "Search for treasures",
	// 		SubActions: []structs.ActionInterface{},
	// 	},
	// 	SearchBox: *structs.GetSearchBox("Player Inventory Merchant"),
	// 	Target:    "rare",
	// }
	root.AddSubAction(imageSearch)
	imageSearch.AddSubAction(&structs.MouseMoveAction{BaseAction: structs.NewBaseAction(), X: -1, Y: -1})
	root.AddSubAction(&structs.WaitAction{BaseAction: structs.NewBaseAction(), Time: 500})

	imageSearch.AddSubAction(&structs.ClickAction{BaseAction: structs.NewBaseAction(), Button: "left"})
	//root.AddSubAction(&structs.MouseMoveAction{BaseAction: structs.NewBaseAction(), X: structs.GetSpot("Make Deal").Coordinates.X, Y: structs.GetSpot("Make Deal").Coordinates.Y})

	updateTree(&tree, root)
	searchAreaSelector.SetSelected(searchAreaSelector.Options[0])

	content := container.NewHSplit(
		container.NewHSplit(
			createItemsCheckBoxes(),
			container.NewVSplit(
				container.NewVBox(
					&canvas.Text{Text: "ACTION SETTINGS", TextSize: 25, Alignment: fyne.TextAlignCenter, TextStyle: fyne.TextStyle{Bold: true, Monospace: true}},
					// **********************************************************************************************************Wait
					&widget.Label{Text: "Wait Action", TextStyle: fyne.TextStyle{Bold: true}, Alignment: fyne.TextAlignCenter},
					createWaitActionSettings(),
					widget.NewSeparator(),
					// ************************************************************************************************************Move
					&widget.Label{Text: "Mouse Move Action", TextStyle: fyne.TextStyle{Bold: true}, Alignment: fyne.TextAlignCenter},
					createMouseMoveSettings(),
					widget.NewSeparator(),
					// ************************************************************************************************************Click
					&widget.Label{Text: "Click Action", TextStyle: fyne.TextStyle{Bold: true}, Alignment: fyne.TextAlignCenter},
					createClickSettings(),
					widget.NewSeparator(),
					// *************************************************************************************************************Key
					&widget.Label{Text: "Key Action", TextStyle: fyne.TextStyle{Bold: true}, Alignment: fyne.TextAlignCenter},
					createKeySettings(),
					widget.NewSeparator(),
				),
				container.NewVBox(
					// ***************************************************************************************************************Search Settings
					&canvas.Text{Text: "ADVANCED ACTION SETTINGS", TextSize: 25, Alignment: fyne.TextAlignCenter, TextStyle: fyne.TextStyle{Bold: true, Monospace: true}},
					createAdvancedActionSettings(),
					// *************************************************************************************************************Loop
					&widget.Label{Text: "Loop", TextStyle: fyne.TextStyle{Bold: true}, Alignment: fyne.TextAlignCenter},
					createLoopActionSettings(),
					widget.NewSeparator(),
					// container.NewGridWithColumns(2,
					// 	searchAreaSelector,
					// 	layout.NewSpacer(),
					// ),
					// ******************************************************************************************************************Image Search
					&widget.Label{Text: "Image Search Action", TextStyle: fyne.TextStyle{Bold: true}, Alignment: fyne.TextAlignCenter},
					createImageSearchSettings(),
					widget.NewSeparator(),
					// *******************************************************************************************************************OCR
					&widget.Label{Text: "OCR Action", TextStyle: fyne.TextStyle{Bold: true}, Alignment: fyne.TextAlignCenter},
					createOCRSettings(),
					widget.NewSeparator(),
				),
			),
		),
		container.NewBorder(
			createMoveButtons(root, &tree),
			createMacroSettings(),
			nil,
			nil,
			&tree,
		),
	)
	return content
}

func ExecuteActionTree(root *structs.LoopAction) { //error
	var context interface{}
	root.Execute(context)
}

// ***************************************************************************************Start Macro
func createMacroSettings() *fyne.Container {
	macroSelector := widget.NewSelect([]string{"Collect Sell"}, func(s string) {})
	startMacroButton := &widget.Button{
		Text: "Start Macro",
		OnTapped: func() {
			ExecuteActionTree(root)
		},
		Icon:       theme.MediaPlayIcon(),
		Importance: widget.SuccessImportance,
	}
	return container.NewVBox(
		macroSelector,
		startMacroButton,
	)
}

// func ToggleWidgets(c *fyne.Container, b bool) {
// 	for _, obj := range c.Objects {
// 		switch obj := obj.(type) {
// 		case fyne.Disableable:
// 			if b {
// 				obj.Enable()
// 			} else {
// 				obj.Disable()
// 			}
// 		case *fyne.Container:
// 			ToggleWidgets(obj, b)
// 		}
// 	}
// }
