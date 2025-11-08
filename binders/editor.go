package binders

import (
	"Squire/internal/config"
	"Squire/internal/models"
	"Squire/internal/models/coordinates"
	"Squire/internal/models/repositories"
	"Squire/ui"
	"errors"
	"log"
	"strconv"
	"strings"

	"fyne.io/fyne/v2/dialog"
	"fyne.io/fyne/v2/widget"
)

func SetEditorUi() {
	setEditorLists()
	setEditorForms()
	setEditorButtons()
	ui.GetUi().EditorUi.ProgramSelector.SetOptions(repositories.ProgramRepo().GetAllKeys())
}

func setEditorLists() {
	et := ui.GetUi().EditorTabs
	setProgramList(
		et.ProgramsTab.Widgets["list"].(*widget.List),
	)
	setAccordionItemsLists(
		et.ItemsTab.Widgets["Accordion"].(*widget.Accordion),
	)
	setAccordionPointsLists(
		et.PointsTab.Widgets["Accordion"].(*widget.Accordion),
	)
	setAccordionSearchAreasLists(
		et.SearchAreasTab.Widgets["Accordion"].(*widget.Accordion),
	)
	et.ProgramsTab.SelectedItem = &models.Program{}
	et.ItemsTab.SelectedItem = &models.Item{}
	et.PointsTab.SelectedItem = &coordinates.Point{}
	et.SearchAreasTab.SelectedItem = &coordinates.SearchArea{}
}

