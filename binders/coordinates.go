package binders

import (
	"Squire/internal/config"
	"Squire/internal/models"
	"Squire/internal/models/actions"
	"Squire/internal/models/repositories"
	"Squire/ui"
	"log"
	"strconv"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/container"
	"fyne.io/fyne/v2/widget"
	"github.com/lithammer/fuzzysearch/fuzzy"
)

func setSearchAreaWidgets(sa models.SearchArea) {
	st := ui.GetUi().EditorTabs.SearchAreasTab.Widgets
	st["Name"].(*widget.Entry).SetText(sa.Name)
	st["LeftX"].(*widget.Entry).SetText(strconv.Itoa(sa.LeftX))
	st["TopY"].(*widget.Entry).SetText(strconv.Itoa(sa.TopY))
	st["RightX"].(*widget.Entry).SetText(strconv.Itoa(sa.RightX))
	st["BottomY"].(*widget.Entry).SetText(strconv.Itoa(sa.BottomY))
}

func setPointWidgets(p models.Point) {
	pt := ui.GetUi().EditorTabs.PointsTab
	pt.Widgets["Name"].(*widget.Entry).SetText(p.Name)
	pt.Widgets["X"].(*widget.Entry).SetText(strconv.Itoa(p.X))
	pt.Widgets["Y"].(*widget.Entry).SetText(strconv.Itoa(p.Y))
	func() {
		defer func() {
			if r := recover(); r != nil {
				log.Printf("Point: Preview update panic recovered - %v (point: %s)", r, p.Name)
			}
		}()
		ui.GetUi().UpdatePointPreview(&p)
	}()
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
			filtered:    p.SearchAreaRepo(config.MainMonitorSizeString).GetAllKeys(),
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
				sa, err := program.SearchAreaRepo(config.MainMonitorSizeString).Get(name)
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

			sa, err := program.SearchAreaRepo(config.MainMonitorSizeString).Get(saName)
			if err != nil {
				return
			}
			ui.GetUi().EditorTabs.SearchAreasTab.SelectedItem = sa
			setSearchAreaWidgets(*sa)

			// Update search area preview with error handling
			func() {
				defer func() {
					if r := recover(); r != nil {
						log.Printf("SearchArea: Preview update panic recovered - %v (area: %s)", r, sa.Name)
					}
				}()
				ui.GetUi().UpdateSearchAreaPreview(sa)
			}()

			if ui.GetUi().MainUi.Visible() {
				if v, ok := ui.GetUi().Mui.MTabs.SelectedTab().Macro.Root.GetAction(ui.GetUi().Mui.MTabs.SelectedTab().SelectedNode).(*actions.ImageSearch); ok {
					if ui.GetUi().ActionTabs.AppTabs.Selected().Text == "Image" {
						v.SearchArea = actions.SearchArea{Name: sa.Name, LeftX: sa.LeftX, TopY: sa.TopY, RightX: sa.RightX, BottomY: sa.BottomY}
						bindAction(v)
					}
				}
				if v, ok := ui.GetUi().Mui.MTabs.SelectedTab().Macro.Root.GetAction(ui.GetUi().Mui.MTabs.SelectedTab().SelectedNode).(*actions.Ocr); ok {
					if ui.GetUi().ActionTabs.AppTabs.Selected().Text == "OCR" {
						v.SearchArea = actions.SearchArea{Name: sa.Name, LeftX: sa.LeftX, TopY: sa.TopY, RightX: sa.RightX, BottomY: sa.BottomY}
						bindAction(v)
					}
				}
				lists.searchareas.Unselect(id)
			}
		}
		lists.searchbar = &widget.Entry{
			PlaceHolder: "Search here",
			OnChanged: func(s string) {
				defaultList := p.SearchAreaRepo(config.MainMonitorSizeString).GetAllKeys()
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
		ui.GetUi().EditorTabs.SearchAreasTab.Widgets[p.Name+"-searchbar"] = lists.searchbar
		ui.GetUi().EditorTabs.SearchAreasTab.Widgets[p.Name+"-list"] = lists.searchareas
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
			filtered:  p.PointRepo(config.MainMonitorSizeString).GetAllKeys(),
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
				point, err := program.PointRepo(config.MainMonitorSizeString).Get(name)
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
			point, err := p.PointRepo(config.MainMonitorSizeString).Get(pointName)
			if err != nil {
				return
			}
			ui.GetUi().EditorTabs.PointsTab.SelectedItem = point
			setPointWidgets(*point)
			if v, ok := ui.GetUi().Mui.MTabs.SelectedTab().Macro.Root.GetAction(ui.GetUi().Mui.MTabs.SelectedTab().SelectedNode).(*actions.Move); ok {
				v.Point = actions.Point{Name: point.Name, X: point.X, Y: point.Y}
				bindAction(v)
			}
			if ui.GetUi().MainUi.Visible() {
				lists.points.Unselect(id)
			}
		}
		lists.searchBar = &widget.Entry{
			PlaceHolder: "Search here",
			OnChanged: func(s string) {
				defaultList := p.PointRepo(config.MainMonitorSizeString).GetAllKeys()
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
		ui.GetUi().EditorTabs.PointsTab.Widgets[p.Name+"-searchbar"] = lists.searchBar
		ui.GetUi().EditorTabs.PointsTab.Widgets[p.Name+"-list"] = lists.points
		acc.Append(&programPointListWidget)
	}
}
func setAccordionAutoPicSearchAreasLists(acc *widget.Accordion) {
	acc.Items = []*widget.AccordionItem{}
	for _, p := range repositories.ProgramRepo().GetAll() {
		lists := struct {
			searchbar   *widget.Entry
			searchareas *widget.List
			filtered    []string
		}{
			searchbar:   new(widget.Entry),
			searchareas: new(widget.List),
			filtered:    p.SearchAreaRepo(config.MainMonitorSizeString).GetAllKeys(),
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
				sa, err := program.SearchAreaRepo(config.MainMonitorSizeString).Get(name)
				if err != nil {
					return
				}
				label.SetText(sa.Name)
			},
		)

		lists.searchareas.OnSelected = func(id widget.ListItemID) {
			// Validate selection ID
			if id < 0 || id >= len(lists.filtered) {
				log.Printf("AutoPic: Invalid selection ID %d, filtered list length: %d", id, len(lists.filtered))
				return
			}

			program, err := repositories.ProgramRepo().Get(p.Name)
			if err != nil {
				log.Printf("AutoPic: Error getting program %s: %v", p.Name, err)
				return
			}

			saName := lists.filtered[id]
			if saName == "" {
				log.Printf("AutoPic: Empty search area name at index %d", id)
				return
			}

			sa, err := program.SearchAreaRepo(config.MainMonitorSizeString).Get(saName)
			if err != nil {
				log.Printf("AutoPic: Error getting search area %s: %v", saName, err)
				return
			}

			if sa == nil {
				log.Printf("AutoPic: Search area %s is nil", saName)
				return
			}

			// Set selected item for AutoPic tab
			ui.GetUi().EditorTabs.AutoPicTab.SelectedItem = sa

			// Enable save button and update preview with error handling
			atw := ui.GetUi().EditorTabs.AutoPicTab.Widgets
			if saveButton, ok := atw["saveButton"].(*widget.Button); ok {
				saveButton.Enable()
			} else {
				log.Printf("AutoPic: Save button not found or wrong type")
			}

			// Update preview with comprehensive error handling
			func() {
				defer func() {
					if r := recover(); r != nil {
						log.Printf("AutoPic: Preview update panic recovered - %v (area: %s)", r, sa.Name)
					}
				}()
				ui.GetUi().UpdateAutoPicPreview(sa)
			}()

			// Unselect after handling (same pattern as other tabs)
			if ui.GetUi().MainUi.Visible() {
				lists.searchareas.Unselect(id)
			}
		}

		lists.searchbar = &widget.Entry{
			PlaceHolder: "Search here",
			OnChanged: func(s string) {
				defaultList := p.SearchAreaRepo(config.MainMonitorSizeString).GetAllKeys()
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
		ui.GetUi().EditorTabs.AutoPicTab.Widgets[p.Name+"-searchbar"] = lists.searchbar
		ui.GetUi().EditorTabs.AutoPicTab.Widgets[p.Name+"-list"] = lists.searchareas
		acc.Append(&programSAListWidget)
	}
}
