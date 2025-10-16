package binders

import (
	"Squire/internal/assets"
	"Squire/internal/config"
	"Squire/internal/models/actions"
	"Squire/internal/models/coordinates"
	"Squire/ui"
	"image/color"
	"log"
	"slices"
	"strings"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/canvas"
	"fyne.io/fyne/v2/container"
	"fyne.io/fyne/v2/data/binding"
	"fyne.io/fyne/v2/theme"
	"fyne.io/fyne/v2/widget"
	"github.com/lithammer/fuzzysearch/fuzzy"
)

func SetAccordionPointsLists(acc *widget.Accordion) {
	for key, pb := range GetBoundPrograms() {
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
				return len(pb.PointsBindings)
			},
			func() fyne.CanvasObject {
				return widget.NewLabel("template")
			},
			func(id widget.ListItemID, co fyne.CanvasObject) {
				boundPoint := pb.PointsBindings[id]
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
			boundPoint := pb.PointsBindings[id]
			bindPointWidgets(boundPoint)
			ui.GetUi().ActionTabs.BoundPoint = boundPoint
			// ui.GetUi().ActionTabs.BoundMove = boundPoint
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
			pb.Program.Name,
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
	for _, pb := range GetBoundPrograms() {
		lists := struct {
			boundSASearchBar *widget.Entry
			boundSAList      *widget.List
			// searchAreaSearchList []string
		}{
			boundSASearchBar: &widget.Entry{},
			boundSAList:      &widget.List{},
			// searchAreaSearchList: GetPointsAsStringSlice(key, config.MainMonitorSizeString),
		}

		lists.boundSAList = widget.NewList(
			func() int {
				return len(pb.SearchAreaBindings)
			},
			func() fyne.CanvasObject {
				return widget.NewLabel("template")
			},
			func(id widget.ListItemID, co fyne.CanvasObject) {
				boundSA := pb.SearchAreaBindings[id]
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
			boundSA := pb.SearchAreaBindings[id]
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
			pb.Program.Name,
			container.NewBorder(
				lists.boundSASearchBar,
				nil, nil, nil,
				lists.boundSAList,
			),
		)
		acc.Append(&programSAListWidget)
	}
}

func SetAccordionItemsLists(acc *widget.Accordion) {
	// bSearchList = binding.BindStringList(&searchList)
	var (
		ats   = ui.GetUi().ActionTabs
		icons = *assets.BytesToFyneIcons()
	)
	for _, pb := range GetBoundPrograms() {
		var (
			searchList = slices.Clone(pb.Program.GetItems().SortByCategory())
			// bSearchList binding.ExternalStringList
		)
		lists := struct {
			boundItemSearchBar *widget.Entry
			boundItemGrid      *widget.GridWrap
			ItemSearchList     []string
		}{
			boundItemSearchBar: &widget.Entry{},
			boundItemGrid:      &widget.GridWrap{},
			// searchAreaSearchList: GetPointsAsStringSlice(key, config.MainMonitorSizeString),
		}
		lists.ItemSearchList = pb.Program.GetItems().SortByCategory()
		lists.boundItemGrid = widget.NewGridWrap(
			func() int {
				return len(pb.ItemBindings)
			},
			func() fyne.CanvasObject {
				rect := canvas.NewRectangle(color.RGBA{})
				rect.SetMinSize(fyne.NewSquareSize(45))
				rect.CornerRadius = 5

				icon := canvas.NewImageFromResource(theme.BrokenImageIcon())
				icon.SetMinSize(fyne.NewSquareSize(40))
				icon.FillMode = canvas.ImageFillOriginal

				stack := container.NewStack(rect, container.NewPadded(icon))
				return stack
			},
			func(id widget.GridWrapItemID, o fyne.CanvasObject) {
				boundItem := pb.ItemBindings[id]
				name, _ := boundItem.GetValue("Name")
				// name = strings.ToLower(name.(string))

				stack := o.(*fyne.Container)
				rect := stack.Objects[0].(*canvas.Rectangle)
				icon := stack.Objects[1].(*fyne.Container).Objects[0].(*canvas.Image)

				ist, _ := ats.BoundImageSearch.GetValue("Targets")
				t := ist.([]string)

				if slices.Contains(t, pb.Program.Name+"|"+name.(string)) {
					rect.FillColor = color.RGBA{R: 0, G: 128, B: 0, A: 128}
				} else {
					rect.FillColor = color.RGBA{}
				}

				path := name.(string) + ".png"
				if icons[path] != nil {
					icon.Resource = icons[path]
				} else {
					icon.Resource = theme.BrokenImageIcon()
				}
				o.Refresh()
				// nameLabel := c.Objects[0].(*widget.Label)
				// coordsLabel := c.Objects[1].(*widget.Label)

				// t := fmt.Sprintf("%v| %d, %d", poi.Name, poi.X, poi.Y)
				// label.Bind(name.(binding.String))

				// label.SetText((fmt.Sprintf("%v: %d, %d", poi.Name, poi.X, poi.Y)))
			},
		)
		lists.boundItemGrid.OnSelected = func(id widget.GridWrapItemID) {
			defer lists.boundItemGrid.UnselectAll()
			defer lists.boundItemGrid.RefreshItem(id)
			ist, _ := ats.BoundImageSearch.GetValue("Targets")
			t := ist.([]string)
			log.Println(lists.ItemSearchList[id])
			name := strings.Split(lists.ItemSearchList[id], "|")[1]
			item := searchList[id]
			if !slices.Contains(t, name) {
				t = append(t, name)
			} else {
				i := slices.Index(t, item)
				if i != -1 {
					t = slices.Delete(t, i, i+1)
				}
			}
			ats.BoundImageSearch.SetValue("Targets", t)
		}

		// lists.boundSAList = widget.NewList(
		// 	func() int {
		// 		return len(pro.SearchAreaBindings)
		// 	},
		// 	func() fyne.CanvasObject {
		// 		return widget.NewLabel("template")
		// 	},
		// 	func(id widget.ListItemID, co fyne.CanvasObject) {
		// 		boundSA := pro.SearchAreaBindings[id]
		// 		name, _ := boundSA.GetItem("Name")
		// 		label := co.(*widget.Label)
		// 		// nameLabel := c.Objects[0].(*widget.Label)
		// 		// coordsLabel := c.Objects[1].(*widget.Label)

		// 		// t := fmt.Sprintf("%v| %d, %d", poi.Name, poi.X, poi.Y)
		// 		label.Bind(name.(binding.String))

		// 		// label.SetText((fmt.Sprintf("%v: %d, %d", poi.Name, poi.X, poi.Y)))
		// 	},
		// )

		// lists.boundSAList.OnSelected = func(id widget.ListItemID) {
		// 	boundMacro := boundMacros[ui.GetUi().Mui.MTabs.SelectedTab().Macro.Name]
		// 	boundSA := pro.SearchAreaBindings[id]
		// 	bindSearchAreaWidgets(boundSA)
		// 	ui.GetUi().ActionTabs.BoundSearchArea = boundSA
		// 	if v, ok := ui.GetUi().Mui.MTabs.SelectedTab().Macro.Root.GetAction(ui.GetUi().Mui.MTabs.SelectedTab().SelectedNode).(*actions.ImageSearch); ok {
		// 		log.Println("This is getting called")
		// 		name, _ := boundSA.GetValue("Name")
		// 		x1, _ := boundSA.GetValue("LeftX")
		// 		y1, _ := boundSA.GetValue("TopY")
		// 		x2, _ := boundSA.GetValue("RightX")
		// 		y2, _ := boundSA.GetValue("BottomY")
		// 		v.SearchArea = coordinates.SearchArea{
		// 			Name:    name.(string),
		// 			LeftX:   x1.(int),
		// 			TopY:    y1.(int),
		// 			RightX:  x2.(int),
		// 			BottomY: y2.(int),
		// 		}
		// 		boundMacro.bindAction(v)

		// 	}
		// 	lists.boundSAList.Unselect(id)
		// }
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
		programItemsListWidget := *widget.NewAccordionItem(
			pb.Program.Name,
			container.NewBorder(
				lists.boundItemSearchBar,
				nil, nil, nil,
				lists.boundItemGrid,
			),
		)
		acc.Append(&programItemsListWidget)
	}
}
