package binders

import (
	"Sqyre/internal/config"
	"Sqyre/internal/models"
	"Sqyre/internal/models/actions"
	"Sqyre/internal/models/repositories"
	"Sqyre/internal/services"
	"Sqyre/ui"
	"Sqyre/ui/custom_widgets"
	"fmt"
	"log"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/widget"
	"github.com/lithammer/fuzzysearch/fuzzy"
)

func setSearchAreaWidgets(sa models.SearchArea) {
	st := ui.GetUi().EditorTabs.SearchAreasTab.Widgets
	st["Name"].(*widget.Entry).SetText(sa.Name)
	custom_widgets.SetEntryText(st["LeftX"], fmt.Sprintf("%v", sa.LeftX))
	custom_widgets.SetEntryText(st["TopY"], fmt.Sprintf("%v", sa.TopY))
	custom_widgets.SetEntryText(st["RightX"], fmt.Sprintf("%v", sa.RightX))
	custom_widgets.SetEntryText(st["BottomY"], fmt.Sprintf("%v", sa.BottomY))
}

func setPointWidgets(p models.Point) {
	pt := ui.GetUi().EditorTabs.PointsTab
	pt.Widgets["Name"].(*widget.Entry).SetText(p.Name)
	custom_widgets.SetEntryText(pt.Widgets["X"], fmt.Sprintf("%v", p.X))
	custom_widgets.SetEntryText(pt.Widgets["Y"], fmt.Sprintf("%v", p.Y))
	func() {
		defer func() {
			if r := recover(); r != nil {
				services.LogPanicToFile(r, "Point: Preview update (point: "+p.Name+")")
			}
		}()
		ui.GetUi().UpdatePointPreview(&p)
	}()
}

func setAccordionSearchAreasLists(acc *widget.Accordion) {
	acc.Items = []*widget.AccordionItem{}
	et := ui.GetUi().EditorTabs.SearchAreasTab
	filterText := ""
	if sb, ok := et.Widgets["searchbar"].(*widget.Entry); ok {
		filterText = sb.Text
		sb.OnChanged = func(string) { setAccordionSearchAreasLists(acc) }
	}

	for _, p := range repositories.ProgramRepo().GetAll() {
		defaultList := p.SearchAreaRepo(config.MainMonitorSizeString).GetAllKeys()
		filtered := defaultList
		if filterText != "" {
			filtered = []string{}
			for _, i := range defaultList {
				if fuzzy.MatchFold(filterText, i) {
					filtered = append(filtered, i)
				}
			}
		}
		// Show program if search is empty, or program name matches, or any search area name matches
		if filterText != "" && !fuzzy.MatchFold(filterText, p.Name) && len(filtered) == 0 {
			continue
		}

		lists := struct {
			searchareas *widget.List
			filtered    []string
		}{filtered: filtered}

		lists.searchareas = widget.NewList(
			func() int { return len(lists.filtered) },
			func() fyne.CanvasObject { return widget.NewLabel("template") },
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
			func() {
				defer func() {
					if r := recover(); r != nil {
						services.LogPanicToFile(r, "SearchArea: Preview update (area: "+sa.Name+")")
					}
				}()
				ui.GetUi().UpdateSearchAreaPreview(sa)
			}()
			markSearchAreasClean()
		}

		programSAListWidget := *widget.NewAccordionItem(
			p.Name,
			lists.searchareas,
		)
		ui.GetUi().EditorTabs.SearchAreasTab.Widgets[p.Name+"-list"] = lists.searchareas
		acc.Append(&programSAListWidget)
	}
}

func setAccordionPointsLists(acc *widget.Accordion) {
	acc.Items = []*widget.AccordionItem{}
	et := ui.GetUi().EditorTabs.PointsTab
	filterText := ""
	if sb, ok := et.Widgets["searchbar"].(*widget.Entry); ok {
		filterText = sb.Text
		sb.OnChanged = func(string) { setAccordionPointsLists(acc) }
	}

	for _, p := range repositories.ProgramRepo().GetAll() {
		defaultList := p.PointRepo(config.MainMonitorSizeString).GetAllKeys()
		filtered := defaultList
		if filterText != "" {
			filtered = []string{}
			for _, i := range defaultList {
				if fuzzy.MatchFold(filterText, i) {
					filtered = append(filtered, i)
				}
			}
		}
		// Show program if search is empty, or program name matches, or any point name matches
		if filterText != "" && !fuzzy.MatchFold(filterText, p.Name) && len(filtered) == 0 {
			continue
		}

		lists := struct {
			points   *widget.List
			filtered []string
		}{filtered: filtered}

		lists.points = widget.NewList(
			func() int { return len(lists.filtered) },
			func() fyne.CanvasObject { return widget.NewLabel("template") },
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
			markPointsClean()
			if st := ui.GetUi().Mui.MTabs.SelectedTab(); st != nil {
				if v, ok := st.Macro.Root.GetAction(st.SelectedNode).(*actions.Move); ok {
					v.Point = actions.Point{Name: point.Name, X: point.X, Y: point.Y}
				}
			}
		}

		programPointListWidget := *widget.NewAccordionItem(p.Name, lists.points)
		ui.GetUi().EditorTabs.PointsTab.Widgets[p.Name+"-list"] = lists.points
		acc.Append(&programPointListWidget)
	}
}
func setAccordionAutoPicSearchAreasLists(acc *widget.Accordion) {
	acc.Items = []*widget.AccordionItem{}
	et := ui.GetUi().EditorTabs.AutoPicTab
	filterText := ""
	if sb, ok := et.Widgets["searchbar"].(*widget.Entry); ok {
		filterText = sb.Text
		sb.OnChanged = func(string) { setAccordionAutoPicSearchAreasLists(acc) }
	}

	for _, p := range repositories.ProgramRepo().GetAll() {
		defaultList := p.SearchAreaRepo(config.MainMonitorSizeString).GetAllKeys()
		filtered := defaultList
		if filterText != "" {
			filtered = []string{}
			for _, i := range defaultList {
				if fuzzy.MatchFold(filterText, i) {
					filtered = append(filtered, i)
				}
			}
		}
		// Show program if search is empty, or program name matches, or any search area name matches
		if filterText != "" && !fuzzy.MatchFold(filterText, p.Name) && len(filtered) == 0 {
			continue
		}

		lists := struct {
			searchareas *widget.List
			filtered    []string
		}{filtered: filtered}

		lists.searchareas = widget.NewList(
			func() int { return len(lists.filtered) },
			func() fyne.CanvasObject { return widget.NewLabel("template") },
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
			ui.GetUi().EditorTabs.AutoPicTab.SelectedItem = sa
			atw := ui.GetUi().EditorTabs.AutoPicTab.Widgets
			if saveButton, ok := atw["saveButton"].(*widget.Button); ok {
				saveButton.Enable()
			}
			func() {
				defer func() {
					if r := recover(); r != nil {
						services.LogPanicToFile(r, "AutoPic: Preview update (area: "+sa.Name+")")
					}
				}()
				ui.GetUi().UpdateAutoPicPreview(sa)
			}()
		}

		programSAListWidget := *widget.NewAccordionItem(p.Name, lists.searchareas)
		ui.GetUi().EditorTabs.AutoPicTab.Widgets[p.Name+"-list"] = lists.searchareas
		acc.Append(&programSAListWidget)
	}
}
