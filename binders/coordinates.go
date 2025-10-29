package binders

import (
	"Squire/internal/config"
	"Squire/internal/models/actions"
	"Squire/internal/models/coordinates"
	"Squire/ui"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/container"
	"fyne.io/fyne/v2/data/binding"
	"fyne.io/fyne/v2/widget"
	"github.com/lithammer/fuzzysearch/fuzzy"
)

func bindSearchAreaEditorWidgets(di binding.Struct) {
	dl := binding.NewDataListener(func() {
		mt := ui.GetUi().Mui.MTabs.SelectedTab()
		fyne.Do(func() { mt.RefreshItem(mt.SelectedNode) })
	})

	ets := ui.GetUi().EditorTabs
	st := ets.SearchAreasTab.BindableWidgets

	name, _ := di.GetItem("Name")
	x1, _ := di.GetItem("LeftX")
	y1, _ := di.GetItem("TopY")
	x2, _ := di.GetItem("RightX")
	y2, _ := di.GetItem("BottomY")

	st["Name"].(*widget.Entry).Unbind()
	st["LeftX"].(*widget.Entry).Unbind()
	st["TopY"].(*widget.Entry).Unbind()
	st["RightX"].(*widget.Entry).Unbind()
	st["BottomY"].(*widget.Entry).Unbind()
	x1.RemoveListener(dl)
	y1.RemoveListener(dl)
	x2.RemoveListener(dl)
	y2.RemoveListener(dl)

	st["Name"].(*widget.Entry).Bind(name.(binding.String))
	st["LeftX"].(*widget.Entry).Bind(binding.IntToString(x1.(binding.Int)))
	st["TopY"].(*widget.Entry).Bind(binding.IntToString(y1.(binding.Int)))
	st["RightX"].(*widget.Entry).Bind(binding.IntToString(x2.(binding.Int)))
	st["BottomY"].(*widget.Entry).Bind(binding.IntToString(y2.(binding.Int)))
	x1.AddListener(dl)
	y1.AddListener(dl)
	x2.AddListener(dl)
	y2.AddListener(dl)
}

func bindPointWidgets(di binding.Struct) {
	dl := binding.NewDataListener(func() {
		mt := ui.GetUi().Mui.MTabs.SelectedTab()
		fyne.Do(func() { mt.RefreshItem(mt.SelectedNode) })
	})

	ets := ui.GetUi().EditorTabs
	pt := ets.PointsTab.BindableWidgets

	name, _ := di.GetItem("Name")
	x, _ := di.GetItem("X")
	y, _ := di.GetItem("Y")

	pt["Name"].(*widget.Entry).Unbind()
	pt["X"].(*widget.Entry).Unbind()
	pt["Y"].(*widget.Entry).Unbind()
	x.RemoveListener(dl)
	y.RemoveListener(dl)

	pt["Name"].(*widget.Entry).Bind(name.(binding.String))
	pt["X"].(*widget.Entry).Bind(binding.IntToString(x.(binding.Int)))
	pt["Y"].(*widget.Entry).Bind(binding.IntToString(y.(binding.Int)))
	x.AddListener(dl)
	y.AddListener(dl)

}