func setEditorForms() {
	et := ui.GetUi().EditorTabs
	et.ProgramsTab.Widgets["Form"].(*widget.Form).OnSubmit = func() {
		w := et.ProgramsTab.Widgets
		n := w["Name"].(*widget.Entry).Text
		if si, ok := et.ProgramsTab.SelectedItem.(*models.Program); ok {
			p := ui.GetUi().ProgramSelector.Text
			if err := repositories.ProgramRepo().Delete(p); err != nil {
				log.Printf("Error deleting program %s: %v", p, err)
			}
			si.Name = n
			// pro, err := repositories.ProgramRepo().Get(p)
			// if err != nil || pro.Name == "" {
			if err := repositories.ProgramRepo().Set(si.Name, si); err != nil {
				log.Printf("Error setting program %s: %v", p, err)
				return
			}
			// }
			w["searchbar"].(*widget.Entry).SetText(si.Name)
		}
	}
	et.ItemsTab.Widgets["Form"].(*widget.Form).OnSubmit = func() {
		w := et.ItemsTab.Widgets
		n := w["Name"].(*widget.Entry).Text
		x, _ := strconv.Atoi(w["Cols"].(*widget.Entry).Text)
		y, _ := strconv.Atoi(w["Rows"].(*widget.Entry).Text)
		sm, _ := strconv.Atoi(w["StackMax"].(*widget.Entry).Text)
		// tags, _ := strconv.Atoi(w["Tags"].(*widget.Entry).Text)
		if si, ok := et.ItemsTab.SelectedItem.(*models.Item); ok {
			p := ui.GetUi().ProgramSelector.Text
			program, err := repositories.ProgramRepo().Get(p)
			if err != nil {
				log.Printf("Error getting program %s: %v", p, err)
				return
			}
			v := si
			if err := program.ItemRepo().Delete(si.Name); err != nil {
				log.Printf("Error deleting item %s: %v", si.Name, err)
			}
			v.Name = n
			v.GridSize = [2]int{x, y}
			v.StackMax = sm
			// v.Tags = tags
			if err := program.ItemRepo().Set(v.Name, v); err != nil {
				log.Printf("Error setting item %s: %v", v.Name, err)
				return
			}
			if err := repositories.ProgramRepo().Set(program.Name, program); err != nil {
				log.Printf("Error saving program %s: %v", p, err)
				return
			}
			w[p+"-searchbar"].(*widget.Entry).SetText(v.Name)
		}
		// si := &ui.GetUi().EditorTabs.PointsTab.SelectedItem.(coordinates.Point)
	}
	et.PointsTab.Widgets["Form"].(*widget.Form).OnSubmit = func() {
		w := et.PointsTab.Widgets
		n := w["Name"].(*widget.Entry).Text
		x, _ := strconv.Atoi(w["X"].(*widget.Entry).Text)
		y, _ := strconv.Atoi(w["Y"].(*widget.Entry).Text)
		if si, ok := et.PointsTab.SelectedItem.(*coordinates.Point); ok {
			p := ui.GetUi().ProgramSelector.Text
			program, err := repositories.ProgramRepo().Get(p)
			if err != nil {
				log.Printf("Error getting program %s: %v", p, err)
				return
			}
			v := si
			program.Coordinates[config.MainMonitorSizeString].DeletePoint(si.Name)
			v.Name = n
			v.X = x
			v.Y = y
			program.Coordinates[config.MainMonitorSizeString].SetPoint(v)
			if err := repositories.ProgramRepo().Set(program.Name, program); err != nil {
				log.Printf("Error saving program %s: %v", p, err)
				return
			}
			w[p+"-searchbar"].(*widget.Entry).SetText(v.Name)
		}
		// si := &ui.GetUi().EditorTabs.PointsTab.SelectedItem.(coordinates.Point)
	}
	et.SearchAreasTab.Widgets["Form"].(*widget.Form).OnSubmit = func() {
		w := et.SearchAreasTab.Widgets
		n := w["Name"].(*widget.Entry).Text
		lx, _ := strconv.Atoi(w["LeftX"].(*widget.Entry).Text)
		ty, _ := strconv.Atoi(w["TopY"].(*widget.Entry).Text)
		rx, _ := strconv.Atoi(w["RightX"].(*widget.Entry).Text)
		by, _ := strconv.Atoi(w["BottomY"].(*widget.Entry).Text)
		if si, ok := et.SearchAreasTab.SelectedItem.(*coordinates.SearchArea); ok {
			p := ui.GetUi().ProgramSelector.Text
			program, err := repositories.ProgramRepo().Get(p)
			if err != nil {
				log.Printf("Error getting program %s: %v", p, err)
				return
			}
			v := si
			program.Coordinates[config.MainMonitorSizeString].DeleteSearchArea(si.Name)
			v.Name = n
			v.LeftX = lx
			v.TopY = ty
			v.RightX = rx
			v.BottomY = by
			program.Coordinates[config.MainMonitorSizeString].SetSearchArea(v)
			if err := repositories.ProgramRepo().Set(program.Name, program); err != nil {
				log.Printf("Error saving program %s: %v", p, err)
				return
			}
			w[p+"-searchbar"].(*widget.Entry).SetText(v.Name)
		}
		// si := &ui.GetUi().EditorTabs.PointsTab.SelectedItem.(coordinates.Point)
	}

}

