package editor

import (
	"Sqyre/internal/config"
	"Sqyre/internal/models"
	"Sqyre/internal/models/actions"
	"Sqyre/internal/models/repositories"
	"Sqyre/internal/services"
	"Sqyre/ui/custom_widgets"
	"fmt"
	"log"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/widget"
)

func setSearchAreaWidgets(sa models.SearchArea) {
	st := shell().EditorTabs.SearchAreasTab.Widgets
	st["Name"].(*widget.Entry).SetText(sa.Name)
	custom_widgets.SetEntryText(st["LeftX"], fmt.Sprintf("%v", sa.LeftX))
	custom_widgets.SetEntryText(st["TopY"], fmt.Sprintf("%v", sa.TopY))
	custom_widgets.SetEntryText(st["RightX"], fmt.Sprintf("%v", sa.RightX))
	custom_widgets.SetEntryText(st["BottomY"], fmt.Sprintf("%v", sa.BottomY))
	shell().RefreshEditorActionBar()
}

func setPointWidgets(p models.Point) {
	pt := shell().EditorTabs.PointsTab
	pt.Widgets["Name"].(*widget.Entry).SetText(p.Name)
	custom_widgets.SetEntryText(pt.Widgets["X"], fmt.Sprintf("%v", p.X))
	custom_widgets.SetEntryText(pt.Widgets["Y"], fmt.Sprintf("%v", p.Y))
	func() {
		defer func() {
			if r := recover(); r != nil {
				services.LogPanicToFile(r, "Point: Preview update (point: "+p.Name+")")
			}
		}()
		shell().UpdatePointPreview(&p)
	}()
	shell().RefreshEditorActionBar()
}

func buildSearchAreasAccordionItemForProgram(p *models.Program, filterText string) *widget.AccordionItem {
	defaultList := p.SearchAreaRepo(config.MainMonitorSizeString).GetAllKeys()
	filtered := filterKeysByFuzzy(filterText, defaultList)
	sortSearchAreaKeysByDisplayName(p, filtered)
	if skipProgramAccordionRow(filterText, p.Name, filtered) {
		return nil
	}
	prog := p
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
			program, err := repositories.ProgramRepo().Get(prog.Name)
			if err != nil {
				log.Printf("Error getting program %s: %v", prog.Name, err)
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
		program, err := repositories.ProgramRepo().Get(prog.Name)
		if err != nil {
			log.Printf("Error getting program %s: %v", prog.Name, err)
			return
		}
		shell().EditorTabs.SearchAreasTab.ProgramSelector.SetSelected(program.Name)
		saName := lists.filtered[id]
		sa, err := program.SearchAreaRepo(config.MainMonitorSizeString).Get(saName)
		if err != nil {
			return
		}
		shell().EditorTabs.SearchAreasTab.SelectedItem = sa
		setSearchAreaWidgets(*sa)
		func() {
			defer func() {
				if r := recover(); r != nil {
					services.LogPanicToFile(r, "SearchArea: Preview update (area: "+sa.Name+")")
				}
			}()
			shell().UpdateSearchAreaPreview(sa)
		}()
		markSearchAreasClean()
	}

	shell().EditorTabs.SearchAreasTab.Widgets[prog.Name+"-list"] = lists.searchareas
	return widget.NewAccordionItem(fmt.Sprintf("%s (%d)", prog.Name, len(filtered)), lists.searchareas)
}

func setAccordionSearchAreasLists(acc *widget.Accordion) {
	et := shell().EditorTabs.SearchAreasTab
	filterText := ""
	if sb, ok := et.Widgets["searchbar"].(*widget.Entry); ok {
		filterText = sb.Text
		sb.OnChanged = func(string) { setAccordionSearchAreasLists(acc) }
	}
	var items []*widget.AccordionItem
	for _, p := range repositories.ProgramRepo().GetAllSortedByName() {
		if it := buildSearchAreasAccordionItemForProgram(p, filterText); it != nil {
			items = append(items, it)
		}
	}
	acc.Items = items
	acc.Refresh()
}

// syncEditorSearchAreaAccordions rebuilds Search Areas and AutoPic accordions from the repo (each tab uses its own filter bar).
func syncEditorSearchAreaAccordions() {
	et := shell().EditorTabs
	if acc, ok := et.SearchAreasTab.Widgets["Accordion"].(*widget.Accordion); ok {
		setAccordionSearchAreasLists(acc)
	}
	if acc, ok := et.AutoPicTab.Widgets["Accordion"].(*widget.Accordion); ok {
		setAccordionAutoPicSearchAreasLists(acc)
	}
}

