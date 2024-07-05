// package main

// import (
// 	"Dark-And-Darker/gui"
// 	"log"

// 	"github.com/go-vgo/robotgo"
// 	"github.com/otiai10/gosseract/v2"
// )

// Can't seem to get the resolution of a single display
// 	- Can I just add / subtract the other displays from calculations to ensure proper cursor placement?
// 	- Create a select option in the GUI for this?

// func main() {

// 	log.Println("Screen Size")
// 	log.Println(robotgo.GetScreenSize())
// 	log.Println("Monitor 1 size")
// 	log.Println(robotgo.GetDisplayBounds(0))
// 	log.Println("Monitor 2 size")
// 	log.Println(robotgo.GetDisplayBounds(1))
// 	//gosseractOCR([4]int{0 + XAdditionalMonitorOffset,0 + YAdditionalMonitorOffset, 2560, 300})

// 	gui.Load()
// }

// func gosseractOCR(sb [4]int) {
// 	client := gosseract.NewClient()
// 	defer client.Close()
// 	//img := robotgo.ToByteImg(robotgo.CaptureImg(sb[0], sb[1], sb[2], sb[3]))
// 	//capture := robotgo.CaptureImg(sb[0], sb[1], sb[2], sb[3])
// 	capture := robotgo.CaptureImg([]int{sb[0], sb[1], sb[2], sb[3]}...)
// 	robotgo.SaveJpeg(capture, "./images/test1.jpeg")
// 	client.SetImage("./images/test1.jpeg")
// 	text, _ := client.Text()
// 	log.Println(text)
// 	return
// }

package main

import (
	"Dark-And-Darker/gui"
	"Dark-And-Darker/structs"
	"Dark-And-Darker/utils"
	"fmt"
	"image/color"
	"regexp"
	"strconv"
	"strings"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/app"
	"fyne.io/fyne/v2/canvas"
	"fyne.io/fyne/v2/container"
	"fyne.io/fyne/v2/layout"
	"fyne.io/fyne/v2/theme"
	"fyne.io/fyne/v2/widget"
)

var macro = *gui.NewMacro("Macro")
var selectedTreeItem string

//var savedSequences = make(map[string])

