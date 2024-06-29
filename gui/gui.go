package gui

import (
	"Dark-And-Darker/actions"
	"Dark-And-Darker/structs"
	"Dark-And-Darker/utils"
	"log"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/app"
	"fyne.io/fyne/v2/container"
	"fyne.io/fyne/v2/layout"
	"fyne.io/fyne/v2/theme"
	"fyne.io/fyne/v2/widget"
	"github.com/go-vgo/robotgo"
)

//var selectedItemsMap *map[string]bool

var selectedItemsMap = make(map[string]bool)

var macroSettingsParameters map[string]interface{}

var actionsArr []actions.Action

var actionsList = widget.NewList(
	func() int { return len(actionsArr) },
	func() fyne.CanvasObject { return container.NewHBox(widget.NewLabel("Text")) },
	func(lii widget.ListItemID, co fyne.CanvasObject) {
		// switch actionsArr[lii].Parameters := actionsArr[lii].Parameters.(type){
		// case

		// }
		co.(*fyne.Container).Objects[0].(*widget.Label).SetText(actionsArr[lii].PrintParams())
	},
)

var goToSelector = widget.NewSelect([]string{"Stash Tab", "Play Tab"}, func(s string) {})
var goToSettingsForm = widget.Form{
	Items: []*widget.FormItem{
		{Text: "Go To", Widget: goToSelector},
	},
	OnSubmit: func() {
		action := actions.Goto{
			Place:       goToSelector.Selected,
			Coordinates: [2]int{structs.GetSearchSpotCoordinates(goToSelector.Selected).X, structs.GetSearchSpotCoordinates(goToSelector.Selected).Y},
		}
		actionsArr = append(actionsArr, action)
		actionsList.Refresh()
	},
}
var actionSelector = widget.NewSelect([]string{"Go To", "Search", "Click"}, func(s string) {
	goToSettingsForm.Hide()
	//searchSettingsForm.Hide()
	//clickSettingsForm.Hide()
	switch s {
	case "Go To":
		goToSettingsForm.Show()
	case "Search":
		//searchSettingsForm.Show()
	case "Click":
		//clickSettingsForm.Show()
	}
})

func Load() {
	a := app.New()
	w := a.NewWindow("Squire")
	w.Resize(fyne.NewSize(1000, 1000))
	//-----------------------------------------------------------------------------------------------------------------------------------------#Tab 1
	//-----------------------------------------------------------------------------------------------------------------------------------------##Col 1
	//-----------------------------------------------------------------------------------------------------------------------------------------###ITEMS
	itemsCheckBoxes := ItemsCheckBoxes()
	itemsCheckBoxes.MultiOpen = true
	//-----------------------------------------------------------------------------------------------------------------------------------------##Col 2
	//-----------------------------------------------------------------------------------------------------------------------------------------###MACRO SELECTOR
	var macroSettingsContainer *fyne.Container
	macros := []string{
		"Custom Macro",
		"Move Items Player -> Stash", // use icons here?
		"Move Items Player <- Stash",
		"Empty Player Inventory",
		"Sell Treasures",
	}
	macroSelector := widget.NewSelect(macros, func(value string) {
		if value != "Custom Macro" {
			ToggleWidgets(macroSettingsContainer, false)
		} else {
			ToggleWidgets(macroSettingsContainer, true)
		}
	})
	//-----------------------------------------------------------------------------------------------------------------------------------------###GO TO

	//-----------------------------------------------------------------------------------------------------------------------------------------###SEARCH AREA
	searchBoxSelector := SearchBoxSelector()
	searchBoxSelector.SetSelected("Whole Screen")
	//-----------------------------------------------------------------------------------------------------------------------------------------###MOUSE AND KEYBOARD
	mouseButtonToggle := widget.NewCheck("", func(b bool) {})
	mouseContainer := container.NewHBox(
		mouseButtonToggle,
		widget.NewLabel("Left"),
		widget.NewSlider(0, 1),
		widget.NewLabel("Right"),
	)
	keyboardContainer := container.NewHBox(
		widget.NewCheck("Alt", func(b bool) {}),
		widget.NewCheck("Shift", func(b bool) {}),
		widget.NewCheck("Ctrl", func(b bool) {}),
	)

	startMacroButton := StartMacroButton(&selectedItemsMap, searchBoxSelector)
	macroSettingsContainer = container.NewVBox(widget.NewLabelWithStyle("Search Area", fyne.TextAlignCenter, fyne.TextStyle{Bold: true}),
		searchBoxSelector,
		widget.NewLabelWithStyle("Buttons", fyne.TextAlignCenter, fyne.TextStyle{Bold: true}),
		widget.NewLabel("Mouse"),
		mouseContainer,
		widget.NewLabel("Keyboard"),
		keyboardContainer,
		layout.NewSpacer(),
		startMacroButton,
	)
	ToggleWidgets(macroSettingsContainer, false)

	tab1 := container.NewTabItem("Macro Builder", container.New(layout.NewGridLayout(2),
		container.New(
			layout.NewGridLayout(2),
			itemsCheckBoxes,
			container.NewVBox(
				macroSelector,
				macroSettingsContainer,
				layout.NewSpacer(),
				widget.NewLabelWithStyle("Dungeon Setup", fyne.TextAlignCenter, fyne.TextStyle{Bold: true}),
				widget.NewCheck("Check Stash before merchants", func(b bool) {}),
				widget.NewButton("Setup for Dungeon", func() {}),
			),
		),
	),
	// container.NewVBox(
	// 	widget.NewLabelWithStyle("Search Area", fyne.TextAlignCenter, fyne.TextStyle{Bold: true}),
	// 	searchBoxSelector,
	// 	widget.NewCheck("Build your own Macro?", func(b bool) {}),
	// 	widget.NewLabelWithStyle("Game Related", fyne.TextAlignCenter, fyne.TextStyle{Bold: true}),
	// 	widget.NewCheck("Check Stash before merchants", func(b bool) {}),
	// 	widget.NewLabelWithStyle("Buttons", fyne.TextAlignCenter, fyne.TextStyle{Bold: true}),
	// 	widget.NewLabel("Mouse"),
	// 	container.NewHBox(
	// 		mouseButtonToggle,
	// 		widget.NewLabel("Left"),
	// 		widget.NewSlider(0, 1),
	// 		widget.NewLabel("Right"),
	// 	),
	// 	widget.NewLabel("Keyboard"),
	// 	container.NewHBox(
	// 		widget.NewCheck("Alt", func(b bool) {}),
	// 		widget.NewCheck("Shift", func(b bool) {}),
	// 		widget.NewCheck("Ctrl", func(b bool) {}),
	// 	),

	)

	//-------------------------------------------------------------------------------------Tab 2
	tab2 := container.NewTabItem("Tab 2", container.New(layout.NewGridLayout(2),
		container.NewGridWithColumns(2,
			container.NewVBox(
				actionSelector,
			),
			container.NewVBox(
				&goToSettingsForm,
				// layout.NewSpacer(),
				// widget.NewButton("Add", func() {}),
				// widget.NewButton("Remove", func() {}),
			),
		),
		container.NewGridWithColumns(1,
			actionsList,
			container.NewVBox(
				container.NewGridWithColumns(2,
					widget.NewButton("-", func() {}),
					widget.NewButton("+", func() {}),
				),
			),
		),
	))

	//imageDropDown := widget.NewAccordion()
	tabs := container.NewAppTabs(
		tab1,
		tab2,
	)

	tabs.SetTabLocation(container.TabLocationBottom)
	w.SetContent(tabs)

	w.ShowAndRun()
}