// refreshSearchAreasAccordionProgramRow rebuilds one program row after an in-place edit (e.g. Update).
// Falls back to syncEditorSearchAreaAccordions if the row must appear, disappear, or reorder.
// The return value is true when both Search Areas and AutoPic accordions were fully rebuilt (caller can skip a separate AutoPic row refresh).
func refreshSearchAreasAccordionProgramRow(acc *widget.Accordion, programName string) bool {
	p, err := repositories.ProgramRepo().Get(programName)
	if err != nil {
		log.Printf("Error getting program %s: %v", programName, err)
		return false
	}
	et := shell().EditorTabs.SearchAreasTab
	filterText := ""
	if sb, ok := et.Widgets["searchbar"].(*widget.Entry); ok {
		filterText = sb.Text
	}
	i := accordionRowIndexForProgram(acc, programName)
	newItem := buildSearchAreasAccordionItemForProgram(p, filterText)
	if newItem == nil {
		if i >= 0 {
			syncEditorSearchAreaAccordions()
			return true
		}
		return false
	}
	if i < 0 {
		syncEditorSearchAreaAccordions()
		return true
	}
	wasOpen := acc.Items[i].Open
	newItem.Open = wasOpen
	acc.Items[i] = newItem
	acc.Refresh()
	return false
}

func buildPointsAccordionItemForProgram(p *models.Program, filterText string) *widget.AccordionItem {
	defaultList := p.PointRepo(config.MainMonitorSizeString).GetAllKeys()
	filtered := filterKeysByFuzzy(filterText, defaultList)
	sortPointKeysByDisplayName(p, filtered)
	if skipProgramAccordionRow(filterText, p.Name, filtered) {
		return nil
	}
	prog := p
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
			program, err := repositories.ProgramRepo().Get(prog.Name)
			if err != nil {
				log.Printf("Error getting program %s: %v", prog.Name, err)
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
		program, err := repositories.ProgramRepo().Get(prog.Name)
		if err != nil {
			log.Printf("Error getting program %s: %v", prog.Name, err)
			return
		}
		shell().EditorTabs.PointsTab.ProgramSelector.SetSelected(program.Name)
		pointName := lists.filtered[id]
		point, err := prog.PointRepo(config.MainMonitorSizeString).Get(pointName)
		if err != nil {
			return
		}
		shell().EditorTabs.PointsTab.SelectedItem = point
		setPointWidgets(*point)
		markPointsClean()
		if st := activeWire.MacroMTabs().SelectedTab(); st != nil {
			if v, ok := st.Macro.Root.GetAction(st.SelectedNode).(*actions.Move); ok {
				v.Point = actions.Point{Name: point.Name, X: point.X, Y: point.Y}
			}
		}
	}

	shell().EditorTabs.PointsTab.Widgets[prog.Name+"-list"] = lists.points
	return widget.NewAccordionItem(fmt.Sprintf("%s (%d)", prog.Name, len(filtered)), lists.points)
}

func setAccordionPointsLists(acc *widget.Accordion) {
	et := shell().EditorTabs.PointsTab
	filterText := ""
	if sb, ok := et.Widgets["searchbar"].(*widget.Entry); ok {
		filterText = sb.Text
		sb.OnChanged = func(string) { setAccordionPointsLists(acc) }
	}
	var items []*widget.AccordionItem
	for _, p := range repositories.ProgramRepo().GetAllSortedByName() {
		if it := buildPointsAccordionItemForProgram(p, filterText); it != nil {
			items = append(items, it)
		}
	}
	acc.Items = items
	acc.Refresh()
}

// refreshPointsAccordionProgramRow rebuilds one program row after an in-place edit (e.g. Update).
func refreshPointsAccordionProgramRow(acc *widget.Accordion, programName string) {
	p, err := repositories.ProgramRepo().Get(programName)
	if err != nil {
		log.Printf("Error getting program %s: %v", programName, err)
		return
	}
	et := shell().EditorTabs.PointsTab
	filterText := ""
	if sb, ok := et.Widgets["searchbar"].(*widget.Entry); ok {
		filterText = sb.Text
	}
	i := accordionRowIndexForProgram(acc, programName)
	newItem := buildPointsAccordionItemForProgram(p, filterText)
	if newItem == nil {
		if i >= 0 {
			setAccordionPointsLists(acc)
		}
		return
	}
	if i < 0 {
		setAccordionPointsLists(acc)
		return
	}
	wasOpen := acc.Items[i].Open
	newItem.Open = wasOpen
	acc.Items[i] = newItem
	acc.Refresh()
}