func main() {
	a := app.New()
	a.Settings().SetTheme(theme.DarkTheme())
	myWindow := a.NewWindow("Squire")
	root := createSampleTree()
	tree := widget.Tree{}
	updateTree(&tree, root)
	// ***************************************************************************************
	// **********************Buttons
	// ***************************************************************************************
	sequenceName := widget.NewEntry()
	sequenceLoops := widget.NewSlider(1, 10)
	addSequenceButton := &widget.Button{
		Text: utils.GetEmoji("Sequence") + "Add New Sequence",
		OnTapped: func() {
			seq := sequenceName.Text + " x" + strconv.FormatInt(int64(sequenceLoops.Value), 10)
			gui.NewSequence(root, seq)
			updateTree(&tree, root)
		},
		Icon:       theme.ContentAddIcon(),
		Importance: widget.SuccessImportance,
	}
	// ***************************************************************************************Wait
	millisecondsWaitEntry := widget.NewEntry()
	addWaitActionButton := &widget.Button{
		Text: utils.GetEmoji("Wait") + "Add Wait",
		OnTapped: func() {
			if selectedTreeItem == "" {
				return
			}
			selectedNode := findNode(root, selectedTreeItem)
			if selectedNode != nil && selectedNode.Type == gui.SequenceType {
				wait, _ := strconv.Atoi(millisecondsWaitEntry.Text)
				gui.NewAction(selectedNode, &structs.WaitAction{Time: wait})
				updateTree(&tree, root)
			}
		},
		IconPlacement: widget.ButtonIconPlacement(widget.ButtonAlignTrailing),
		Icon:          theme.NavigateNextIcon(),
		Importance:    widget.HighImportance,
	}
	// ***************************************************************************************Move
	structs.SpotMapInit()
	spotSelector := &widget.Select{Options: *structs.GetSpotMapKeys(*structs.GetSpotMap())}
	spotSelector.SetSelected(spotSelector.Options[0])
	mouseMoveXEntry := widget.NewEntry()
	mouseMoveYEntry := widget.NewEntry()
	addMouseMoveActionButton := &widget.Button{
		Text: utils.GetEmoji("Move") + "Add Move",
		OnTapped: func() {
			if selectedTreeItem == "" {
				return
			}
			selectedNode := findNode(root, selectedTreeItem)
			if selectedNode != nil && selectedNode.Type == gui.SequenceType {
				x, _ := strconv.Atoi(mouseMoveXEntry.Text)
				y, _ := strconv.Atoi(mouseMoveYEntry.Text)
				gui.NewAction(selectedNode, &structs.MouseMoveAction{X: x, Y: y})
				updateTree(&tree, root)
			}
		},
		IconPlacement: widget.ButtonIconPlacement(widget.ButtonAlignTrailing),
		Icon:          theme.NavigateNextIcon(),
		Importance:    widget.HighImportance,
	}
	// ***************************************************************************************Click
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
			if selectedNode != nil && selectedNode.Type == gui.SequenceType {

				gui.NewAction(selectedNode, &structs.ClickAction{Button: mouseButtonRadioGroup.Selected})
				updateTree(&tree, root)
			}
		},
		IconPlacement: widget.ButtonIconPlacement(widget.ButtonAlignTrailing),
		Icon:          theme.NavigateNextIcon(),
		Importance:    widget.HighImportance,
	}
	// ***************************************************************************************Key
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
			if selectedNode != nil && selectedNode.Type == gui.SequenceType {
				gui.NewAction(selectedNode, &structs.KeyAction{Key: "Enter", State: keyUpDownRadioGroup.Selected})
				updateTree(&tree, root)
			}
		},
		IconPlacement: widget.ButtonIconPlacement(widget.ButtonAlignTrailing),
		Icon:          theme.NavigateNextIcon(),
		Importance:    widget.HighImportance,
	}

	// ***************************************************************************************Search settings
	structs.SearchBoxMapInit()
	searchAreaSelector := &widget.Select{Options: *structs.GetSearchBoxMapKeys(*structs.GetSearchBoxMap())}
	searchAreaSelector.SetSelected(searchAreaSelector.Options[0])
	itemsCheckBoxes := gui.ItemsCheckBoxes()
	itemsCheckBoxes.MultiOpen = true
	// ***************************************************************************************Image Search

	addImageSearchActionButton := &widget.Button{
		Text: utils.GetEmoji("Image Search") + "Add Image Search",
		OnTapped: func() {
			if selectedTreeItem == "" {
				return
			}
			selectedNode := findNode(root, selectedTreeItem)
			if selectedNode != nil && selectedNode.Type == gui.SequenceType {
				gui.NewAction(selectedNode, &structs.ImageSearchAction{})
				updateTree(&tree, root)
			}
		},
		IconPlacement: widget.ButtonIconPlacement(widget.ButtonAlignTrailing),
		Icon:          theme.NavigateNextIcon(),
		Importance:    widget.HighImportance,
	}
	// ***************************************************************************************OCR
	addOCRActionButton := &widget.Button{
		Text: utils.GetEmoji("OCR") + "Add OCR",
		OnTapped: func() {
			if selectedTreeItem == "" {
				return
			}
			selectedNode := findNode(root, selectedTreeItem)
			if selectedNode != nil && selectedNode.Type == gui.SequenceType {
				gui.NewAction(selectedNode, &structs.OcrAction{})
				updateTree(&tree, root)
			}
		},
		IconPlacement: widget.ButtonIconPlacement(widget.ButtonAlignTrailing),
		Icon:          theme.NavigateNextIcon(),
		Importance:    widget.HighImportance,
	}
	// ***************************************************************************************Start Macro
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

	// ***************************************************************************************
	// *****************************Main Content
	// ***************************************************************************************
	content := container.NewGridWithColumns(2,
		container.NewHSplit(
			itemsCheckBoxes,
			container.NewVBox(
				// macroSettingsContainer,
				// ****************************************************************************************************************************************Wait
				&widget.Label{Text: "Wait Action", TextStyle: fyne.TextStyle{Bold: true}, Alignment: fyne.TextAlignCenter},
				container.NewGridWithColumns(4,
					layout.NewSpacer(),
					widget.NewLabel("Wait in ms"),
					millisecondsWaitEntry,
					addWaitActionButton,
				),
				canvas.NewRectangle(color.Gray{}),
				// ****************************************************************************************************************************************Move
				&widget.Label{Text: "Mouse Move Action", TextStyle: fyne.TextStyle{Bold: true}, Alignment: fyne.TextAlignCenter},
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
				canvas.NewRectangle(color.Gray{}),
				// ****************************************************************************************************************************************Click
				&widget.Label{Text: "Click Action", TextStyle: fyne.TextStyle{Bold: true}, Alignment: fyne.TextAlignCenter},
				container.NewHBox(
					layout.NewSpacer(),
					mouseButtonRadioGroup,
					addClickActionButton,
				),
				canvas.NewRectangle(color.Gray{}),
				// ****************************************************************************************************************************************Key
				&widget.Label{Text: "Key Action", TextStyle: fyne.TextStyle{Bold: true}, Alignment: fyne.TextAlignCenter},
				container.NewHBox(
					layout.NewSpacer(),
					widget.NewSelect([]string{"ctrl", "alt"}, func(s string) {}),
					keyUpDownRadioGroup,
					addKeyPressActionButton,
				),
				container.NewHBox(
					widget.NewLabel(""),

					canvas.NewRectangle(color.Gray{}),
				),

				// ****************************************************************************************************************************************Search Settings
				&widget.Label{Text: "Search Settings", TextStyle: fyne.TextStyle{Bold: true, Monospace: true}, Alignment: fyne.TextAlignCenter},
				searchAreaSelector,
				// ****************************************************************************************************************************************Image Search
				&widget.Label{Text: "Image Search Action", TextStyle: fyne.TextStyle{Bold: true}, Alignment: fyne.TextAlignCenter},
				container.NewHBox(
					addImageSearchActionButton,
				),
				canvas.NewRectangle(color.Gray{}),
				// ****************************************************************************************************************************************OCR
				&widget.Label{Text: "OCR Action", TextStyle: fyne.TextStyle{Bold: true}, Alignment: fyne.TextAlignCenter},
				container.NewHBox(
					addOCRActionButton,
				),
				canvas.NewRectangle(color.Gray{}),
			),
		),

		container.NewBorder(
			container.NewVBox(
				macroSelector,
				startMacroButton,
			),
			// ****************************************************************************************************************************************Sequence Settings
			container.NewVBox(
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
			),
			nil,
			nil,
			&tree,
		),
	)
	myWindow.SetContent(content)
	myWindow.ShowAndRun()
}