func ToggleWidgets(c *fyne.Container, b bool) {
	for _, obj := range c.Objects {
		switch obj := obj.(type) {
		case fyne.Disableable:
			if b {
				obj.Enable()
			} else {
				obj.Disable()
			}
		case *fyne.Container:
			ToggleWidgets(obj, b)
		}
	}
}

func OffsetMove(x int, y int) {
	robotgo.Move(x+1920, y+utils.YOffset)
	robotgo.Sleep(1)
}

//func ItemsCheckBoxes() *widget.Select {
//	items := *structs.ItemsMap()
//	var names []string
//	for _, item := range items{
//		names = append(names, item.Name)
//	}
//	return widget.NewSelect(names, func(value string){})
//}

func ItemsCheckBoxes() *widget.Accordion {
	itemsByCategory := *structs.ItemsFromFile()
	accordionItems := widget.NewAccordion()
	for category, items := range itemsByCategory.Categories {
		box := container.NewVBox()
		scroll := container.NewVScroll(box)
		for _, item := range items {
			checkBoxWithIcon := container.NewHBox()
			func(itemName string) {
				checkBox := widget.NewCheck(itemName, func(checked bool) {})
				checkBox.OnChanged = func(checked bool) {
					if checked {
						log.Println(itemName)
						selectedItemsMap[itemName] = true // Add selected item to the map
					} else {
						delete(selectedItemsMap, itemName) // Remove unselected item from the map
					}
					log.Println(selectedItemsMap)
				}
				resource, err := fyne.LoadResourceFromPath("./images/" + itemName + ".png")
				if err != nil {
					log.Println(err)
					checkBoxWithIcon.Add(widget.NewIcon(theme.BrokenImageIcon()))
				} else {
					icon := widget.NewIcon(resource)
					checkBoxWithIcon.Add(icon)
				}
				checkBoxWithIcon.Add(checkBox)
				box.Add(checkBoxWithIcon)
			}(item.Name)
		}
		accordionItems.Append(widget.NewAccordionItem(category, scroll))
	}
	return accordionItems
}

func SearchBoxSelector() *widget.Select {
	sbcMap := *structs.SearchBoxCoordinatesMap()
	var names []string
	for _, sbc := range sbcMap {
		names = append(names, sbc.AreaName)
	}
	return widget.NewSelect(names, func(value string) {})
}

func StartMacroButton(selectedItemsMap *map[string]bool, searchBoxSelector *widget.Select) *widget.Button {
	return widget.NewButton("Start Macro", func() {
		err := robotgo.ActiveName("Dark and Darker")
		if err != nil {
			log.Printf("robotgo.ActiveName failed:%d\n", err)
			return
		}

		posArr := []robotgo.Point{}
		for v := range *selectedItemsMap {
			item, _ := structs.GetItem(v)
			sbc := structs.GetSearchBoxCoordinates(searchBoxSelector.Selected)
			found := utils.ImageSearch(sbc, item.Name)
			posArr = append(found, posArr...)
			//OffsetMove(x, y)
		}
		for _, position := range posArr {
			x := position.X
			y := position.Y
			OffsetMove(x, y)
			robotgo.MilliSleep(200)
		}
	})
}
