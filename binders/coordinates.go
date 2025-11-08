package binders

import (
	"Squire/internal/config"
	"Squire/internal/models/actions"
	"Squire/internal/models/coordinates"
	"Squire/internal/models/repositories"
	"Squire/ui"
	"strconv"
	"strings"
"log"
	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/container"
	"fyne.io/fyne/v2/widget"
	"github.com/lithammer/fuzzysearch/fuzzy"
)

func setSearchAreaWidgets(sa coordinates.SearchArea) {
	st := ui.GetUi().EditorTabs.SearchAreasTab.Widgets
	st["Name"].(*widget.Entry).SetText(sa.Name)
	st["LeftX"].(*widget.Entry).SetText(strconv.Itoa(sa.LeftX))
	st["TopY"].(*widget.Entry).SetText(strconv.Itoa(sa.TopY))
	st["RightX"].(*widget.Entry).SetText(strconv.Itoa(sa.RightX))
	st["BottomY"].(*widget.Entry).SetText(strconv.Itoa(sa.BottomY))
}

func setPointWidgets(p coordinates.Point) {
	pt := ui.GetUi().EditorTabs.PointsTab
	pt.Widgets["Name"].(*widget.Entry).SetText(p.Name)
	pt.Widgets["X"].(*widget.Entry).SetText(strconv.Itoa(p.X))
	pt.Widgets["Y"].(*widget.Entry).SetText(strconv.Itoa(p.Y))
}

func setAccordionSearchAreasLists(acc *widget.Accordion) {
	acc.Items = []*widget.AccordionItem{}
	for _, p := range repositories.ProgramRepo().GetAll() {
		lists := struct {
			searchbar   *widget.Entry
			searchareas *widget.List
			filtered    []string
		}{
			searchbar:   new(widget.Entry),
			searchareas: new(widget.List),
			filtered:    p.Coordinates[config.MainMonitorSizeString].GetSearchAreasAsStringSlice(),
		}

		lists.searchareas = widget.NewList(
			func() int {
				return len(lists.filtered)
			},
			func() fyne.CanvasObject {
				return widget.NewLabel("template")
			},
			func(id widget.ListItemID, co fyne.CanvasObject) {
				name := lists.filtered[id]

				label := co.(*widget.Label)
				program, err := repositories.ProgramRepo().Get(p.Name)
				if err != nil {
					log.Printf("Error getting program %s: %v", p.Name, err)
					return
				}
				sa, err := program.Coordinates[config.MainMonitorSizeString].GetSearchArea(name)
				if err != nil {
					return
				}
				label.SetText(sa.Name)
			},
		)

		lists.searchareas.OnSelected = func(id widget.ListItemID) {
			program, err := repositories.ProgramRepo().Get(p.Name)
			if err != nil {
				log.Printf("Error getting program %s: %v", p.Name, err)
				return
			}
			ui.GetUi().ProgramSelector.SetText(program.Name)
			saName := lists.filtered[id]

			sa, err := program.Coordinates[config.MainMonitorSizeString].GetSearchArea(saName)
			if err != nil {
				return
			}
			ui.GetUi().EditorTabs.SearchAreasTab.SelectedItem = sa
			setSearchAreaWidgets(*sa)
			if ui.GetUi().MainUi.Visible() {
				if v, ok := ui.GetUi().Mui.MTabs.SelectedTab().Macro.Root.GetAction(ui.GetUi().Mui.MTabs.SelectedTab().SelectedNode).(*actions.ImageSearch); ok {
					if ui.GetUi().ActionTabs.AppTabs.Selected().Text == "Image" {
						v.SearchArea = *sa
						bindAction(v)
					}
				}
				if v, ok := ui.GetUi().Mui.MTabs.SelectedTab().Macro.Root.GetAction(ui.GetUi().Mui.MTabs.SelectedTab().SelectedNode).(*actions.Ocr); ok {
					if ui.GetUi().ActionTabs.AppTabs.Selected().Text == "OCR" {
						v.SearchArea = *sa
						bindAction(v)
					}
				}
				lists.searchareas.Unselect(id)
			}
		}
		lists.searchbar = &widget.Entry{
			PlaceHolder: "Search here",
			OnChanged: func(s string) {
				defaultList := p.Coordinates[config.MainMonitorSizeString].GetSearchAreasAsStringSlice()
				defer lists.searchareas.ScrollToTop()
				defer lists.searchareas.Refresh()

				if s == "" {
					lists.filtered = defaultList
					return
				}
				lists.filtered = []string{}
				for _, i := range defaultList {
					if fuzzy.MatchFold(s, i) {
						lists.filtered = append(lists.filtered, i)
						lists.searchareas.UnselectAll()

					}
				}
			},
		}
		programSAListWidget := *widget.NewAccordionItem(
			p.Name,
			container.NewBorder(
				lists.searchbar,
				nil, nil, nil,
				lists.searchareas,
			),
		)
		ui.GetUi().EditorTabs.SearchAreasTab.Widgets[strings.ToLower(p.Name+"-searchbar")] = lists.searchbar
		ui.GetUi().EditorTabs.SearchAreasTab.Widgets[strings.ToLower(p.Name+"-list")] = lists.searchareas
		acc.Append(&programSAListWidget)
	}
}

