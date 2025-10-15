package binders

import (
	"Squire/internal/config"
	"Squire/internal/programs/actions"
	"Squire/internal/programs/coordinates"
	"Squire/ui"
	"log"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/container"
	"fyne.io/fyne/v2/data/binding"
	"fyne.io/fyne/v2/widget"
	"github.com/lithammer/fuzzysearch/fuzzy"
)

func SetAccordionPointsLists(acc *widget.Accordion) {
	for key, pro := range GetBoundPrograms() {
		lists := struct {
			boundPointSearchBar *widget.Entry
			boundPointList      *widget.List
			pointSearchList     []string
		}{
			boundPointSearchBar: &widget.Entry{},
			boundPointList:      &widget.List{},
			pointSearchList:     GetPointsAsStringSlice(key, config.MainMonitorSizeString),
		}

		lists.boundPointList = widget.NewList(
			func() int {
				return len(pro.PointsBindings)
			},
			func() fyne.CanvasObject {
				return widget.NewLabel("template")
			},
			func(id widget.ListItemID, co fyne.CanvasObject) {
				boundPoint := pro.PointsBindings[id]
				name, _ := boundPoint.GetItem("Name")
				label := co.(*widget.Label)
				// nameLabel := c.Objects[0].(*widget.Label)
				// coordsLabel := c.Objects[1].(*widget.Label)

				// t := fmt.Sprintf("%v| %d, %d", poi.Name, poi.X, poi.Y)
				label.Bind(name.(binding.String))

				// label.SetText((fmt.Sprintf("%v: %d, %d", poi.Name, poi.X, poi.Y)))
			},
		)

		lists.boundPointList.OnSelected = func(id widget.ListItemID) {
			boundMacro := boundMacros[ui.GetUi().Mui.MTabs.SelectedTab().Macro.Name]
			boundPoint := pro.PointsBindings[id]
			bindPointWidgets(boundPoint)
			ui.GetUi().ActionTabs.BoundMove = boundPoint
			if v, ok := ui.GetUi().Mui.MTabs.SelectedTab().Macro.Root.GetAction(ui.GetUi().Mui.MTabs.SelectedTab().SelectedNode).(*actions.Move); ok {
				log.Println("This is getting called")
				name, _ := boundPoint.GetValue("Name")
				x, _ := boundPoint.GetValue("X")
				y, _ := boundPoint.GetValue("Y")
				v.Point = coordinates.Point{
					Name: name.(string),
					X:    x.(int),
					Y:    y.(int),
				}
				boundMacro.bindAction(v)
				// boundMacros[ui.GetUi().Mui.MTabs.Selected().Text].BoundSelectedAction = boundPoint //ui.GetUi().ActionTabs.BoundMove
				// ui.GetUi().Mui.MTabs.SelectedTab().
			}
			lists.boundPointList.Unselect(id)
			// if GetActionui.GetUi().Mui.MTabs.SelectedTab().SelectedNode
			// if _, ok := boundMacros[ui.GetUi().Mui.MTabs.Selected().Text].BoundSelectedAction; ok {

			// }
		}
		lists.boundPointSearchBar = &widget.Entry{
			PlaceHolder: "Search here",
			OnChanged: func(s string) {
				// defaultList := pro.Coordinates[config.MainMonitorSizeString].Points
				defaultList := GetPointsAsStringSlice(key, config.MainMonitorSizeString)
				defer lists.boundPointList.ScrollToTop()
				defer lists.boundPointList.Refresh()

				if s == "" {
					lists.pointSearchList = defaultList
					return
				}
				lists.pointSearchList = []string{}
				for _, i := range defaultList {
					if fuzzy.MatchFold(s, i) {
						lists.pointSearchList = append(lists.pointSearchList, i)
					}
				}
			},
		}
		programPointListWidget := *widget.NewAccordionItem(
			pro.Name,
			container.NewBorder(
				lists.boundPointSearchBar,
				nil, nil, nil,
				lists.boundPointList,
			),
		)
		acc.Append(&programPointListWidget)
	}
}

