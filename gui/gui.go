package gui

import (
	"Dark-And-Darker/structs"
	"Dark-And-Darker/utils"
	"fmt"
	"log"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/container"
	"fyne.io/fyne/v2/theme"
	"fyne.io/fyne/v2/widget"
)

var (
	root             *ContainerNode
	tree             = widget.Tree{}
	selectedTreeItem string
	selectedItemsMap = make(map[string]bool)
)

func LoadMainContent() *container.Split {
	root = newRootNode()
	log.Println(root)
	updateTree(&tree, root)
	newActionNode(root, &structs.MouseMoveAction{X: 100, Y: 100})
	loop := newContainerNode(root, 1, "Loop Preset")
	newActionNode(loop, &structs.MouseMoveAction{X: 200, Y: 200})
	newActionNode(loop, &structs.WaitAction{Time: 200})
	newActionNode(loop, &structs.MouseMoveAction{X: 300, Y: 300})
	nested := newContainerNode(loop, 2, "Loop Preset 2")
	newActionNode(nested, &structs.MouseMoveAction{X: 400, Y: 400})
	newActionNode(nested, &structs.WaitAction{Time: 200})
	newActionNode(nested, &structs.MouseMoveAction{X: 500, Y: 500})
	newActionNode(nested, &structs.WaitAction{Time: 200})
	newActionNode(nested, &structs.MouseMoveAction{X: 600, Y: 600})
	newActionNode(nested, &structs.WaitAction{Time: 200})
	c := newContainerNode(root, 1, "Container Preset 1")
	newActionNode(c, &structs.MouseMoveAction{X: 600, Y: 600})
	newActionNode(c, &structs.WaitAction{Time: 200})
	updateTree(&tree, root)

	content := container.NewHSplit(
		container.NewHSplit(
			createItemsCheckBoxes(),
			container.NewVSplit(
				container.NewVBox(
					&widget.Label{Text: "ACITON SETTINGS", TextStyle: fyne.TextStyle{Bold: true, Monospace: true}, Alignment: fyne.TextAlignCenter},
					// 	// macroSettingsContainer,
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
					&widget.Label{Text: "SEARCH SETTINGS", TextStyle: fyne.TextStyle{Bold: true, Monospace: true}, Alignment: fyne.TextAlignCenter},
					createSearchAreaSelector(),
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
			// ***********************************************************************************************************************Sequence & Macro Settings
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
			log.Printf("Executing action: %s", node.(*ActionNode).Action.String())
			err := node.(*ActionNode).Action.Execute(context)
			if err != nil {
				return fmt.Errorf("error executing action %s: %v", node.(*ActionNode).Action.String(), err)
			}
		}
	case *ContainerNode:
		{
			for i := 0; i <= n.Iterations; i++ {
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
		Text: utils.GetEmoji("Container") + "Add New Container",
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

// func getLoops(s string) int { //there is probably a better way to do this. maybe a sequence struct with a loop int, idk
// 	re := regexp.MustCompile(`x\d+$`)
// 	match := re.FindString(s)
// 	if match == "" {
// 		return 0
// 	}
// 	loops, _ := strconv.Atoi(strings.TrimPrefix(match, "x"))
// 	return loops
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

// func OffsetMove(x int, y int) {
// 	robotgo.Move(x+1920, y+utils.YOffset)
// 	robotgo.Sleep(1)
// }