func setAccordionPointsLists(acc *widget.Accordion) {
	acc.Items = []*widget.AccordionItem{}
	for _, p := range repositories.ProgramRepo().GetAll() {
		lists := struct {
			searchBar *widget.Entry
			points    *widget.List
			filtered  []string
		}{
			searchBar: new(widget.Entry),
			points:    new(widget.List),
			filtered:  p.Coordinates[config.MainMonitorSizeString].GetPointsAsStringSlice(),
		}
		lists.points = widget.NewList(
			func() int {
				return len(lists.filtered)
			},
			func() fyne.CanvasObject {
				return widget.NewLabel("template")
			},
			func(id widget.ListItemID, co fyne.CanvasObject) {
				name := lists.filtered[id]
				label := co.(*widget.Label)
				program, err := repositories.ProgramRepo().Get(p.Name)
				if err != nil {
					log.Printf("Error getting program %s: %v", p.Name, err)
					return
				}
				point, err := program.Coordinates[config.MainMonitorSizeString].GetPoint(name)
				if err != nil {
					return
				}
				label.SetText(point.Name)
			},
		)

		lists.points.OnSelected = func(id widget.ListItemID) {
			program, err := repositories.ProgramRepo().Get(p.Name)
			if err != nil {
				log.Printf("Error getting program %s: %v", p.Name, err)
				return
			}
			ui.GetUi().ProgramSelector.SetText(program.Name)

			pointName := lists.filtered[id]
			point, err := program.Coordinates[config.MainMonitorSizeString].GetPoint(pointName)
			if err != nil {
				return
			}
			ui.GetUi().EditorTabs.PointsTab.SelectedItem = point
			setPointWidgets(*point)
			if v, ok := ui.GetUi().Mui.MTabs.SelectedTab().Macro.Root.GetAction(ui.GetUi().Mui.MTabs.SelectedTab().SelectedNode).(*actions.Move); ok {
				v.Point = *point
				bindAction(v)
			}
			if ui.GetUi().MainUi.Visible() {
				lists.points.Unselect(id)
			}
		}
		lists.searchBar = &widget.Entry{
			PlaceHolder: "Search here",
			OnChanged: func(s string) {
				defaultList := p.Coordinates[config.MainMonitorSizeString].GetPointsAsStringSlice()
				defer lists.points.ScrollToTop()
				defer lists.points.Refresh()

				if s == "" {
					lists.filtered = defaultList
					return
				}
				lists.filtered = []string{}
				for _, i := range defaultList {
					if fuzzy.MatchFold(s, i) {
						lists.filtered = append(lists.filtered, i)
						lists.points.UnselectAll()
					}
				}
			},
		}
		programPointListWidget := *widget.NewAccordionItem(
			p.Name,
			container.NewBorder(
				lists.searchBar,
				nil, nil, nil,
				lists.points,
			),
		)
		ui.GetUi().EditorTabs.PointsTab.Widgets[strings.ToLower(p.Name)+"-searchbar"] = lists.searchBar
		ui.GetUi().EditorTabs.PointsTab.Widgets[strings.ToLower(p.Name)+"-list"] = lists.points
		acc.Append(&programPointListWidget)
	}
}
