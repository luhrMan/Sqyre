package gui

import (
	"Dark-And-Darker/structs"
	"log"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/app"
	"fyne.io/fyne/v2/container"
	"fyne.io/fyne/v2/theme"
	"fyne.io/fyne/v2/widget"
)

var selectedItemsMap = make(map[string]bool)

func Load() {
	a := app.New()
	w := a.NewWindow("Squire")
	w.Resize(fyne.NewSize(1000, 1000))

	//-----------------------------------------------------------------------------------------------------------------------------------------#Tab 1
	//-----------------------------------------------------------------------------------------------------------------------------------------##Col 1
	//-----------------------------------------------------------------------------------------------------------------------------------------###ITEMS
	itemsCheckBoxes := ItemsCheckBoxes()
	itemsCheckBoxes.MultiOpen = true

	w.ShowAndRun()
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

// func SearchBoxSelector() *widget.Select {
// 	sbcMap := *structs.SearchBoxMap()
// 	var names []string
// 	for _, sbc := range sbcMap {
// 		names = append(names, sbc.AreaName)
// 	}
// 	return widget.NewSelect(names, func(value string) {})
// }

// func StartMacroButton(selectedItemsMap *map[string]bool, searchBoxSelector *widget.Select) *widget.Button {
// 	return widget.NewButton("Start Macro", func() {
// 		err := robotgo.ActiveName("Dark and Darker")
// 		if err != nil {
// 			log.Printf("robotgo.ActiveName failed:%d\n", err)
// 			return
// 		}

// 		posArr := []robotgo.Point{}
// 		for v := range *selectedItemsMap {
// 			item, _ := structs.GetItem(v)
// 			sbc := structs.GetSearchBox(searchBoxSelector.Selected)
// 			found := utils.ImageSearch(sbc, item.Name)
// 			posArr = append(found, posArr...)
// 			//OffsetMove(x, y)
// 		}
// 		for _, position := range posArr {
// 			x := position.X
// 			y := position.Y
// 			OffsetMove(x, y)
// 			robotgo.MilliSleep(200)
// 		}
// 	})
// }
