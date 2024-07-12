package gui

import (
	"Dark-And-Darker/structs"
	"fmt"
	"log"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/canvas"
	"fyne.io/fyne/v2/container"
	"fyne.io/fyne/v2/layout"
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
	loopAction := &structs.LoopAction{
		ActionWithSubActions: structs.ActionWithSubActions{
			BaseAction: structs.BaseAction{
				UID:  "1",
				Name: "first action",
			},
		},
		Count: 5,
	}
	root.AddSubAction(loopAction, "loop 1")
	loopAction.AddSubAction(
		&structs.MouseMoveAction{
			X: structs.GetSpot("Merchants Tab").Coordinates.X,
			Y: structs.GetSpot("Merchants Tab").Coordinates.Y,
			BaseAction: structs.BaseAction{
				UID:  "1.1",
				Name: "second action, nested in first action",
			},
		},
		"second action",
	)

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
					//createKeySettings(),
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
					//createImageSearchSettings(),
					widget.NewSeparator(),
					// *******************************************************************************************************************OCR
					&widget.Label{Text: "OCR Action", TextStyle: fyne.TextStyle{Bold: true}, Alignment: fyne.TextAlignCenter},
					//createOCRSettings(),
					widget.NewSeparator(),
				),
			),
		),
		container.NewBorder(
			nil,
			nil,
			// createMacroSettings(),
			// createContainerSettings(),
			nil,
			nil,
			&tree,
		),
	)
	return content
}

func ExecuteActionTree(root *structs.LoopAction) error {
	var context interface{}
	// context := &structs.Context{
	// 	Variables: make(map[string]interface{}),
	// }
	return executeNode(root, context)
}

func executeNode(action structs.ActionInterface, context interface{}) error {
	if action == nil {
		return nil
	}
	tree.Select(action.GetUID())
	switch n := action.(type) {
	case *structs.BaseAction:
		{
			log.Printf("Executing action: %s", n.String())
			// if a, ok := n.Action.(*structs.ImageSearchAction); ok {
			// 	for _, b := range a.SubActions {
			// 		b.Execute()
			// 	}
			// }
			err := n.Execute(context)
			if err != nil {
				return fmt.Errorf("error executing action %s: %v", n.String(), err)
			}
		}
	case *structs.ActionWithSubActions:
		{
			log.Printf("Entering container: %s x%d", n.Name, len(n.GetSubActions()))
			for i := 1; i <= len(n.GetSubActions()); i++ {
				log.Printf("container iteration: %d", i)
				for _, child := range n.GetSubActions() {
					log.Printf("child looping")
					// if image search
					// if aNode, ok := child.(*ActionNode); ok {
					// 	log.Printf("is ActionNode")
					// 	if _, ok := aNode.Action.(*structs.ImageSearchAction); ok {
					// 		log.Printf("is ImageSearch")
					// 		log.Println(context)

					// 		err := aNode.Action.Execute(context)
					// 		if err != nil {
					// 			return err
					// 		}
					// 		log.Println(context)
					// 	}
					// 	if c, ok := context.Variables[utils.FoundItemsMapString].(map[string][]robotgo.Point); ok {
					// 		log.Printf("is []points")
					// 		//tempMap := make(map[string][]robotgo.Point)
					// 		var tempContext *structs.Context
					// 		tempContext.Variables[utils.ItemContext] = c[utils.ItemContext][0]
					// 		for key, items := range c { //loop items
					// 			for _, point := range items { //loop individual item coordinates

					// 				tempContext.Variables[utils.ItemContext] = point
					// 				//for y := 0; y < items; y++ { //loop individual item coordinates
					// 				for _, child1 := range n.Children[x+1:] { // loop til end of container
					// 					switch child1 := child1.(type) {
					// 					case *ActionNode:
					// 						{
					// 							child1.Action.Execute(tempContext)
					// 						}
					// 					case *ContainerNode:
					// 						{
					// 							break
					// 						}
					// 					}
					// 				}
					// 			}
					// 			log.Println(c)
					// 			delete(c, key)
					// 			log.Println(c)
					// 		}
					// 	}

					// 	break
					// }

					err := executeNode(child, context)
					if err != nil {
						return err
					}
				}
			}
		}
	}
	return nil
}

// func createContainerSettings() *fyne.Container {
// 	containerName := widget.NewEntry()
// 	containerLoops := widget.NewSlider(1, 10)
// 	addContainerButton := &widget.Button{
// 		Text: utils.GetEmoji("Container") + "Add Container",
// 		OnTapped: func() {
// 			selectedNode := findNode(root, selectedTreeItem)
// 			if _, ok := selectedNode.(*ContainerNode); ok {
// 				if selectedNode != nil {
// 					newAction(selectedNode.(*ContainerNode), int(containerLoops.Value), containerName.Text)
// 				}
// 			} else {
// 				if selectedNode != nil {
// 					newContainerNode(selectedNode.GetParent(), int(containerLoops.Value), containerName.Text)
// 				}
// 			}
// 			updateTree(&tree, root)
// 		},
// 		Icon:       theme.ContentAddIcon(),
// 		Importance: widget.SuccessImportance,
// 	}
// 	return container.NewVBox(
// 		container.NewGridWithColumns(3,
// 			container.NewGridWithColumns(2,
// 				widget.NewLabel("Name:"),
// 				containerName,
// 			),
// 			container.NewGridWithColumns(2,
// 				widget.NewLabel("Loops:"),
// 				containerLoops,
// 			),
// 			createMoveButtons(root, &tree),
// 		),
// 		addContainerButton,
// 	)
// }

// // ***************************************************************************************Start Macro
// func createMacroSettings() *fyne.Container {
// 	macroSelector := widget.NewSelect([]string{"Collect Sell"}, func(s string) {})
// 	startMacroButton := &widget.Button{
// 		Text: "Start Macro",
// 		OnTapped: func() {
// 			ExecuteActionTree(root)
// 		},
// 		Icon:       theme.MediaPlayIcon(),
// 		Importance: widget.WarningImportance,
// 	}
// 	return container.NewVBox(
// 		macroSelector,
// 		startMacroButton,
// 	)
// }

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
