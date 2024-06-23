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
func Load(){
	width, height := robotgo.GetScreenSize()
	log.Println(height + width)
	a := app.New()
	w := a.NewWindow("Squire")
	w.Resize(fyne.NewSize(500,500))
	
//-------------------------------------------------------------------------------Tab 1
	itemSelector 		:= ItemSelector()
	searchBoxSelector 	:= SearchBoxSelector()
	imageSearchButton 	:= ImageSearchButton(itemSelector, searchBoxSelector)
	tab1 := container.NewTabItem("image screenshot", container.New(layout.NewGridLayout(2),
		container.NewVBox(
			searchBoxSelector,
			widget.NewAccordion(
				widget.NewAccordionItem(
					"Items",
					itemSelector),
				),
			imageSearchButton,
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

//func ItemSelector() *widget.Select {
//	items := *structs.ItemsMap()
//	var names []string
//	for _, item := range items{
//		names = append(names, item.Name)
//	}
//	return widget.NewSelect(names, func(value string){})
//}

func ItemSelector() *widget.CheckGroup {
	items := *structs.ItemsMap()
	var names []string
	for _, item := range items{
		names = append(names, item.Name)
	}
	return widget.NewCheckGroup(names, func(value []string){})
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
			item := structs.GetItem(v)
			sbc := structs.GetSearchBoxCoordinates(searchBoxSelector.Selected)
			ip := "./images/" + item.Name + ".png"
			x, y := utils.ImageSearch(sbc, ip)
			robotgo.Move(x, y)
		}
	})
}