func createSampleTree() *gui.Node {
	seq1 := gui.NewSequence(&macro, "preset x2")
	gui.NewAction(seq1, &structs.ClickAction{Button: "Left"})
	gui.NewAction(seq1, &structs.MouseMoveAction{X: 100, Y: 100})

	seq2 := gui.NewSequence(&macro, "preset x1")
	gui.NewAction(seq2, &structs.ClickAction{Button: "Right"})
	gui.NewAction(seq2, &structs.MouseMoveAction{X: 2000, Y: 200})
	return &macro
}

func updateTree(tree *widget.Tree, root *gui.Node) {
	tree.Root = root.UID
	tree.ChildUIDs = func(uid string) []string {
		node := findNode(root, uid)
		if node == nil {
			return []string{}
		}
		childIDs := make([]string, len(node.Children))
		for i, child := range node.Children {
			childIDs[i] = child.UID
		}
		return childIDs
	}
	tree.IsBranch = func(uid string) bool {
		node := findNode(root, uid)
		return node != nil && (node.Type == gui.MacroType || node.Type == gui.SequenceType)
	}
	tree.CreateNode = func(branch bool) fyne.CanvasObject {
		return container.NewHBox(widget.NewLabel("Template"), layout.NewSpacer(), &widget.Button{Icon: theme.CancelIcon(), Importance: widget.DangerImportance})
	}
	tree.UpdateNode = func(uid string, branch bool, obj fyne.CanvasObject) {
		node := findNode(root, uid)
		if node == nil {
			return
		}
		container := obj.(*fyne.Container)
		label := container.Objects[0].(*widget.Label)
		removeButton := container.Objects[2].(*widget.Button)

		switch node.Type {
		case gui.MacroType:
			label.SetText(fmt.Sprintf("📁 %s", node.UID))
		case gui.SequenceType:
			label.SetText(fmt.Sprintf("%s %s %s", utils.GetEmoji("Sequence"), node.UID, node.Name))
		case gui.ActionType:
			label.SetText(node.Action.String())
		}

		if node.Parent != nil {
			removeButton.OnTapped = func() {
				node.Parent.RemoveChild(node)
				updateTree(tree, root)
			}
			removeButton.Show()
		} else {
			removeButton.Hide()
		}
	}
	tree.OnSelected = func(uid widget.TreeNodeID) {
		selectedTreeItem = uid
	}
	tree.Refresh()
}

func findNode(node *gui.Node, uid string) *gui.Node {
	if node.UID == uid {
		return node
	}
	for _, child := range node.Children {
		if found := findNode(child, uid); found != nil {
			return found
		}
	}
	return nil
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
