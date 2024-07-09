package gui

import (
	"Dark-And-Darker/structs"
	"Dark-And-Darker/utils"
	"fmt"
	"log"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/canvas"
	"fyne.io/fyne/v2/container"
	"fyne.io/fyne/v2/layout"
	"fyne.io/fyne/v2/theme"
	"fyne.io/fyne/v2/widget"
)

var (
	root               *ContainerNode
	tree               = widget.Tree{}
	selectedTreeItem   string
	selectedItemsMap   = make(map[string]bool)
	searchAreaSelector = &widget.Select{Options: *structs.GetSearchBoxMapKeys(*structs.GetSearchBoxMap())}
)

func LoadMainContent() *container.Split {
	root = newRootNode()
	updateTree(&tree, root)
	c1 := newContainerNode(root, 1, "Go to Collector")
	newActionNode(c1, &structs.MouseMoveAction{X: structs.GetSpot("Merchants Tab").Coordinates.X, Y: structs.GetSpot("Merchants Tab").Coordinates.Y})
	newActionNode(c1, &structs.WaitAction{Time: 100})
	newActionNode(c1, &structs.ClickAction{Button: "Left"})
	newActionNode(c1, &structs.WaitAction{Time: 100})
	newActionNode(c1, &structs.MouseMoveAction{X: structs.GetSpot("Merchant: Collector").Coordinates.X, Y: structs.GetSpot("Merchant: Collector").Coordinates.Y})
	newActionNode(c1, &structs.WaitAction{Time: 100})
	newActionNode(c1, &structs.ClickAction{Button: "Left"})
	newActionNode(c1, &structs.WaitAction{Time: 100})
	c2 := newContainerNode(root, 1, "Sell Collectibles")
	newActionNode(c2, &structs.ImageSearchAction{SearchBox: *structs.GetSearchBox("Whole Screen"), Targets: []string{"Healing Potion", "Protection Potion", "Bandage"}})
	//newActionNode(c2, &structs.MouseMoveAction{X: 300, Y: 300})
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
					createOCRSettings(),
					widget.NewSeparator(),
				),
			),
		),
		container.NewBorder(
			createMacroSettings(),
			createContainerSettings(),
			nil,
			nil,
			&tree,
		),
	)
	return content
}

func ExecuteActionTree(root *ContainerNode) error {
	context := &structs.Context{
		Variables: make(map[string]interface{}),
	}
	return executeNode(root, context)
}

func executeNode(node NodeInterface, context *structs.Context) error {
	if node == nil {
		return nil
	}
	switch n := node.(type) {
	case *ActionNode:
		{
			log.Printf("Executing action: %s", n.Action.String())
			err := n.Action.Execute(context)
			if err != nil {
				return fmt.Errorf("error executing action %s: %v", n.Action.String(), err)
			}
		}
	case *ContainerNode:
		{
			log.Printf("Entering container: %s x%d", n.Name, n.Iterations)
			for i := 1; i <= n.Iterations; i++ {
				log.Printf("container iteration: %d", i)
				for _, child := range node.(*ContainerNode).Children {
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

func createContainerSettings() *fyne.Container {
	containerName := widget.NewEntry()
	containerLoops := widget.NewSlider(1, 10)
	addContainerButton := &widget.Button{
		Text: utils.GetEmoji("Container") + "Add Container",
		OnTapped: func() {
			selectedNode := findNode(root, selectedTreeItem)
			if _, ok := selectedNode.(*ContainerNode); ok {
				if selectedNode != nil {
					newContainerNode(selectedNode.(*ContainerNode), int(containerLoops.Value), containerName.Text)
				}
			} else {
				if selectedNode != nil {
					newContainerNode(selectedNode.GetParent(), int(containerLoops.Value), containerName.Text)
				}
			}
			updateTree(&tree, root)
		},
		Icon:       theme.ContentAddIcon(),
		Importance: widget.SuccessImportance,
	}
	return container.NewVBox(
		container.NewGridWithColumns(3,
			container.NewGridWithColumns(2,
				widget.NewLabel("Name:"),
				containerName,
			),
			container.NewGridWithColumns(2,
				widget.NewLabel("Loops:"),
				containerLoops,
			),
			createMoveButtons(root, &tree),
		),
		addContainerButton,
	)
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
		Importance: widget.WarningImportance,
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