func setAccordionSearchAreasLists(acc *widget.Accordion) {
	for _, pb := range GetBoundPrograms() {
		lists := struct {
			boundSASearchBar *widget.Entry
			boundSAList      *widget.List
			filtered         []string
		}{
			boundSASearchBar: &widget.Entry{},
			boundSAList:      &widget.List{},
			filtered:         pb.Program.Coordinates[config.MainMonitorSizeString].GetSearchAreasAsStringSlice(),
		}

		lists.boundSAList = widget.NewList(
			func() int {
				return len(lists.filtered)
			},
			func() fyne.CanvasObject {
				return widget.NewLabel("template")
			},
			func(id widget.ListItemID, co fyne.CanvasObject) {
				sa := lists.filtered[id]
				boundSA := pb.SearchAreaBindings[sa]
				name, _ := boundSA.GetItem("Name")
				label := co.(*widget.Label)
				label.Bind(name.(binding.String))
			},
		)

		lists.boundSAList.OnSelected = func(id widget.ListItemID) {
			sa := lists.filtered[id]
			boundSA := pb.SearchAreaBindings[sa]
			bindSearchAreaEditorWidgets(boundSA)
			// ui.GetUi().ActionTabs.BoundSearchArea = boundSA
			if v, ok := ui.GetUi().Mui.MTabs.SelectedTab().Macro.Root.GetAction(ui.GetUi().Mui.MTabs.SelectedTab().SelectedNode).(*actions.ImageSearch); ok {
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
				bindAction(v)
			}
			lists.boundSAList.Unselect(id)
		}
		lists.boundSASearchBar = &widget.Entry{
			PlaceHolder: "Search here",
			OnChanged: func(s string) {
				// defaultList := pro.Coordinates[config.MainMonitorSizeString].Points
				defaultList := pb.Program.Coordinates[config.MainMonitorSizeString].GetSearchAreasAsStringSlice()
				defer lists.boundSAList.ScrollToTop()
				defer lists.boundSAList.Refresh()

				if s == "" {
					lists.filtered = defaultList
					return
				}
				lists.filtered = []string{}
				for _, i := range defaultList {
					if fuzzy.MatchFold(s, i) {
						lists.filtered = append(lists.filtered, i)
					}
				}
			},
		}
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

func setAccordionPointsLists(acc *widget.Accordion) {
	for _, pb := range GetBoundPrograms() {
		lists := struct {
			boundPointSearchBar *widget.Entry
			boundPointList      *widget.List
			filtered            []string
		}{
			boundPointSearchBar: &widget.Entry{},
			boundPointList:      &widget.List{},
			filtered:            pb.Program.Coordinates[config.MainMonitorSizeString].GetPointsAsStringSlice(),
		}
		lists.boundPointList = widget.NewList(
			func() int {
				return len(lists.filtered)
			},
			func() fyne.CanvasObject {
				return widget.NewLabel("template")
			},
			func(id widget.ListItemID, co fyne.CanvasObject) {
				point := lists.filtered[id]
				boundPoint := pb.PointsBindings[point]
				name, _ := boundPoint.GetItem("Name")
				label := co.(*widget.Label)
				label.Bind(name.(binding.String))
			},
		)

		lists.boundPointList.OnSelected = func(id widget.ListItemID) {
			point := lists.filtered[id]
			boundPoint := pb.PointsBindings[point]
			bindPointWidgets(boundPoint)
			ui.GetUi().ActionTabs.BoundPoint = boundPoint
			if v, ok := ui.GetUi().Mui.MTabs.SelectedTab().Macro.Root.GetAction(ui.GetUi().Mui.MTabs.SelectedTab().SelectedNode).(*actions.Move); ok {
				name, _ := boundPoint.GetValue("Name")
				x, _ := boundPoint.GetValue("X")
				y, _ := boundPoint.GetValue("Y")
				v.Point = coordinates.Point{
					Name: name.(string),
					X:    x.(int),
					Y:    y.(int),
				}
				bindAction(v)
			}
			lists.boundPointList.Unselect(id)
		}
		lists.boundPointSearchBar = &widget.Entry{
			PlaceHolder: "Search here",
			OnChanged: func(s string) {
				defaultList := pb.Program.Coordinates[config.MainMonitorSizeString].GetPointsAsStringSlice()
				defer lists.boundPointList.ScrollToTop()
				defer lists.boundPointList.Refresh()

				if s == "" {
					lists.filtered = defaultList
					return
				}
				lists.filtered = []string{}
				for _, i := range defaultList {
					if fuzzy.MatchFold(s, i) {
						lists.filtered = append(lists.filtered, i)
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