func buildAutoPicSearchAreasAccordionItemForProgram(p *models.Program, filterText string) *widget.AccordionItem {
	defaultList := p.SearchAreaRepo(config.MainMonitorSizeString).GetAllKeys()
	filtered := filterKeysByFuzzy(filterText, defaultList)
	sortSearchAreaKeysByDisplayName(p, filtered)
	if skipProgramAccordionRow(filterText, p.Name, filtered) {
		return nil
	}
	prog := p
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
			program, err := repositories.ProgramRepo().Get(prog.Name)
			if err != nil {
				log.Printf("Error getting program %s: %v", prog.Name, err)
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
		program, err := repositories.ProgramRepo().Get(prog.Name)
		if err != nil {
			log.Printf("AutoPic: Error getting program %s: %v", prog.Name, err)
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
		shell().EditorTabs.AutoPicTab.SelectedItem = sa
		atw := shell().EditorTabs.AutoPicTab.Widgets
		if saveButton, ok := atw["saveButton"].(*widget.Button); ok {
			saveButton.Enable()
		}
		func() {
			defer func() {
				if r := recover(); r != nil {
					services.LogPanicToFile(r, "AutoPic: Preview update (area: "+sa.Name+")")
				}
			}()
			shell().UpdateAutoPicPreview(sa)
		}()
	}

	shell().EditorTabs.AutoPicTab.Widgets[prog.Name+"-list"] = lists.searchareas
	return widget.NewAccordionItem(fmt.Sprintf("%s (%d)", prog.Name, len(filtered)), lists.searchareas)
}

func setAccordionAutoPicSearchAreasLists(acc *widget.Accordion) {
	et := shell().EditorTabs.AutoPicTab
	filterText := ""
	if sb, ok := et.Widgets["searchbar"].(*widget.Entry); ok {
		filterText = sb.Text
		sb.OnChanged = func(string) { setAccordionAutoPicSearchAreasLists(acc) }
	}
	var items []*widget.AccordionItem
	for _, p := range repositories.ProgramRepo().GetAllSortedByName() {
		if it := buildAutoPicSearchAreasAccordionItemForProgram(p, filterText); it != nil {
			items = append(items, it)
		}
	}
	acc.Items = items
	acc.Refresh()
}

// refreshAutoPicSearchAreasAccordionProgramRow rebuilds one program row on the AutoPic tab (uses AutoPic filter bar).
func refreshAutoPicSearchAreasAccordionProgramRow(acc *widget.Accordion, programName string) {
	p, err := repositories.ProgramRepo().Get(programName)
	if err != nil {
		log.Printf("Error getting program %s: %v", programName, err)
		return
	}
	et := shell().EditorTabs.AutoPicTab
	filterText := ""
	if sb, ok := et.Widgets["searchbar"].(*widget.Entry); ok {
		filterText = sb.Text
	}
	i := accordionRowIndexForProgram(acc, programName)
	newItem := buildAutoPicSearchAreasAccordionItemForProgram(p, filterText)
	if newItem == nil {
		if i >= 0 {
			setAccordionAutoPicSearchAreasLists(acc)
		}
		return
	}
	if i < 0 {
		setAccordionAutoPicSearchAreasLists(acc)
		return
	}
	wasOpen := acc.Items[i].Open
	newItem.Open = wasOpen
	acc.Items[i] = newItem
	acc.Refresh()
}

// refreshEditorSearchAreaAccordionsForProgram updates Search Areas and AutoPic rows for one program (e.g. after Update).
func refreshEditorSearchAreaAccordionsForProgram(programName string) {
	et := shell().EditorTabs
	if acc, ok := et.SearchAreasTab.Widgets["Accordion"].(*widget.Accordion); ok {
		if refreshSearchAreasAccordionProgramRow(acc, programName) {
			return
		}
	}
	if acc, ok := et.AutoPicTab.Widgets["Accordion"].(*widget.Accordion); ok {
		refreshAutoPicSearchAreasAccordionProgramRow(acc, programName)
	}
}
