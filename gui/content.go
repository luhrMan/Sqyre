package gui

import (
	"Dark-And-Darker/custom_widgets"
	"Dark-And-Darker/utils"
	"fmt"
	"fyne.io/fyne/v2/data/binding"
	"fyne.io/fyne/v2/layout"

	"Dark-And-Darker/structs"
	"log"
	"os"
	"strings"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/container"
	"fyne.io/fyne/v2/theme"
	"fyne.io/fyne/v2/widget"
	"github.com/go-vgo/robotgo"
)

var (
	macro               = &macroTree{}
	macroName           string
	boundMacroName      = binding.BindString(&macroName)
	boundMacroNameEntry = widget.NewEntryWithData(boundMacroName)

	selectedTreeItem   = ".1"
	selectedItemsMap   = make(map[string]any)
	searchAreaSelector = &widget.Select{Options: *structs.GetSearchBoxMapKeys(*structs.GetSearchBoxMap())}
	customImport       = custom_widgets.NewToggle(func(b bool) {})
)

//action settings
var (
	//	settingsAccordion = widget.NewAccordion()
	settingsTabs = container.AppTabs{}
	//BASICS
	//wait
	time            float64
	boundTime       = binding.BindFloat(&time)
	boundTimeSlider = widget.NewSliderWithData(0.0, 250.0, boundTime)
	boundTimeLabel  = widget.NewLabelWithData(binding.FloatToStringWithFormat(boundTime, "%0.0f"))

	//move
	moveX float64
	moveY float64
	// spot             structs.Spot
	// boundSpot        = binding.BindString(&spot.Name)
	boundMoveX = binding.BindFloat(&moveX)
	boundMoveY = binding.BindFloat(&moveY)
	// boundSpotSelect  = widget.NewSelect(*structs.GetSpotMapKeys(*structs.GetSpotMap()), func(s string) { boundSpot.Set(s) })
	boundMoveXSlider = widget.NewSliderWithData(0.0, float64(utils.MonitorWidth), boundMoveX)
	boundMoveYSlider = widget.NewSliderWithData(0.0, float64(utils.MonitorHeight), boundMoveY)
	boundMoveXLabel  = widget.NewLabelWithData(binding.FloatToStringWithFormat(boundMoveX, "%0.0f"))
	boundMoveYLabel  = widget.NewLabelWithData(binding.FloatToStringWithFormat(boundMoveY, "%0.0f"))
	boundMoveXEntry  = widget.NewEntryWithData(binding.FloatToStringWithFormat(boundMoveX, "%0.0f"))
	boundMoveYEntry  = widget.NewEntryWithData(binding.FloatToStringWithFormat(boundMoveY, "%0.0f"))

	//click
	button            bool
	boundButton       = binding.BindBool(&button)
	boundButtonToggle = custom_widgets.NewToggleWithData(boundButton)

	//key
	key              string
	state            bool
	boundKey         = binding.BindString(&key)
	boundState       = binding.BindBool(&state)
	boundKeySelect   = widget.NewSelect([]string{"ctrl", "alt", "shift"}, func(s string) { boundKey.Set(s) })
	boundStateToggle = custom_widgets.NewToggleWithData(boundState)

	//ADVANCED

	//loop
	loopName           string
	count              float64 = 1
	boundLoopName              = binding.BindString(&loopName)
	boundCount                 = binding.BindFloat(&count)
	boundLoopNameEntry         = widget.NewEntryWithData(boundLoopName)
	boundCountSlider           = widget.NewSliderWithData(1, 10, boundCount)
	boundCountLabel            = widget.NewLabelWithData(binding.FloatToStringWithFormat(boundCount, "%0.0f"))
	//image search
	imageSearchName           string
	searchArea                string
	targets                   []string
	boundImageSearchName      = binding.BindString(&imageSearchName)
	boundSearchArea           = binding.BindString(&searchArea)
	boundTargets              = binding.BindStringList(&targets)
	boundImageSearchNameEntry = widget.NewEntryWithData(boundImageSearchName)
	boundSearchAreaSelect     = widget.NewSelect(*structs.GetSearchBoxMapKeys(*structs.GetSearchBoxMap()), func(s string) { boundSearchArea.Set(s) })
	boundSelectedItemsMap     = binding.BindUntypedMap(&selectedItemsMap)
	//ocr
)

