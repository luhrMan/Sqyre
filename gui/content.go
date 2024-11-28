package gui

import (
	"Dark-And-Darker/custom_widgets"

	"Dark-And-Darker/structs"
	"log"
	"os"
	"strings"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/canvas"
	"fyne.io/fyne/v2/container"
	"fyne.io/fyne/v2/layout"
	"fyne.io/fyne/v2/theme"
	"fyne.io/fyne/v2/widget"
	"github.com/go-vgo/robotgo"
)

var (
	root *structs.LoopAction
	//root               = structs.LoopAction{}
	tree               = widget.Tree{}
	selectedTreeItem   = ".1"
	selectedItemsMap   = make(map[string]any)
	searchAreaSelector = &widget.Select{Options: *structs.GetSearchBoxMapKeys(*structs.GetSearchBoxMap())}
	customImport       = custom_widgets.NewToggle(func(b bool) {})
)

func LoadMainContent() *fyne.Container {
	log.Println("Screen Size")
	log.Println(robotgo.GetScreenSize())
	log.Println("Monitor 1 size")
	log.Println(robotgo.GetDisplayBounds(0))
	log.Println("Monitor 2 size")
	log.Println(robotgo.GetDisplayBounds(1))
	root = getRoot()
	updateTree(&tree, root)
	//err := loadTreeFromFile("test.json")
	//log.Println(err)

	//click merchants tab, click merchant
	root.AddSubAction(&structs.MouseMoveAction{BaseAction: structs.NewBaseAction(), X: structs.GetSpot("Merchants Tab").X, Y: structs.GetSpot("Merchants Tab").Y})
	root.AddSubAction(&structs.ClickAction{BaseAction: structs.NewBaseAction(), Button: "left"})
	root.AddSubAction(&structs.WaitAction{BaseAction: structs.NewBaseAction(), Time: 200})
	root.AddSubAction(&structs.MouseMoveAction{BaseAction: structs.NewBaseAction(), X: structs.GetSpot("Collector").X, Y: structs.GetSpot("Collector").Y})
	root.AddSubAction(&structs.ClickAction{BaseAction: structs.NewBaseAction(), Button: "left"})
	root.AddSubAction(&structs.WaitAction{BaseAction: structs.NewBaseAction(), Time: 200})
	//initR()
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

	//encodeToGobFile(root, "./saved-macros/Sell Collectibles.gob")
	//decodeFromFile("./saved-macros/Sell Collectibles.gob")

	//saveTreeToFile(root, "./saved-macros/Sell Collectibles.json")

	// searchAreaSelector.SetSelected(searchAreaSelector.Options[0])
	mainLayout := container.NewBorder(createToolbar(), nil, nil, nil)
	settingsLayout := container.NewBorder(nil, createUpdateButton(), createItemsCheckBoxes(), nil)

	settingsLayout.Add(container.NewVBox(
		&widget.Label{Text: "Wait Action (ms)", TextStyle: fyne.TextStyle{Bold: true}, Alignment: fyne.TextAlignCenter},
		container.NewGridWithColumns(2,
			container.NewHBox(layout.NewSpacer(), boundTimeLabel, widget.NewLabel("ms")),
			boundTimeSlider,
		),
		widget.NewSeparator(),
		&widget.Label{Text: "Mouse Move Action", TextStyle: fyne.TextStyle{Bold: true}, Alignment: fyne.TextAlignCenter},
		container.NewGridWithColumns(2,
			container.NewHBox(layout.NewSpacer(), widget.NewLabel("X:"), boundMoveXLabel),
			boundMoveXSlider,
			container.NewHBox(layout.NewSpacer(), widget.NewLabel("Y:"), boundMoveYLabel),
			boundMoveYSlider,
		),
		widget.NewSeparator(),
		&widget.Label{Text: "Click Action", TextStyle: fyne.TextStyle{Bold: true}, Alignment: fyne.TextAlignCenter},
		container.NewHBox(layout.NewSpacer(), widget.NewLabel("left"), boundButtonToggle, widget.NewLabel("right"), layout.NewSpacer()),
		widget.NewSeparator(),
		&widget.Label{Text: "Key Action", TextStyle: fyne.TextStyle{Bold: true}, Alignment: fyne.TextAlignCenter},
		container.NewHBox(layout.NewSpacer(), boundKeySelect, widget.NewLabel("down"), boundStateToggle, widget.NewLabel("up"), layout.NewSpacer()),
		widget.NewSeparator(),

		&canvas.Text{Text: "ADVANCED ACTION SETTINGS", TextSize: 20, Alignment: fyne.TextAlignCenter, TextStyle: fyne.TextStyle{Bold: true, Monospace: true}},
		container.NewGridWithColumns(3,
			layout.NewSpacer(),
			container.NewGridWithColumns(2,
				widget.NewLabel("Name:"),
				boundAdvancedActionNameEntry,
			),
			layout.NewSpacer(),
			layout.NewSpacer(),
			container.NewGridWithColumns(2,
				widget.NewLabel("Search area:"),
				boundSearchAreaSelect,
			),
			layout.NewSpacer(),
		),
		&widget.Label{Text: "Loop", TextStyle: fyne.TextStyle{Bold: true}, Alignment: fyne.TextAlignCenter},
		container.NewGridWithColumns(2,
			container.NewHBox(layout.NewSpacer(), widget.NewLabel("loops:"), boundCountLabel),
			boundCountSlider,
		),
		&widget.Label{Text: "Image Search Action", TextStyle: fyne.TextStyle{Bold: true}, Alignment: fyne.TextAlignCenter},
		&widget.Label{Text: "Conditional Action", TextStyle: fyne.TextStyle{Bold: true}, Alignment: fyne.TextAlignCenter},
	))
	macroLayout := container.NewBorder(
		container.NewHBox(
			widget.NewLabel("Global Delay"),
			widget.NewEntry(),
			createMoveButtons(root, &tree),
		),
		createMacroSettings(),
		nil,
		nil,
		&tree,
	)
	middleSplit := container.NewHSplit(settingsLayout, macroLayout)
	mainLayout.Add(middleSplit)
	return mainLayout
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
	return widget.NewSelect(macroList, func(s string) { loadTreeFromJsonFile(root, s+".json") })
}

