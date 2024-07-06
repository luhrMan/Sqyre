package gui

import (
	"Dark-And-Darker/structs"
	"Dark-And-Darker/utils"
	"fmt"
	"log"
	"regexp"
	"strconv"
	"strings"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/container"
	"fyne.io/fyne/v2/theme"
	"fyne.io/fyne/v2/widget"
)

var (
	root             *Node //= &Node{Name: "root", UID: "root", Action: &structs.ContainerAction{}, Parent: nil}
	tree             = widget.Tree{}
	selectedTreeItem string
	selectedItemsMap = make(map[string]bool)
)

func LoadMainContent() *fyne.Container {
	root = newRootNode("1")
	updateTree(&tree, root)
	newActionNode(root, &structs.MouseMoveAction{X: 100, Y: 100})
	loop := newActionNode(root, &structs.LoopAction{Iterations: 2}) //TRY newActionNode for this created action node?
	newActionNode(loop, &structs.MouseMoveAction{X: 200, Y: 200})
	newActionNode(loop, &structs.WaitAction{Time: 200})
	newActionNode(loop, &structs.MouseMoveAction{X: 300, Y: 300})
	nestedLoop := newActionNode(loop, &structs.LoopAction{Iterations: 3})
	newActionNode(nestedLoop, &structs.MouseMoveAction{X: 400, Y: 400})
	newActionNode(nestedLoop, &structs.WaitAction{Time: 200})
	newActionNode(nestedLoop, &structs.MouseMoveAction{X: 500, Y: 500})
	newActionNode(nestedLoop, &structs.WaitAction{Time: 200})
	newActionNode(nestedLoop, &structs.MouseMoveAction{X: 600, Y: 600})
	newActionNode(nestedLoop, &structs.WaitAction{Time: 200})
	c := newActionNode(root, &structs.ContainerAction{})
	newActionNode(c, &structs.MouseMoveAction{X: 600, Y: 600})
	newActionNode(c, &structs.WaitAction{Time: 200})
	updateTree(&tree, root)

	content := container.NewGridWithColumns(2,
		container.NewHSplit(
			createItemsCheckBoxes(),
			widget.NewLabel(""),
			// container.NewVBox(
			// 	// macroSettingsContainer,
			// 	// **********************************************************************************************************Wait
			// 	&widget.Label{Text: "Wait Action", TextStyle: fyne.TextStyle{Bold: true}, Alignment: fyne.TextAlignCenter},
			// 	createWaitActionSettings(),
			// 	canvas.NewRectangle(color.Gray{}),
			// 	// ************************************************************************************************************Move
			// 	&widget.Label{Text: "Mouse Move Action", TextStyle: fyne.TextStyle{Bold: true}, Alignment: fyne.TextAlignCenter},
			// 	createMouseMoveSettings(),
			// 	canvas.NewRectangle(color.Gray{}),
			// 	// ************************************************************************************************************Click
			// 	&widget.Label{Text: "Click Action", TextStyle: fyne.TextStyle{Bold: true}, Alignment: fyne.TextAlignCenter},
			// 	createClickSettings(),
			// 	canvas.NewRectangle(color.Gray{}),
			// 	// *************************************************************************************************************Key
			// 	&widget.Label{Text: "Key Action", TextStyle: fyne.TextStyle{Bold: true}, Alignment: fyne.TextAlignCenter},
			// 	createKeySettings(),
			// 	container.NewHBox(
			// 		widget.NewLabel(""),
			// 		canvas.NewRectangle(color.Gray{}),
			// 	),
			// 	container.NewHBox(
			// 		widget.NewLabel(""),
			// 		canvas.NewRectangle(color.Gray{}),
			// 	),
			// 	container.NewHBox(
			// 		widget.NewLabel(""),
			// 		canvas.NewRectangle(color.Gray{}),
			// 	),

			// 	// ***************************************************************************************************************Search Settings
			// 	&widget.Label{Text: "Search Settings", TextStyle: fyne.TextStyle{Bold: true, Monospace: true}, Alignment: fyne.TextAlignCenter},
			// 	createSearchAreaSelector(),
			// 	// ******************************************************************************************************************Image Search
			// 	&widget.Label{Text: "Image Search Action", TextStyle: fyne.TextStyle{Bold: true}, Alignment: fyne.TextAlignCenter},
			// 	createImageSearchSettings(),
			// 	canvas.NewRectangle(color.Gray{}),
			// 	// *******************************************************************************************************************OCR
			// 	&widget.Label{Text: "OCR Action", TextStyle: fyne.TextStyle{Bold: true}, Alignment: fyne.TextAlignCenter},
			// 	createOCRSettings(),
			// 	canvas.NewRectangle(color.Gray{}),
			// ),
		),
		container.NewBorder(
			// ***********************************************************************************************************************Sequence & Macro Settings
			createMacroSettings(),
			createSequenceSettings(),
			nil,
			nil,
			&tree,
		),
	)
	return content
}

func ExecuteActionTree(root *Node) error {
	context := &structs.Context{
		Variables: make(map[string]interface{}),
	}
	return executeNode(root, context)
}

func executeNode(node *Node, context *structs.Context) error {
	if node == nil {
		return nil
	}
	log.Printf("Executing action: %s", node.Action.String())

	err := node.Action.Execute(context)
	if err != nil {
		return fmt.Errorf("error executing action %s: %v", node.Action.String(), err)
	}

	var executeChildren = func() error {
		for _, child := range node.Children {
			err = executeNode(child, context)
			if err != nil {
				return err
			}
		}
		return nil
	}

	switch n := node.Action.(type) {
	case *structs.LoopAction:
		{
			for i := 0; i <= n.Iterations; i++ {
				executeChildren()
			}
		}
	default:
		executeChildren()
	}
	return nil
}

func createSequenceSettings() *fyne.Container {
	sequenceName := widget.NewEntry()
	sequenceLoops := widget.NewSlider(1, 10)
	addSequenceButton := &widget.Button{
		Text: utils.GetEmoji("Sequence") + "Add New Sequence",
		OnTapped: func() {
			//seq := sequenceName.Text + " x" + strconv.FormatInt(int64(sequenceLoops.Value), 10)
			//newSequence(root, seq)
			updateTree(&tree, root)
		},
		Icon:       theme.ContentAddIcon(),
		Importance: widget.SuccessImportance,
	}
	return container.NewVBox(
		container.NewGridWithColumns(2,
			container.NewGridWithColumns(2,
				widget.NewLabel("Name:"),
				sequenceName,
			),
			container.NewGridWithColumns(2,
				widget.NewLabel("Loops:"),
				sequenceLoops,
			),
		),
		addSequenceButton,
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

func getLoops(s string) int { //there is probably a better way to do this. maybe a sequence struct with a loop int, idk
	re := regexp.MustCompile(`x\d+$`)
	match := re.FindString(s)
	if match == "" {
		return 0
	}
	loops, _ := strconv.Atoi(strings.TrimPrefix(match, "x"))
	return loops
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

// func OffsetMove(x int, y int) {
// 	robotgo.Move(x+1920, y+utils.YOffset)
// 	robotgo.Sleep(1)
// }
