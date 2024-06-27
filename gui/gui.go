package gui

import (
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

func Load() {
	a := app.New()
	w := a.NewWindow("Squire")
	w.Resize(fyne.NewSize(1000, 1000))
	//-------------------------------------------------------------------------------Tab 1
	//selectedItemsMap = make(map[string]bool)
	itemsCheckBoxes := ItemsCheckBoxes()
	itemsCheckBoxes.MultiOpen = true
	searchBoxSelector := SearchBoxSelector()
	imageSearchButton := ImageSearchButton(&selectedItemsMap, searchBoxSelector)
	tab1 := container.NewTabItem("Macro Builder", container.New(layout.NewGridLayout(2),
		container.New(
			layout.NewGridLayout(2),
			container.NewVBox(searchBoxSelector, layout.NewSpacer(), imageSearchButton),
			itemsCheckBoxes,
		),
		widget.NewButton("move mouse", func() { OffsetMove(400, 400) }),
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
					icon.Resize(fyne.NewSquareSize(50))
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

func ImageSearchButton(selectedItemsMap *map[string]bool, searchBoxSelector *widget.Select) *widget.Button {
	return widget.NewButton("Find items", func() {
		err := robotgo.ActiveName("Dark and Darker")
		if err != nil {
			log.Printf("robotgo.ActiveName failed:%d\n", err)
			return
		}
		for v := range *selectedItemsMap {
			item, _ := structs.GetItem(v)
			sbc := structs.GetSearchBoxCoordinates(searchBoxSelector.Selected)
			x, y := utils.ImageSearch(sbc, item.Name)
			robotgo.Move(x, y)
			//OffsetMove(x, y)
		}
	})
}
