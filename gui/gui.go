package gui

import (
	"Dark-And-Darker/utils"
	"image/color"
	"regexp"
	"strconv"
	"strings"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/canvas"
	"fyne.io/fyne/v2/container"
	"fyne.io/fyne/v2/theme"
	"fyne.io/fyne/v2/widget"
)

var macro = *NewMacro("Macro")
var root = createSampleTree()
var tree = widget.Tree{}
var selectedTreeItem string
var selectedItemsMap = make(map[string]bool)

func LoadMainContent() *fyne.Container {
	updateTree(&tree, root)
	content := container.NewGridWithColumns(2,
		container.NewHSplit(
			createItemsCheckBoxes(),
			container.NewVBox(
				// macroSettingsContainer,
				// **********************************************************************************************************Wait
				&widget.Label{Text: "Wait Action", TextStyle: fyne.TextStyle{Bold: true}, Alignment: fyne.TextAlignCenter},
				createWaitActionSettings(),
				canvas.NewRectangle(color.Gray{}),
				// ************************************************************************************************************Move
				&widget.Label{Text: "Mouse Move Action", TextStyle: fyne.TextStyle{Bold: true}, Alignment: fyne.TextAlignCenter},
				createMouseMoveSettings(),
				canvas.NewRectangle(color.Gray{}),
				// ************************************************************************************************************Click
				&widget.Label{Text: "Click Action", TextStyle: fyne.TextStyle{Bold: true}, Alignment: fyne.TextAlignCenter},
				createClickSettings(),
				canvas.NewRectangle(color.Gray{}),
				// *************************************************************************************************************Key
				&widget.Label{Text: "Key Action", TextStyle: fyne.TextStyle{Bold: true}, Alignment: fyne.TextAlignCenter},
				createKeySettings(),
				container.NewHBox(
					widget.NewLabel(""),

					canvas.NewRectangle(color.Gray{}),
				),
				container.NewHBox(
					widget.NewLabel(""),

					canvas.NewRectangle(color.Gray{}),
				),
				container.NewHBox(
					widget.NewLabel(""),

					canvas.NewRectangle(color.Gray{}),
				),

				// ***************************************************************************************************************Search Settings
				&widget.Label{Text: "Search Settings", TextStyle: fyne.TextStyle{Bold: true, Monospace: true}, Alignment: fyne.TextAlignCenter},
				createSearchAreaSelector(),
				// ******************************************************************************************************************Image Search
				&widget.Label{Text: "Image Search Action", TextStyle: fyne.TextStyle{Bold: true}, Alignment: fyne.TextAlignCenter},
				createImageSearchSettings(),
				canvas.NewRectangle(color.Gray{}),
				// *******************************************************************************************************************OCR
				&widget.Label{Text: "OCR Action", TextStyle: fyne.TextStyle{Bold: true}, Alignment: fyne.TextAlignCenter},
				createOCRSettings(),
				canvas.NewRectangle(color.Gray{}),
			),
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
func createSequenceSettings() *fyne.Container {
	sequenceName := widget.NewEntry()
	sequenceLoops := widget.NewSlider(1, 10)
	addSequenceButton := &widget.Button{
		Text: utils.GetEmoji("Sequence") + "Add New Sequence",
		OnTapped: func() {
			seq := sequenceName.Text + " x" + strconv.FormatInt(int64(sequenceLoops.Value), 10)
			NewSequence(root, seq)
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
			for _, sequence := range macro.Children {
				for range getLoops(sequence.Name) {
					for _, action := range sequence.Children {
						action.Action.Execute()
					}
				}
			}
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