//action settings layout
var (
	waitSettings = container.NewVBox(
		container.NewGridWithColumns(2, container.NewHBox(layout.NewSpacer(), boundTimeLabel, widget.NewLabel("ms")), boundTimeSlider),
	)
	moveSettings = container.NewVBox(container.NewGridWithColumns(2,
		container.NewHBox(layout.NewSpacer(), widget.NewLabel("X:"), boundMoveXEntry, boundMoveXLabel), boundMoveXSlider,
		container.NewHBox(layout.NewSpacer(), widget.NewLabel("Y:"), boundMoveYEntry, boundMoveYLabel), boundMoveYSlider),
	)
	clickSettings = container.NewVBox(
		container.NewHBox(layout.NewSpacer(), widget.NewLabel("left"), boundButtonToggle, widget.NewLabel("right"), layout.NewSpacer()),
	)
	keySettings = container.NewVBox(
		container.NewHBox(layout.NewSpacer(), boundKeySelect, widget.NewLabel("down"), boundStateToggle, widget.NewLabel("up"), layout.NewSpacer()))
	loopSettings = container.NewVBox(
		container.NewGridWithColumns(2, container.NewHBox(layout.NewSpacer(), widget.NewLabel("name:")), boundLoopNameEntry),
		container.NewGridWithColumns(2, container.NewHBox(layout.NewSpacer(), widget.NewLabel("loops:"), boundCountLabel), boundCountSlider),
	)
	imageSearchSettings = container.NewVBox(
		container.NewGridWithColumns(2, container.NewHBox(layout.NewSpacer(), widget.NewLabel("name:")), boundImageSearchNameEntry),
		container.NewHBox(layout.NewSpacer(), widget.NewLabel("search area:"), boundSearchAreaSelect),
	)
	ocrSettings = container.NewHBox(layout.NewSpacer(), layout.NewSpacer())
)

func LoadMainContent() *fyne.Container {
	log.Println("Screen Size")
	log.Println(robotgo.GetScreenSize())
	log.Println("Monitor 1 size")
	log.Println(robotgo.GetDisplayBounds(0))
	log.Println("Monitor 2 size")
	log.Println(robotgo.GetDisplayBounds(1))
	macro.createTree()
	// searchAreaSelector.SetSelected(searchAreaSelector.Options[0])
	mainLayout := container.NewBorder(nil, nil, nil, nil)
	settingsLayout := container.NewBorder(&settingsTabs, macro.createUpdateButton(), nil, nil)

	settingsTabs.Append(container.NewTabItem("Wait", waitSettings))
	settingsTabs.Append(container.NewTabItem("Move", moveSettings))
	settingsTabs.Append(container.NewTabItem("Click", clickSettings))
	settingsTabs.Append(container.NewTabItem("Key", keySettings))
	settingsTabs.Append(container.NewTabItem("Loop", loopSettings))
	settingsTabs.Append(container.NewTabItem("Image Search", imageSearchSettings))
	settingsTabs.Append(container.NewTabItem("OCR", ocrSettings))

	macroLayout := container.NewBorder(
		container.NewGridWithColumns(3,
			container.NewHBox(
				widget.NewLabel("Global Delay:"),
				widget.NewEntry(),
				layout.NewSpacer(),
				widget.NewLabel("Macro Name:"),
			),
			boundMacroNameEntry,
			macro.createMacroToolbar(),
		),
		nil,
		nil,
		nil,
		macro.tree,
	)
	middleSplit := container.NewHSplit(settingsLayout, macroLayout)
	mainLayout.Add(middleSplit)
	macro.loadTreeFromJsonFile("Currency Testing.json")
	return mainLayout
}

func (m *macroTree) ExecuteActionTree() { //error
	var context interface{}
	err := m.root.Execute(context)
	if err != nil {
		log.Println(err)
		return
	}
}

