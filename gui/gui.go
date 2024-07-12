package gui

import (
	"Dark-And-Darker/structs"
	"log"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/canvas"
	"fyne.io/fyne/v2/container"
	"fyne.io/fyne/v2/layout"
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
	root.AddSubAction(&structs.MouseMoveAction{
		X: structs.GetSpot("Merchants Tab").Coordinates.X,
		Y: structs.GetSpot("Merchants Tab").Coordinates.Y,
		BaseAction: structs.BaseAction{
			UID: "1.1",
		},
	}, "Go to Merchants Tab")
	root.AddSubAction(&structs.ClickAction{
		Button: "left",
		BaseAction: structs.BaseAction{
			UID: "1.2",
		},
	}, "Click")
	root.AddSubAction(&structs.WaitAction{
		Time: 500,
		BaseAction: structs.BaseAction{
			UID: "1.3",
		},
	}, "Click")
	root.AddSubAction(&structs.MouseMoveAction{
		X: structs.GetSpot("Merchants: Collector").Coordinates.X,
		Y: structs.GetSpot("Merchants: Collector").Coordinates.Y,
		BaseAction: structs.BaseAction{
			UID: "1.4",
		},
	}, "Go to Collector")
	root.AddSubAction(&structs.ClickAction{
		Button: "left",
		BaseAction: structs.BaseAction{
			UID: "1.5",
		},
	}, "Click")
	root.AddSubAction(&structs.ImageSearchAction{
		ActionWithSubActions: structs.ActionWithSubActions{
			BaseAction: structs.BaseAction{
				UID: "1.6",
			},
			SubActions: []structs.ActionInterface{
				&structs.MouseMoveAction{
					BaseAction: structs.BaseAction{
						UID: "1.6.1",
					},
				},
			},
		},
		SearchBox: *structs.GetSearchBox("Whole Screen"),
		Targets:   *structs.GetItemsMapCategory("treasures"),
	}, "Search for collectibles")

	// c1 := newAction(root, &structs.LoopAction{}, "Go to Collector")
	// newAction(c1, &structs.WaitAction{Time: 100}, "name")
	// newAction(c1, &structs.ClickAction{Button: "Left"}, "name")
	// newAction(c1, &structs.WaitAction{Time: 100}, "name")
	// newAction(c1, &structs.MouseMoveAction{X: structs.GetSpot("Merchant: Collector").Coordinates.X, Y: structs.GetSpot("Merchant: Collector").Coordinates.Y})
	// newAction(c1, &structs.WaitAction{Time: 100})
	// newAction(c1, &structs.ClickAction{Button: "Left"})
	// newAction(c1, &structs.WaitAction{Time: 100})
	// c2 := newContainerNode(root, 1, "Sell Collectibles")

	// newActionNode(c2, &structs.KeyAction{Key: "shift", State: "Down"})
	// newActionNode(c2, &structs.ImageSearchAction{
	// 	SearchBox: *structs.GetSearchBox("Whole Screen"),
	// 	Targets:   []string{"Healing Potion", "Protection Potion", "Bandage"},
	// 	SubActions: []structs.Action{
	// 		&structs.MouseMoveAction{},
	// 		&structs.WaitAction{Time: 500},
	// 		&structs.ClickAction{Button: "right"}}})
	// //newActionNode(c2, &structs.MouseMoveAction{X: 300, Y: 300})
	// newActionNode(c2, &structs.KeyAction{Key: "shift", State: "Up"})
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
					// *************************************************************************************************************Loop
					&widget.Label{Text: "Loop", TextStyle: fyne.TextStyle{Bold: true}, Alignment: fyne.TextAlignCenter},
					createLoopActionSettings(),
					widget.NewSeparator(),
				),
				container.NewVBox(
					// ***************************************************************************************************************Search Settings
					&canvas.Text{Text: "SEARCH SETTINGS", TextSize: 25, Alignment: fyne.TextAlignCenter, TextStyle: fyne.TextStyle{Bold: true, Monospace: true}},
					container.NewGridWithColumns(2,
						searchAreaSelector,
						layout.NewSpacer(),
					),
					// ******************************************************************************************************************Image Search
					&widget.Label{Text: "Image Search Action", TextStyle: fyne.TextStyle{Bold: true}, Alignment: fyne.TextAlignCenter},
					createImageSearchSettings(),
					widget.NewSeparator(),
					// *******************************************************************************************************************OCR
					&widget.Label{Text: "OCR Action", TextStyle: fyne.TextStyle{Bold: true}, Alignment: fyne.TextAlignCenter},
					//createOCRSettings(),
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
	//return executeNode(root, context)
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