func SetAccordionSearchAreasLists(acc *widget.Accordion) {
	for _, pro := range GetBoundPrograms() {
		lists := struct {
			boundSASearchBar     *widget.Entry
			boundSAList          *widget.List
			searchAreaSearchList []string
		}{
			boundSASearchBar: &widget.Entry{},
			boundSAList:      &widget.List{},
			// searchAreaSearchList: GetPointsAsStringSlice(key, config.MainMonitorSizeString),
		}

		lists.boundSAList = widget.NewList(
			func() int {
				return len(pro.SearchAreaBindings)
			},
			func() fyne.CanvasObject {
				return widget.NewLabel("template")
			},
			func(id widget.ListItemID, co fyne.CanvasObject) {
				boundSA := pro.SearchAreaBindings[id]
				name, _ := boundSA.GetItem("Name")
				label := co.(*widget.Label)
				// nameLabel := c.Objects[0].(*widget.Label)
				// coordsLabel := c.Objects[1].(*widget.Label)

				// t := fmt.Sprintf("%v| %d, %d", poi.Name, poi.X, poi.Y)
				label.Bind(name.(binding.String))

				// label.SetText((fmt.Sprintf("%v: %d, %d", poi.Name, poi.X, poi.Y)))
			},
		)

		lists.boundSAList.OnSelected = func(id widget.ListItemID) {
			boundMacro := boundMacros[ui.GetUi().Mui.MTabs.SelectedTab().Macro.Name]
			boundSA := pro.SearchAreaBindings[id]
			bindSearchAreaWidgets(boundSA)
			ui.GetUi().ActionTabs.BoundSearchArea = boundSA
			if v, ok := ui.GetUi().Mui.MTabs.SelectedTab().Macro.Root.GetAction(ui.GetUi().Mui.MTabs.SelectedTab().SelectedNode).(*actions.ImageSearch); ok {
				log.Println("This is getting called")
				name, _ := boundSA.GetValue("Name")
				x1, _ := boundSA.GetValue("LeftX")
				y1, _ := boundSA.GetValue("TopY")
				x2, _ := boundSA.GetValue("RightX")
				y2, _ := boundSA.GetValue("BottomY")
				v.SearchArea = coordinates.SearchArea{
					Name:    name.(string),
					LeftX:   x1.(int),
					TopY:    y1.(int),
					RightX:  x2.(int),
					BottomY: y2.(int),
				}
				boundMacro.bindAction(v)
				// boundMacros[ui.GetUi().Mui.MTabs.Selected().Text].BoundSelectedAction = boundPoint //ui.GetUi().ActionTabs.BoundMove
				// ui.GetUi().Mui.MTabs.SelectedTab().
			}
			lists.boundSAList.Unselect(id)
			// if GetActionui.GetUi().Mui.MTabs.SelectedTab().SelectedNode
			// if _, ok := boundMacros[ui.GetUi().Mui.MTabs.Selected().Text].BoundSelectedAction; ok {

			// }
		}
		// lists.boundSASearchBar = &widget.Entry{
		// 	PlaceHolder: "Search here",
		// 	OnChanged: func(s string) {
		// 		// defaultList := pro.Coordinates[config.MainMonitorSizeString].Points
		// 		defaultList := GetPointsAsStringSlice(key, config.MainMonitorSizeString)
		// 		defer lists.boundSAList.ScrollToTop()
		// 		defer lists.boundSAList.Refresh()

		// 		if s == "" {
		// 			lists.searchAreaSearchList = defaultList
		// 			return
		// 		}
		// 		lists.searchAreaSearchList = []string{}
		// 		for _, i := range defaultList {
		// 			if fuzzy.MatchFold(s, i) {
		// 				lists.searchAreaSearchList = append(lists.searchAreaSearchList, i)
		// 			}
		// 		}
		// 	},
		// }
		programSAListWidget := *widget.NewAccordionItem(
			pro.Name,
			container.NewBorder(
				lists.boundSASearchBar,
				nil, nil, nil,
				lists.boundSAList,
			),
		)
		acc.Append(&programSAListWidget)
	}
}