func (m *macroTree) createMacroToolbar() *widget.Toolbar {
	tb := widget.NewToolbar(
		widget.NewToolbarAction(theme.DocumentSaveIcon(), func() {
			err := m.saveTreeToJsonFile(boundMacroNameEntry.Text)
			log.Printf("createSaveSettings(): %v", err)
		}),
		widget.NewToolbarSpacer(),
		widget.NewToolbarSeparator(),
		widget.NewToolbarAction(theme.MoveDownIcon(), func() {
			m.moveNodeDown(selectedTreeItem)
		}),
		widget.NewToolbarAction(theme.MoveUpIcon(), func() {
			m.moveNodeUp(selectedTreeItem)
		}),
		widget.NewToolbarAction(theme.MediaPlayIcon(), func() {
			macro.ExecuteActionTree()
		}),
		widget.NewToolbarSeparator(),
		widget.NewToolbarSpacer(),
	)
	return tb
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
	return widget.NewSelect(macroList, func(s string) { macro.loadTreeFromJsonFile(s + ".json") })
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

func createItemsCheckBoxes() *widget.Accordion {
	// var boundTargetsCheck []widget.Check
	var (
		accordionItems = widget.NewAccordion()
	)
	accordionItems.MultiOpen = true
	for category, items := range *structs.GetItemsMap() {
		var (
			box           = container.NewVBox()
			scroll        = container.NewVScroll(box)
			categoryCheck = widget.NewCheck("select all", func(checked bool) {
				switch checked {
				case true:
					for _, item := range items {
						boundSelectedItemsMap.SetValue(item.Name, true)
					}
				case false:
					for _, item := range items {
						boundSelectedItemsMap.Delete(item.Name)
					}
				}
				log.Println(selectedItemsMap)
			})
		)
		accordionItems.Append(widget.NewAccordionItem(category, scroll))
		box.Add(categoryCheck)
		for _, item := range items {
			var (
				itemName                = item.Name
				HBoxWithCheckBoxAndIcon = container.NewHBox()
				itemCheckBox            = widget.NewCheck(itemName, func(checked bool) {
					switch checked {
					case true:
						boundSelectedItemsMap.SetValue(itemName, true)
					case false:
						delete(selectedItemsMap, itemName)
					}
					log.Println(selectedItemsMap)
				})
				// itemBool                bool
				// boundItemBool           = binding.BindBool(&itemBool)
				// boundItemCheck          = widget.NewCheckWithData(itemName, boundItemBool)
				resource, imageLoadErr = fyne.LoadResourceFromPath("./images/icons/" + itemName + ".png")
			)
			if imageLoadErr != nil {
				log.Println(imageLoadErr)
				HBoxWithCheckBoxAndIcon.Add(widget.NewIcon(theme.BrokenImageIcon()))
			} else {
				icon := widget.NewIcon(resource)
				HBoxWithCheckBoxAndIcon.Add(icon)
			}
			HBoxWithCheckBoxAndIcon.Add(itemCheckBox)
			box.Add(HBoxWithCheckBoxAndIcon)
		}
	}

	// for category, items := range *structs.GetItemsMap() {
	// 	var (
	// 		box              = container.NewVBox()
	// 		scroll           = container.NewVScroll(box)
	// 		categoryCheckbox = widget.NewCheck("select all", func(checked bool) {
	// 			switch checked {
	// 			case true:
	// 				for _, item := range items {
	// 					selectedItemsMap[item.Name] = true
	// 				}
	// 			case false:
	// 				for _, item := range items {
	// 					delete(selectedItemsMap, item.Name)
	// 				}
	// 			}
	// 			log.Println(selectedItemsMap)
	// 		})
	// 	)
	// 	accordionItems.Append(widget.NewAccordionItem(category, scroll))
	// 	box.Add(categoryCheckbox)
	// 	for _, item := range items {
	// 		var (
	// 			itemName                = item.Name
	// 			HBoxWithCheckBoxAndIcon = container.NewHBox()
	// 			itemCheckBox            = widget.NewCheck(itemName, func(checked bool) {
	// 				switch checked {
	// 				case true:
	// 					selectedItemsMap[itemName] = true // Add selected item to the map
	// 				case false:
	// 					delete(selectedItemsMap, itemName) // Remove unselected item from the map
	// 				}
	// 				log.Println(selectedItemsMap)
	// 			})
	// 			resource, imageLoadErr = fyne.LoadResourceFromPath("./images/icons/" + itemName + ".png")
	// 		)
	// 		utils.HandleError(
	// 			imageLoadErr,
	// 			func() {
	// 				HBoxWithCheckBoxAndIcon.Add(widget.NewIcon(theme.BrokenImageIcon()))
	// 			},
	// 			func() {
	// 				icon := widget.NewIcon(resource)
	// 				HBoxWithCheckBoxAndIcon.Add(icon)
	// 			})
	// 		HBoxWithCheckBoxAndIcon.Add(itemCheckBox)
	// 		box.Add(HBoxWithCheckBoxAndIcon)
	// 	}
	// }
	return accordionItems
}

func (m *macroTree) createUpdateButton() *widget.Button {
	return widget.NewButton("Update", func() {
		node := m.findNode(m.root, selectedTreeItem)
		if selectedTreeItem == "" {
			log.Println("No node selected")
			return
		}
		og := node.String()
		switch node := node.(type) {
		case *structs.WaitAction:
			t, _ := boundTime.Get()
			node.Time = int(t)
		case *structs.MoveAction:
			x, _ := boundMoveX.Get()
			y, _ := boundMoveY.Get()
			node.X = int(x)
			node.Y = int(y)
		case *structs.ClickAction:
			b, _ := boundButton.Get()
			if !b {
				node.Button = "left"
			} else {
				node.Button = "right"
			}
		case *structs.KeyAction:
			k, _ := boundKey.Get()
			s, _ := boundState.Get()
			node.Key = k
			if !s {
				node.State = "down"
			} else {
				node.State = "up"
			}
		case *structs.LoopAction:
			n, _ := boundLoopName.Get()
			c, _ := boundCount.Get()
			node.Name = n
			node.Count = int(c)
		case *structs.ImageSearchAction:
			n, _ := boundImageSearchName.Get()
			s, _ := boundSearchArea.Get()
			t := boundSelectedItemsMap.Keys()
			node.Name = n
			node.SearchBox = *structs.GetSearchBox(s)
			node.Targets = t
		}

		fmt.Printf("Updated node: %+v from '%v' to '%v' \n", node.GetUID(), og, node)

		m.tree.Refresh()
	})
}

func CreateActionMenu() *fyne.Menu {
	basicActionsSubMenu := fyne.NewMenuItem("Basic Actions", nil)
	basicActionsSubMenu.ChildMenu = fyne.NewMenu("")
	advancedActionsSubMenu := fyne.NewMenuItem("Advanced Actions", nil)
	advancedActionsSubMenu.ChildMenu = fyne.NewMenu("")

	waitActionMenuItem := fyne.NewMenuItem("Wait", func() { macro.addActionToTree(&structs.WaitAction{}) })
	mouseMoveActionMenuItem := fyne.NewMenuItem("Mouse Move", func() { macro.addActionToTree(&structs.MoveAction{}) })
	clickActionMenuItem := fyne.NewMenuItem("Click", func() { macro.addActionToTree(&structs.ClickAction{}) })
	keyActionMenuItem := fyne.NewMenuItem("Key", func() { macro.addActionToTree(&structs.KeyAction{}) })

	loopActionMenuItem := fyne.NewMenuItem("Loop", func() { macro.addActionToTree(&structs.LoopAction{}) })
	imageSearchActionMenuItem := fyne.NewMenuItem("Image Search", func() { macro.addActionToTree(&structs.ImageSearchAction{}) })
	ocrActionMenuItem := fyne.NewMenuItem("Image Search", func() { macro.addActionToTree(&structs.OcrAction{}) })

	basicActionsSubMenu.ChildMenu.Items = append(basicActionsSubMenu.ChildMenu.Items,
		waitActionMenuItem,
		mouseMoveActionMenuItem,
		clickActionMenuItem,
		keyActionMenuItem,
	)

	advancedActionsSubMenu.ChildMenu.Items = append(advancedActionsSubMenu.ChildMenu.Items,
		loopActionMenuItem,
		imageSearchActionMenuItem,
		ocrActionMenuItem,
	)

	return fyne.NewMenu("Add Action", basicActionsSubMenu, advancedActionsSubMenu)
}
