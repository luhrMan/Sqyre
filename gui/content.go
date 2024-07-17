package gui

import (
	"Dark-And-Darker/structs"
	"log"
	"os"
	"strings"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/canvas"
	"fyne.io/fyne/v2/container"
	"fyne.io/fyne/v2/theme"
	"fyne.io/fyne/v2/widget"
	"github.com/go-vgo/robotgo"
)

var (
	root               = structs.LoopAction{}
	tree               = widget.Tree{}
	selectedTreeItem   string
	selectedItemsMap   = make(map[string]bool)
	searchAreaSelector = &widget.Select{Options: *structs.GetSearchBoxMapKeys(*structs.GetSearchBoxMap())}
)

func initR() {
	// Register interface implementations
	// gob.Register(&structs.BaseAction{})
	// gob.Register(&structs.AdvancedAction{})
	// gob.Register(&structs.LoopAction{})
	// gob.Register(&structs.ImageSearchAction{})
	// gob.Register(&structs.OcrAction{})
	// gob.Register(&structs.WaitAction{})
	// gob.Register(&structs.ClickAction{})
	// gob.Register(&structs.MouseMoveAction{})
	// gob.Register(&structs.KeyAction{})
}

func LoadMainContent() *container.Split {
	log.Println("Screen Size")
	log.Println(robotgo.GetScreenSize())
	log.Println("Monitor 1 size")
	log.Println(robotgo.GetDisplayBounds(0))
	log.Println("Monitor 2 size")
	log.Println(robotgo.GetDisplayBounds(1))
	root = *newRootNode()
	updateTree(&tree, &root)
	//err := loadTreeFromFile("test.json")
	//log.Println(err)

	//click merchants tab, click merchant
	root.AddSubAction(&structs.MouseMoveAction{BaseAction: structs.NewBaseAction(), X: structs.GetSpot("Merchants Tab").X, Y: structs.GetSpot("Merchants Tab").Y})
	root.AddSubAction(&structs.ClickAction{BaseAction: structs.NewBaseAction(), Button: "left"})
	root.AddSubAction(&structs.WaitAction{BaseAction: structs.NewBaseAction(), Time: 200})
	root.AddSubAction(&structs.MouseMoveAction{BaseAction: structs.NewBaseAction(), X: structs.GetSpot("Collector").X, Y: structs.GetSpot("Collector").Y})
	root.AddSubAction(&structs.ClickAction{BaseAction: structs.NewBaseAction(), Button: "left"})
	root.AddSubAction(&structs.WaitAction{BaseAction: structs.NewBaseAction(), Time: 200})
	initR()
	//image search for treasures
	imageSearch := &structs.ImageSearchAction{
		AdvancedAction: structs.AdvancedAction{
			BaseAction: structs.NewBaseAction(),
			Name:       "Search for treasures",
			SubActions: []structs.ActionInterface{},
		},
		SearchBox: *structs.GetSearchBox("Stash Inventory"),
		Targets:   *structs.GetItemsMapCategory("treasures"),
	}
	ocrSearch := &structs.OcrAction{
		AdvancedAction: structs.AdvancedAction{
			BaseAction: structs.NewBaseAction(),
			Name:       "Search for Rare",
			SubActions: []structs.ActionInterface{},
		},
		SearchBox: *structs.GetSearchBox("Stash Inventory"),
		Target:    "Rare",
	}
	root.AddSubAction(imageSearch)
	imageSearch.AddSubAction(&structs.MouseMoveAction{BaseAction: structs.NewBaseAction(), X: -1, Y: -1})
	imageSearch.AddSubAction(&structs.WaitAction{BaseAction: structs.NewBaseAction(), Time: 200})
	imageSearch.AddSubAction(ocrSearch)
	ocrSearch.AddSubAction(&structs.KeyAction{BaseAction: structs.NewBaseAction(), Key: "shift", State: "down"})
	ocrSearch.AddSubAction(&structs.ClickAction{BaseAction: structs.NewBaseAction(), Button: "right"})
	ocrSearch.AddSubAction(&structs.KeyAction{BaseAction: structs.NewBaseAction(), Key: "shift", State: "up"})
	imageSearch.AddSubAction(&structs.WaitAction{BaseAction: structs.NewBaseAction(), Time: 200})
	root.AddSubAction(&structs.MouseMoveAction{BaseAction: structs.NewBaseAction(), X: structs.GetSpot("Make Deal").X, Y: structs.GetSpot("Make Deal").Y})

	encodeToGobFile(&root, "./saved-macros/Sell Collectibles.gob")
	//decodeFromFile("./saved-macros/Sell Collectibles.gob")

	//saveTreeToFile(root, "./saved-macros/Sell Collectibles.json")

	searchAreaSelector.SetSelected(searchAreaSelector.Options[0])
	content :=
		container.NewHSplit(
			container.NewHSplit(
				createItemsCheckBoxes(),
				container.NewVSplit(
					container.NewVBox(
						&canvas.Text{Text: "ACTION SETTINGS", TextSize: 25, Alignment: fyne.TextAlignCenter, TextStyle: fyne.TextStyle{Bold: true, Monospace: true}},
						// **********************************************************************************************************Wait
						&widget.Label{Text: "Wait Action", TextStyle: fyne.TextStyle{Bold: true}, Alignment: fyne.TextAlignCenter},
						createWaitSettings(),
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
						// ***************************************************************************************************************Advanced Actions
						&canvas.Text{Text: "ADVANCED ACTION SETTINGS", TextSize: 25, Alignment: fyne.TextAlignCenter, TextStyle: fyne.TextStyle{Bold: true, Monospace: true}},
						createAdvancedActionSettings(),
						// *************************************************************************************************************Loop
						&widget.Label{Text: "Loop", TextStyle: fyne.TextStyle{Bold: true}, Alignment: fyne.TextAlignCenter},
						createLoopSettings(),
						widget.NewSeparator(),
						// ******************************************************************************************************************Image Search
						&widget.Label{Text: "Image Search Action", TextStyle: fyne.TextStyle{Bold: true}, Alignment: fyne.TextAlignCenter},
						createImageSearchSettings(),
						widget.NewSeparator(),
						// *******************************************************************************************************************OCR
						&widget.Label{Text: "OCR Action", TextStyle: fyne.TextStyle{Bold: true}, Alignment: fyne.TextAlignCenter},
						createOCRSettings(),
						widget.NewSeparator(),
						// *******************************************************************************************************************Conditional
						// &widget.Label{Text: "Conditional Action", TextStyle: fyne.TextStyle{Bold: true}, Alignment: fyne.TextAlignCenter},
						// createConditionalSettings(),
						// widget.NewSeparator(),
					),
				),
			),
			container.NewBorder(
				container.NewHBox(
					widget.NewLabel("Global Delay"),
					widget.NewEntry(),
					createMoveButtons(&root, &tree),
				),
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

// ***************************************************************************************Macro
func createMacroSettings() *fyne.Container {
	return container.NewVBox(
		createSaveSettings(),
		macroSelector(),
		macroStartButton(),
	)
}

func macroSelector() *widget.Select {
	files, err := os.ReadDir("saved-macros")
	if err != nil {
		log.Fatal(err)
	}
	var macroList []string
	for _, f := range files {
		macroList = append(macroList, strings.TrimSuffix(f.Name(), ".json"))
	}
	return widget.NewSelect(macroList, func(s string) { loadTreeFromJsonFile(s + ".json") })
}

func macroStartButton() *widget.Button {
	return &widget.Button{
		Text: "Start Macro",
		OnTapped: func() {
			ExecuteActionTree(&root)
		},
		Icon:       theme.MediaPlayIcon(),
		Importance: widget.SuccessImportance,
	}
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
