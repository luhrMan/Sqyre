package gui

import (
	"Dark-And-Darker/structs"
	"Dark-And-Darker/utils"
	"log"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/app"
	"fyne.io/fyne/v2/container"
	"fyne.io/fyne/v2/layout"
	"fyne.io/fyne/v2/widget"
	"github.com/go-vgo/robotgo"
)

//var selectedItemsMap *map[string]bool

var selectedItemsMap = make(map[string]bool)

func Load() {
	a := app.New()
	w := a.NewWindow("Squire")
	w.Resize(fyne.NewSize(1000, 1000))
	//-------------------------------------------------------------------------------Tab 1
	//selectedItemsMap = make(map[string]bool)
	itemsCheckBoxes := ItemsCheckBoxes()
	itemsCheckBoxes.MultiOpen = true
	searchBoxSelector := SearchBoxSelector()
	//imageSearchButton 	:= ImageSearchButton(itemsCheckBoxes, searchBoxSelector)
	tab1 := container.NewTabItem("Macro Builder", container.New(layout.NewGridLayout(2),
		container.New(
			layout.NewGridLayout(2),
			container.NewVBox(searchBoxSelector),
			itemsCheckBoxes,
		),
	),
	//			widget.NewAccordion(
	//				widget.NewAccordionItem(
	//					"Items",
	//                    itemsCheckBoxes),
	//				),
	//		imageSearchButton,
	//widget.NewAccordion(),
	)
	//-------------------------------------------------------------------------------------Tab 2
	tab2 := container.NewTabItem("macro builder", container.New(layout.NewGridLayout(1),
		widget.NewSelect([]string{"1", "2"}, func(value string) {
			log.Println(value)
		})),
	)

	//imageDropDown := widget.NewAccordion()
	tabs := container.NewAppTabs(
		tab1,
		tab2,
	)

	tabs.SetTabLocation(container.TabLocationBottom)
	w.SetContent(tabs)

	w.ShowAndRun()
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
				box.Add(checkBox)
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

func ImageSearchButton(itemSelector *widget.CheckGroup, searchBoxSelector *widget.Select) *widget.Button {
	return widget.NewButton("Find image", func() {
		err := robotgo.ActiveName("Fleet")
		if err != nil {
			log.Printf("robotgo.ActiveName failed:%d\n", err)
			return
		}
		for _, v := range itemSelector.Selected {
			item, _ := structs.GetItem(v)
			sbc := structs.GetSearchBoxCoordinates(searchBoxSelector.Selected)
			ip := "./images/" + item.Name + ".png"
			x, y := utils.ImageSearch(sbc, ip)
			robotgo.Move(x, y)
		}
	})
}
