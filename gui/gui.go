package gui

import (
    "Dark-And-Darker/structs"
    "Dark-And-Darker/utils"
    "fyne.io/fyne/v2"
    "fyne.io/fyne/v2/app"
    "fyne.io/fyne/v2/container"
    "fyne.io/fyne/v2/layout"
    "fyne.io/fyne/v2/widget"
    "github.com/go-vgo/robotgo"
    "log"
)
//var selectedItemsMap *map[string]bool

var selectedItemsMap = make(map[string]bool)
var selectedItemsMapPtr = &selectedItemsMap

func Load(){
	a := app.New()
	w := a.NewWindow("Squire")
	w.Resize(fyne.NewSize(1500,1500))
//-------------------------------------------------------------------------------Tab 1
	//selectedItemsMap = make(map[string]bool)
	itemsCheckBoxes 	:= ItemsCheckBoxes()
	itemsCheckBoxes.MultiOpen = true
	//searchBoxSelector 	:= SearchBoxSelector()
	//imageSearchButton 	:= ImageSearchButton(itemsCheckBoxes, searchBoxSelector)
	tab1 := container.NewTabItem("Macro Builder", container.New(layout.NewGridLayout(2),
		container.NewScroll(
            //itemsCheckBoxes,
			//searchBoxSelector,
			itemsCheckBoxes,
//			widget.NewAccordion(
//				widget.NewAccordionItem(
//					"Items",
//                    itemsCheckBoxes),
//				),
	//		imageSearchButton,
			//widget.NewAccordion(),
		),
	))
//-------------------------------------------------------------------------------------Tab 2
	tab2 := container.NewTabItem("macro builder", container.New(layout.NewGridLayout(1),
		widget.NewSelect([]string{"1", "2"}, func(value string){
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
	var itemsList []string
	accordion := widget.NewAccordion()
	for category, items := range itemsByCategory.Categories {
		itemsList = []string{}
		log.Println(category)
		log.Println(items)
		log.Println(itemsList)

        for _, item := range items {
			itemsList = append(itemsList, item.Name)
		}
		log.Println(itemsList)
		checkGroup := widget.NewCheckGroup(itemsList, func(selected []string){})
		checkGroup.OnChanged = func(selected []string){
			for _, item := range selected {
                	(selectedItemsMap)[item] = true // Add selected items to the map
	            }
	            for _, item := range checkGroup.Options {
	                if !contains(selected, item) {
	                    delete(selectedItemsMap, item) // Remove unselected items from the map
	                }
            	}
				log.Println(selectedItemsMap)
		}
		accordionItem := widget.NewAccordionItem(category, checkGroup)
		accordion.Append(accordionItem)
	}
	return accordion
}
func contains(slice []string, item string) bool {
    for _, s := range slice {
        if s == item {
            return true
        }
    }
    return false
}

func SearchBoxSelector() *widget.Select{
	sbcMap := *structs.SearchBoxCoordinatesMap()
	var names []string
	for _, sbc := range sbcMap{
		names = append(names, sbc.AreaName)
	}
	return widget.NewSelect(names, func(value string){})
}

func ImageSearchButton(itemSelector *widget.CheckGroup, searchBoxSelector *widget.Select) *widget.Button {
	return widget.NewButton("Find image", func() {
		err := robotgo.ActiveName("Fleet")
		if err != nil {
            log.Printf("robotgo.ActiveName failed:%d\n", err)
			return 
		}
		for _, v := range itemSelector.Selected{
			item, _ := structs.GetItem(v)
			sbc := structs.GetSearchBoxCoordinates(searchBoxSelector.Selected)
			ip := "./images/" + item.Name + ".png"
			x, y := utils.ImageSearch(sbc, ip)
			robotgo.Move(x, y)
		}
	})
}