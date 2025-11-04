package binders

import (
	"Squire/internal/config"
	"Squire/internal/models"
	"Squire/internal/models/coordinates"
	"Squire/internal/models/repositories"
	"Squire/ui"
	"log"
	"strconv"
	"strings"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/container"
	"fyne.io/fyne/v2/dialog"
	"fyne.io/fyne/v2/widget"
)

func SetEditorUi() {
	setEditorLists()
	setEditorButtons()
	ui.GetUi().EditorUi.ProgramSelector.SetOptions(repositories.ProgramRepo().GetAllAsStringSlice())
}

func setEditorLists() {
	setAccordionPointsLists(
		ui.GetUi().EditorUi.EditorTabs.
			PointsTab.Content.(*container.Split).Leading.(*fyne.Container).Objects[0].(*widget.Accordion),
	)
	setAccordionSearchAreasLists(
		ui.GetUi().EditorUi.EditorTabs.
			SearchAreasTab.Content.(*container.Split).Leading.(*fyne.Container).Objects[0].(*widget.Accordion),
	)
	setAccordionItemsLists(
		ui.GetUi().EditorUi.EditorTabs.
			ItemsTab.Content.(*container.Split).Leading.(*fyne.Container).Objects[0].(*widget.Accordion),
	)
}

func setEditorButtons() {
	ui.GetUi().EditorTabs.PointsTab.Widgets["Form"].(*widget.Form).OnSubmit = func() {
		w := ui.GetUi().EditorTabs.PointsTab.Widgets
		n := w["Name"].(*widget.Entry).Text
		x, _ := strconv.Atoi(w["X"].(*widget.Entry).Text)
		y, _ := strconv.Atoi(w["Y"].(*widget.Entry).Text)
		if si, ok := ui.GetUi().EditorTabs.PointsTab.SelectedItem.(*coordinates.Point); ok {
			p := ui.GetUi().ProgramSelector.Text
			v := si
			repositories.ProgramRepo().Get(p).Coordinates[config.MainMonitorSizeString].DeletePoint(si.Name)
			v.Name = n
			v.X = x
			v.Y = y
			repositories.ProgramRepo().Get(p).Coordinates[config.MainMonitorSizeString].SetPoint(v)
			// repositories.ProgramRepo().Get(ui.GetUi().ProgramSelector.Text).
			repositories.ProgramRepo().Set(p, repositories.ProgramRepo().Get(p))
			repositories.ProgramRepo().EncodeAll()
			w[p+"-searchbar"].(*widget.Entry).SetText(v.Name)
			// log.Println(repositories.ProgramRepo().Get(ui.GetUi().ProgramSelector.Text).Coordinates[config.MainMonitorSizeString].Points)
			// ui.GetUi().EditorTabs.PointsTab.Widgets["Points"] = &widget.Accordion{}
			// ui.GetUi().ActionTabs.PointsAccordion = &widget.Accordion{}
			// setAccordionPointsLists(ui.GetUi().EditorTabs.PointsTab.Widgets["Points"].(*widget.Accordion))
			// setAccordionPointsLists(ui.GetUi().ActionTabs.PointsAccordion)
			// ui.GetUi().EditorTabs.PointsTab.Widgets["Points"].Refresh()
			// ui.GetUi().ActionTabs.PointsAccordion.Refresh()
			// ui.GetUi().EditorUi.EditorTabs.
			// 	PointsTab.Content.(*container.Split).Leading.(*fyne.Container).Objects[0].(*widget.Accordion).Items[0].Detail.(*fyne.Container).Objects[0].(*widget.List).Refresh()
			// ui.GetUi().EditorUi.EditorTabs.
			// 	PointsTab.Content.(*container.Split).Leading.(*fyne.Container).Objects[0].(*widget.Accordion).Items[1].Detail.(*fyne.Container).Objects[0].(*widget.List).Refresh()
		}
		// si := &ui.GetUi().EditorTabs.PointsTab.SelectedItem.(coordinates.Point)
	}
	ui.GetUi().EditorUi.AddButton.OnTapped = func() {
		program := ui.GetUi().EditorUi.ProgramSelector.Text

		getProgram := func() *models.Program {
			pro := repositories.ProgramRepo().Get(program)
			if pro.Name == "" {
				log.Println("editor binder: new progam created")
				pro = repositories.ProgramRepo().New()
				pro.Name = ui.GetUi().ProgramSelector.Text
				repositories.ProgramRepo().Set(pro.Name, pro)
				setEditorLists()
			}
			return pro
		}

		switch ui.GetUi().EditorUi.EditorTabs.Selected().Text {
		case "Items":
			n := ui.GetUi().EditorTabs.ItemsTab.Widgets["Name"].(*widget.Entry).Text
			x, _ := strconv.Atoi(ui.GetUi().EditorTabs.ItemsTab.Widgets["GridSizeX"].(*widget.Entry).Text)
			y, _ := strconv.Atoi(ui.GetUi().EditorTabs.ItemsTab.Widgets["GridSizeY"].(*widget.Entry).Text)
			sm, _ := strconv.Atoi(ui.GetUi().EditorTabs.ItemsTab.Widgets["StackMax"].(*widget.Entry).Text)
			pro := getProgram()
			item, _ := pro.GetItem(n)
			item.GridSize[0] = x
			item.GridSize[1] = y
			item.StackMax = sm
			// item.Tags =
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
			v, err := pro.Coordinates[config.MainMonitorSizeString].AddPoint(p)
			if err != nil {
				dialog.ShowError(err, ui.GetUi().Window)
				return
			}
			ui.GetUi().EditorTabs.PointsTab.Widgets[program+"-searchbar"].(*widget.Entry).SetText(v.Name)
			ui.GetUi().EditorTabs.PointsTab.Widgets["Name"].(*widget.Entry).SetText(v.Name)
			ui.GetUi().EditorTabs.PointsTab.Widgets[strings.ToLower(program+"-points")].(*widget.List).Select(0)
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
			v, err := pro.Coordinates[config.MainMonitorSizeString].AddSearchArea(sa)
			if err != nil {
				dialog.ShowError(err, ui.GetUi().Window)
				return
			}
			ui.GetUi().EditorTabs.SearchAreasTab.Widgets[program+"-searchbar"].(*widget.Entry).SetText(v.Name)
			ui.GetUi().EditorTabs.SearchAreasTab.Widgets["Name"].(*widget.Entry).SetText(v.Name)
			ui.GetUi().EditorTabs.SearchAreasTab.Widgets[strings.ToLower(program+"-searchareas")].(*widget.List).Select(0)

		}
		repositories.ProgramRepo().EncodeAll()

	}
	ui.GetUi().EditorUi.RemoveButton.OnTapped = func() {
		program := ui.GetUi().EditorUi.ProgramSelector.Text
		switch ui.GetUi().EditorUi.EditorTabs.Selected().Text {
		case "Items":

		case "Points":
			repositories.ProgramRepo().Get(program).Coordinates[config.MainMonitorSizeString].DeletePoint(ui.GetUi().EditorTabs.PointsTab.SelectedItem.(*coordinates.Point).Name)
			ui.GetUi().EditorTabs.PointsTab.SelectedItem = &coordinates.Point{}
			text := ui.GetUi().EditorTabs.PointsTab.Widgets[program+"-searchbar"].(*widget.Entry).Text
			ui.GetUi().EditorTabs.PointsTab.Widgets[program+"-searchbar"].(*widget.Entry).SetText("uuid")
			ui.GetUi().EditorTabs.PointsTab.Widgets[program+"-searchbar"].(*widget.Entry).SetText(text)
			ui.GetUi().EditorTabs.PointsTab.Widgets[strings.ToLower(program+"-points")].(*widget.List).UnselectAll()

			ui.GetUi().EditorTabs.PointsTab.Widgets[strings.ToLower(program+"-points")].(*widget.List).Select(0)
		case "Search Areas":

		}
	}
}