func setEditorButtons() {
	ui.GetUi().EditorUi.AddButton.OnTapped = func() {
		program := ui.GetUi().EditorUi.ProgramSelector.Text

		getProgram := func() *models.Program {
			pro, err := repositories.ProgramRepo().Get(program)
			if err != nil {
				log.Println("editor binder: new program created")
				pro = repositories.ProgramRepo().New()
				pro.Name = ui.GetUi().ProgramSelector.Text
				if err := repositories.ProgramRepo().Set(pro.Name, pro); err != nil {
					log.Printf("Error creating program %s: %v", pro.Name, err)
					return nil
				}
				setEditorLists()
			}
			return pro
		}

		switch ui.GetUi().EditorUi.EditorTabs.Selected().Text {
		case "Items":
			n := ui.GetUi().EditorTabs.ItemsTab.Widgets["Name"].(*widget.Entry).Text
			x, _ := strconv.Atoi(ui.GetUi().EditorTabs.ItemsTab.Widgets["Cols"].(*widget.Entry).Text)
			y, _ := strconv.Atoi(ui.GetUi().EditorTabs.ItemsTab.Widgets["Rows"].(*widget.Entry).Text)
			sm, _ := strconv.Atoi(ui.GetUi().EditorTabs.ItemsTab.Widgets["StackMax"].(*widget.Entry).Text)
			i := models.Item{
				Name:     n,
				GridSize: [2]int{x, y},
				StackMax: sm,
			}
			pro := getProgram()
			if pro == nil {
				return
			}
			// Check if item already exists
			_, err := pro.ItemRepo().Get(i.Name)
			if err == nil {
				dialog.ShowError(errors.New("an item with that name already exists"), ui.GetUi().Window)
				return
			}
			if err := pro.ItemRepo().Set(i.Name, &i); err != nil {
				dialog.ShowError(err, ui.GetUi().Window)
				return
			}
			v := &i
			ui.GetUi().EditorTabs.ItemsTab.Widgets[strings.ToLower(program)+"-searchbar"].(*widget.Entry).SetText(v.Name)
			ui.GetUi().EditorTabs.ItemsTab.Widgets["Name"].(*widget.Entry).SetText(v.Name)
			ui.GetUi().EditorTabs.ItemsTab.Widgets[strings.ToLower(program)+"-list"].(*widget.GridWrap).Select(0)
			RefreshItemsAccordionItems()
		case "Points":
			n := ui.GetUi().EditorTabs.PointsTab.Widgets["Name"].(*widget.Entry).Text
			x, _ := strconv.Atoi(ui.GetUi().EditorTabs.PointsTab.Widgets["X"].(*widget.Entry).Text)
			y, _ := strconv.Atoi(ui.GetUi().EditorTabs.PointsTab.Widgets["Y"].(*widget.Entry).Text)
			p := coordinates.Point{
				Name: n,
				X:    x,
				Y:    y,
			}
			pro := getProgram()
			if pro == nil {
				return
			}
			v, err := pro.Coordinates[config.MainMonitorSizeString].AddPoint(p)
			if err != nil {
				dialog.ShowError(err, ui.GetUi().Window)
				return
			}
			ui.GetUi().EditorTabs.PointsTab.Widgets[strings.ToLower(program)+"-searchbar"].(*widget.Entry).SetText(v.Name)
			ui.GetUi().EditorTabs.PointsTab.Widgets["Name"].(*widget.Entry).SetText(v.Name)
			ui.GetUi().EditorTabs.PointsTab.Widgets[strings.ToLower(program)+"-list"].(*widget.List).Select(0)
		case "Search Areas":
			n := ui.GetUi().EditorTabs.SearchAreasTab.Widgets["Name"].(*widget.Entry).Text
			lx, _ := strconv.Atoi(ui.GetUi().EditorTabs.SearchAreasTab.Widgets["LeftX"].(*widget.Entry).Text)
			ty, _ := strconv.Atoi(ui.GetUi().EditorTabs.SearchAreasTab.Widgets["TopY"].(*widget.Entry).Text)
			rx, _ := strconv.Atoi(ui.GetUi().EditorTabs.SearchAreasTab.Widgets["RightX"].(*widget.Entry).Text)
			by, _ := strconv.Atoi(ui.GetUi().EditorTabs.SearchAreasTab.Widgets["BottomY"].(*widget.Entry).Text)
			sa := coordinates.SearchArea{
				Name:    n,
				LeftX:   lx,
				TopY:    ty,
				RightX:  rx,
				BottomY: by,
			}
			pro := getProgram()
			if pro == nil {
				return
			}
			v, err := pro.Coordinates[config.MainMonitorSizeString].AddSearchArea(sa)
			if err != nil {
				dialog.ShowError(err, ui.GetUi().Window)
				return
			}
			ui.GetUi().EditorTabs.SearchAreasTab.Widgets[strings.ToLower(program)+"-searchbar"].(*widget.Entry).SetText(v.Name)
			ui.GetUi().EditorTabs.SearchAreasTab.Widgets["Name"].(*widget.Entry).SetText(v.Name)
			ui.GetUi().EditorTabs.SearchAreasTab.Widgets[strings.ToLower(program)+"-list"].(*widget.List).Select(0)

		}

	}
	ui.GetUi().EditorUi.RemoveButton.OnTapped = func() {
		program := ui.GetUi().EditorUi.ProgramSelector.Text
		et := ui.GetUi().EditorTabs
		prot := et.ProgramsTab
		it := et.ItemsTab
		pt := et.PointsTab
		sat := et.SearchAreasTab
		switch ui.GetUi().EditorUi.EditorTabs.Selected().Text {
		case "Programs":
			if err := repositories.ProgramRepo().Delete(prot.SelectedItem.(*models.Program).Name); err != nil {
				log.Printf("Error deleting program: %v", err)
			}
			prot.SelectedItem = &models.Program{}
			text := prot.Widgets["searchbar"].(*widget.Entry).Text
			prot.Widgets["searchbar"].(*widget.Entry).SetText("uuid")
			prot.Widgets["searchbar"].(*widget.Entry).SetText(text)
			prot.Widgets["list"].(*widget.List).UnselectAll()

			prot.Widgets["list"].(*widget.List).Select(0)
		case "Items":
			prog, err := repositories.ProgramRepo().Get(program)
			if err != nil {
				log.Printf("Error getting program %s: %v", program, err)
				return
			}
			if err := prog.ItemRepo().Delete(it.SelectedItem.(*models.Item).Name); err != nil {
				log.Printf("Error deleting item %s: %v", it.SelectedItem.(*models.Item).Name, err)
			}
			it.SelectedItem = &models.Item{}
			text := it.Widgets[program+"-searchbar"].(*widget.Entry).Text
			it.Widgets[program+"-searchbar"].(*widget.Entry).SetText("uuid")
			it.Widgets[program+"-searchbar"].(*widget.Entry).SetText(text)
			it.Widgets[strings.ToLower(program)+"-list"].(*widget.GridWrap).UnselectAll()

			it.Widgets[strings.ToLower(program)+"-list"].(*widget.GridWrap).Select(0)
		case "Points":
			prog, err := repositories.ProgramRepo().Get(program)
			if err != nil {
				log.Printf("Error getting program %s: %v", program, err)
				return
			}
			prog.Coordinates[config.MainMonitorSizeString].DeletePoint(pt.SelectedItem.(*coordinates.Point).Name)
			pt.SelectedItem = &coordinates.Point{}
			text := pt.Widgets[program+"-searchbar"].(*widget.Entry).Text
			pt.Widgets[program+"-searchbar"].(*widget.Entry).SetText("uuid")
			pt.Widgets[program+"-searchbar"].(*widget.Entry).SetText(text)
			pt.Widgets[strings.ToLower(program)+"-list"].(*widget.List).UnselectAll()

			pt.Widgets[strings.ToLower(program)+"-list"].(*widget.List).Select(0)
		case "Search Areas":
			prog, err := repositories.ProgramRepo().Get(program)
			if err != nil {
				log.Printf("Error getting program %s: %v", program, err)
				return
			}
			prog.Coordinates[config.MainMonitorSizeString].DeleteSearchArea(sat.SelectedItem.(*coordinates.SearchArea).Name)
			sat.SelectedItem = &coordinates.SearchArea{}
			text := sat.Widgets[program+"-searchbar"].(*widget.Entry).Text
			sat.Widgets[program+"-searchbar"].(*widget.Entry).SetText("uuid")
			sat.Widgets[program+"-searchbar"].(*widget.Entry).SetText(text)
			sat.Widgets[strings.ToLower(program)+"-list"].(*widget.List).UnselectAll()

			sat.Widgets[strings.ToLower(program)+"-list"].(*widget.List).Select(0)
		}
	}
}