func macroStartButton() *widget.Button {
	return &widget.Button{
		Text: "Start Macro",
		OnTapped: func() {
			ExecuteActionTree(root)
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

func createToolbar() *widget.Toolbar {

	toolbar := widget.NewToolbar(
		widget.NewToolbarSpacer(),
		widget.NewToolbarSpacer(),
		widget.NewToolbarSeparator(),
		widget.NewToolbarAction(theme.HistoryIcon(), func() {
			addActionToTree(&structs.WaitAction{})
		}),
		widget.NewToolbarSeparator(),
		widget.NewToolbarAction(theme.ContentRedoIcon(), func() {
			addActionToTree(&structs.MouseMoveAction{})
		}),
		widget.NewToolbarSeparator(),
		widget.NewToolbarAction(theme.DownloadIcon(), func() {
			addActionToTree(&structs.ClickAction{})
		}),
		widget.NewToolbarSeparator(),
		widget.NewToolbarAction(theme.ComputerIcon(), func() {
			addActionToTree(&structs.KeyAction{})
		}),
		widget.NewToolbarSeparator(),
		widget.NewToolbarSpacer(),
		widget.NewToolbarSeparator(),
		widget.NewToolbarAction(theme.MediaReplayIcon(), func() {
			addActionToTree(&structs.LoopAction{})
		}),
		widget.NewToolbarSeparator(),
		widget.NewToolbarAction(theme.MediaPhotoIcon(), func() {
			addActionToTree(&structs.ImageSearchAction{})
		}),
		widget.NewToolbarSeparator(),
		widget.NewToolbarAction(theme.DocumentIcon(), func() {
			addActionToTree(&structs.OcrAction{})
		}),
		widget.NewToolbarSeparator(),
		widget.NewToolbarSpacer(),
		widget.NewToolbarSpacer(),
		widget.NewToolbarSpacer(),
	)
	return toolbar
}

func addActionToTree(actionType structs.ActionInterface) {
	var (
		selectedNode = findNode(root, selectedTreeItem)
		action       structs.ActionInterface
	)
	switch actionType.(type) {
	case *structs.WaitAction:
		t, _ := boundTime.Get()
		action = &structs.WaitAction{Time: int(t), BaseAction: structs.NewBaseAction()}
	case *structs.MouseMoveAction:
		x, _ := boundMoveX.Get()
		y, _ := boundMoveY.Get()
		action = &structs.MouseMoveAction{X: int(x), Y: int(y), BaseAction: structs.NewBaseAction()}
	case *structs.ClickAction:
		str := ""
		b, _ := boundButton.Get()
		if !b {
			str = "left"
		} else {
			str = "right"
		}
		action = &structs.ClickAction{Button: str, BaseAction: structs.NewBaseAction()}
	case *structs.KeyAction:
		str := ""
		k, _ := boundKey.Get()
		s, _ := boundState.Get()
		if !s {
			str = "down"
		} else {
			str = "up"
		}
		action = &structs.KeyAction{Key: k, State: str, BaseAction: structs.NewBaseAction()}
	case *structs.LoopAction:
		n, _ := boundAdvancedActionName.Get()
		c, _ := boundCount.Get()
		action = &structs.LoopAction{
			Count: int(c),
			AdvancedAction: structs.AdvancedAction{
				BaseAction: structs.NewBaseAction(),
				Name:       n,
			},
		}
	case *structs.ImageSearchAction:
		n, _ := boundAdvancedActionName.Get()
		t, _ := boundTargets.Get()
		s, _ := boundSearchArea.Get()
		action = &structs.ImageSearchAction{
			Targets:   t,
			SearchBox: *structs.GetSearchBox(s),
			AdvancedAction: structs.AdvancedAction{
				BaseAction: structs.NewBaseAction(),
				Name:       n,
			},
		}
	case *structs.OcrAction:
		// n, _ := boundAdvancedActionName.Get()
		// t, _ := boundOcrTarget.Get()
		// s, _ := boundSearchArea.Get()
		// action = &structs.OcrAction{
		// 	SearchBox: *structs.GetSearchBox(s),
		// 	Target:    t,
		// 	AdvancedAction: structs.AdvancedAction{
		// 		BaseAction: structs.NewBaseAction(),
		// 		Name:       n,
		// 	},
		// }

		// if selectedNode == nil {
		// 	selectedNode = getRoot()
		// }
		if s, ok := selectedNode.(structs.AdvancedActionInterface); ok {
			s.AddSubAction(selectedNode)
		} else {
			selectedNode.GetParent().AddSubAction(action)
		}
		updateTree(&tree, root)
	}
}
