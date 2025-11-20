package binders

import (
	"Squire/internal/config"
	"Squire/internal/models"
	"Squire/internal/models/repositories"
	"Squire/internal/services"
	"Squire/ui"
	"Squire/ui/custom_widgets"
	"errors"
	"log"
	"os"
	"strconv"

	"fyne.io/fyne/v2/dialog"
	"fyne.io/fyne/v2/widget"
)

func SetEditorUi() {
	setEditorLists()
	setEditorForms()
	setEditorButtons()
	updateProgramSelectorOptions()
}

// updateProgramSelectorOptions refreshes the program selector with current programs
func updateProgramSelectorOptions() {
	ui.GetUi().EditorUi.ProgramSelector.SetOptions(repositories.ProgramRepo().GetAllKeys())
}

// refreshAllProgramRelatedUI refreshes all accordions and program list when programs are modified
func refreshAllProgramRelatedUI() {
	// Refresh program list
	et := ui.GetUi().EditorTabs
	if programList, ok := et.ProgramsTab.Widgets["list"].(*widget.List); ok {
		setProgramList(programList)
	}

	// Refresh editor tab accordions
	if accordion, ok := et.ItemsTab.Widgets["Accordion"].(*widget.Accordion); ok {
		setAccordionItemsLists(accordion)
	}
	if accordion, ok := et.PointsTab.Widgets["Accordion"].(*widget.Accordion); ok {
		setAccordionPointsLists(accordion)
	}
	if accordion, ok := et.SearchAreasTab.Widgets["Accordion"].(*widget.Accordion); ok {
		setAccordionSearchAreasLists(accordion)
	}
	if accordion, ok := et.AutoPicTab.Widgets["Accordion"].(*widget.Accordion); ok {
		setAccordionAutoPicSearchAreasLists(accordion)
	}

	// Refresh action tab accordions
	ats := ui.GetUi().ActionTabs
	if ats.ImageSearchItemsAccordion != nil {
		setAccordionItemsLists(ats.ImageSearchItemsAccordion)
	}
	if ats.PointsAccordion != nil {
		setAccordionPointsLists(ats.PointsAccordion)
	}
	if ats.ImageSearchSAAccordion != nil {
		setAccordionSearchAreasLists(ats.ImageSearchSAAccordion)
	}
	if ats.OcrSAAccordion != nil {
		setAccordionSearchAreasLists(ats.OcrSAAccordion)
	}
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
	setAccordionAutoPicSearchAreasLists(
		et.AutoPicTab.Widgets["Accordion"].(*widget.Accordion),
	)
	et.ProgramsTab.SelectedItem = &models.Program{}
	et.ItemsTab.SelectedItem = &models.Item{}
	et.PointsTab.SelectedItem = &models.Point{}
	et.SearchAreasTab.SelectedItem = &models.SearchArea{}
	et.AutoPicTab.SelectedItem = &models.SearchArea{}
}

func setEditorForms() {
	et := ui.GetUi().EditorTabs
	et.ProgramsTab.Widgets["Form"].(*widget.Form).OnSubmit = func() {
		w := et.ProgramsTab.Widgets
		n := w["Name"].(*widget.Entry).Text
		if si, ok := et.ProgramsTab.SelectedItem.(*models.Program); ok {
			v := si
			// p := ui.GetUi().ProgramSelector.Text
			if err := repositories.ProgramRepo().Delete(si.Name); err != nil {
				log.Printf("Error deleting program %s: %v", si.Name, err)
			}
			v.Name = n
			if err := repositories.ProgramRepo().Set(v.Name, v); err != nil {
				log.Printf("Error setting program %s: %v", v.Name, err)
				return
			}

			// Update all UI components after renaming program
			refreshAllProgramRelatedUI()
			updateProgramSelectorOptions()

			// }
			// w["searchbar"].(*widget.Entry).SetText(v.Name)
		}
	}
	et.ItemsTab.Widgets["Form"].(*widget.Form).OnSubmit = func() {
		w := et.ItemsTab.Widgets
		n := w["Name"].(*widget.Entry).Text
		x, _ := strconv.Atoi(w["Cols"].(*widget.Entry).Text)
		y, _ := strconv.Atoi(w["Rows"].(*widget.Entry).Text)
		sm, _ := strconv.Atoi(w["StackMax"].(*widget.Entry).Text)
		// tags, _ := strconv.Atoi(w["Tags"].(*widget.Entry).Text)
		if v, ok := et.ItemsTab.SelectedItem.(*models.Item); ok {
			p := ui.GetUi().ProgramSelector.Text
			program, err := repositories.ProgramRepo().Get(p)
			if err != nil {
				log.Printf("Error getting program %s: %v", p, err)
				return
			}

			// Capture the old name before making changes
			oldItemName := v.Name

			// Check if the name is being changed and if the new name already exists
			if v.Name != n {
				// Check if an item with the new name already exists
				_, err := program.ItemRepo().Get(n)
				if err == nil {
					dialog.ShowError(errors.New("an item with that name already exists"), ui.GetUi().Window)
					return
				}

				// Handle renaming of icon variant files when item name changes
				iconService := services.IconVariantServiceInstance()
				oldVariants, err := iconService.GetVariants(p, v.Name)
				if err == nil && len(oldVariants) > 0 {
					// Move each variant file from old name to new name
					for _, variant := range oldVariants {
						oldPath := iconService.GetVariantPath(p, v.Name, variant)
						newPath := iconService.GetVariantPath(p, n, variant)

						// Check if old file exists
						if _, err := os.Stat(oldPath); err == nil {
							// Move the file
							if err := os.Rename(oldPath, newPath); err != nil {
								log.Printf("Warning: Failed to rename variant file %s to %s: %v", oldPath, newPath, err)
							} else {
								log.Printf("Renamed variant file %s to %s", oldPath, newPath)
							}
						}
					}
				}

				// Delete the old item entry since we're changing the name
				if err := program.ItemRepo().Delete(v.Name); err != nil {
					log.Printf("Error deleting old item %s: %v", v.Name, err)
					dialog.ShowError(errors.New("failed to update item name"), ui.GetUi().Window)
					return
				}
			}

			v.Name = n
			v.GridSize = [2]int{x, y}
			v.StackMax = sm
			// v.Tags = tags

			// Save the item with the new name
			if err := program.ItemRepo().Set(v.Name, v); err != nil {
				log.Printf("Error saving item %s: %v", v.Name, err)
				dialog.ShowError(errors.New("failed to save item"), ui.GetUi().Window)
				return
			}

			if err := repositories.ProgramRepo().Set(program.Name, program); err != nil {
				log.Printf("Error saving program %s: %v", p, err)
				return
			}
			// w[p+"-searchbar"].(*widget.Entry).SetText(v.Name)

			// Refresh only the specific item that was updated
			RefreshItemInGrid(p, oldItemName, v.Name)

			// If the item name was changed, update the IconVariantEditor
			if editor, ok := w["iconVariantEditor"].(*custom_widgets.IconVariantEditor); ok {
				iconService := services.IconVariantServiceInstance()
				baseName := iconService.GetBaseItemName(v.Name)
				editor.SetProgramAndItem(p, baseName)
			}
		}
	}
	et.PointsTab.Widgets["Form"].(*widget.Form).OnSubmit = func() {
		w := et.PointsTab.Widgets
		n := w["Name"].(*widget.Entry).Text
		x, _ := strconv.Atoi(w["X"].(*widget.Entry).Text)
		y, _ := strconv.Atoi(w["Y"].(*widget.Entry).Text)
		if v, ok := et.PointsTab.SelectedItem.(*models.Point); ok {
			p := ui.GetUi().ProgramSelector.Text
			program, err := repositories.ProgramRepo().Get(p)
			if err != nil {
				log.Printf("Error getting program %s: %v", p, err)
				return
			}
			v.Name = n
			v.X = x
			v.Y = y
			if err := repositories.ProgramRepo().Set(program.Name, program); err != nil {
				log.Printf("Error saving program %s: %v", p, err)
				return
			}
			// w[p+"-searchbar"].(*widget.Entry).SetText(v.Name)
		}
	}
	et.SearchAreasTab.Widgets["Form"].(*widget.Form).OnSubmit = func() {
		w := et.SearchAreasTab.Widgets
		n := w["Name"].(*widget.Entry).Text
		lx, _ := strconv.Atoi(w["LeftX"].(*widget.Entry).Text)
		ty, _ := strconv.Atoi(w["TopY"].(*widget.Entry).Text)
		rx, _ := strconv.Atoi(w["RightX"].(*widget.Entry).Text)
		by, _ := strconv.Atoi(w["BottomY"].(*widget.Entry).Text)
		if v, ok := et.SearchAreasTab.SelectedItem.(*models.SearchArea); ok {
			p := ui.GetUi().ProgramSelector.Text
			program, err := repositories.ProgramRepo().Get(p)
			if err != nil {
				log.Printf("Error getting program %s: %v", p, err)
				return
			}
			v.Name = n
			v.LeftX = lx
			v.TopY = ty
			v.RightX = rx
			v.BottomY = by
			if err := repositories.ProgramRepo().Set(program.Name, program); err != nil {
				log.Printf("Error saving program %s: %v", p, err)
				return
			}

			// Update search area preview after form submission
			func() {
				defer func() {
					if r := recover(); r != nil {
						log.Printf("SearchArea: Preview update panic recovered after form update - %v (area: %s)", r, v.Name)
					}
				}()
				ui.GetUi().UpdateSearchAreaPreview(v)
			}()

			// w[p+"-searchbar"].(*widget.Entry).SetText(v.Name)
		}
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
		case "Programs":
			n := ui.GetUi().EditorTabs.ProgramsTab.Widgets["Name"].(*widget.Entry).Text

			// Check if item already exists
			_, err := repositories.ProgramRepo().Get(n)
			if err == nil {
				dialog.ShowError(errors.New("an item with that name already exists"), ui.GetUi().Window)
				return
			}

			newProgram := repositories.ProgramRepo().New()
			newProgram.Name = n
			if err := repositories.ProgramRepo().Set(n, newProgram); err != nil {
				dialog.ShowError(err, ui.GetUi().Window)
				return
			}

			// Update all UI components after adding new program
			refreshAllProgramRelatedUI()
			updateProgramSelectorOptions()

			// ui.GetUi().EditorTabs.ProgramsTab.Widgets["searchbar"].(*widget.Entry).SetText(n)
			ui.GetUi().EditorTabs.ProgramsTab.Widgets["Name"].(*widget.Entry).SetText(n)
			ui.GetUi().EditorTabs.ProgramsTab.Widgets["list"].(*widget.List).Select(0)
		case "Items":
			n := ui.GetUi().EditorTabs.ItemsTab.Widgets["Name"].(*widget.Entry).Text
			x, _ := strconv.Atoi(ui.GetUi().EditorTabs.ItemsTab.Widgets["Cols"].(*widget.Entry).Text)
			y, _ := strconv.Atoi(ui.GetUi().EditorTabs.ItemsTab.Widgets["Rows"].(*widget.Entry).Text)
			sm, _ := strconv.Atoi(ui.GetUi().EditorTabs.ItemsTab.Widgets["StackMax"].(*widget.Entry).Text)

			pro := getProgram()
			if pro == nil {
				return
			}
			// Check if item already exists
			_, err := pro.ItemRepo().Get(n)
			if err == nil {
				dialog.ShowError(errors.New("an item with that name already exists"), ui.GetUi().Window)
				return
			}
			// newItem := pro.ItemREpo.New()
			i := models.Item{
				Name:     n,
				GridSize: [2]int{x, y},
				StackMax: sm,
			}
			if err := pro.ItemRepo().Set(i.Name, &i); err != nil {
				dialog.ShowError(err, ui.GetUi().Window)
				return
			}
			v := &i
			ui.GetUi().EditorTabs.ItemsTab.Widgets["Name"].(*widget.Entry).SetText(v.Name)
			ui.GetUi().EditorTabs.ItemsTab.Widgets[program+"-list"].(*widget.GridWrap).Select(0)
			setItemsWidgets(i)
			RefreshItemsAccordionItems()
		case "Points":
			n := ui.GetUi().EditorTabs.PointsTab.Widgets["Name"].(*widget.Entry).Text
			x, _ := strconv.Atoi(ui.GetUi().EditorTabs.PointsTab.Widgets["X"].(*widget.Entry).Text)
			y, _ := strconv.Atoi(ui.GetUi().EditorTabs.PointsTab.Widgets["Y"].(*widget.Entry).Text)
			p := models.Point{
				Name: n,
				X:    x,
				Y:    y,
			}
			pro := getProgram()
			if pro == nil {
				return
			}
			err := pro.PointRepo(config.MainMonitorSizeString).Set(p.Name, &p)
			if err != nil {
				dialog.ShowError(err, ui.GetUi().Window)
				return
			}
			ui.GetUi().EditorTabs.PointsTab.Widgets["Name"].(*widget.Entry).SetText(p.Name)
			ui.GetUi().EditorTabs.PointsTab.Widgets[program+"-list"].(*widget.List).Select(0)
		case "Search Areas":
			n := ui.GetUi().EditorTabs.SearchAreasTab.Widgets["Name"].(*widget.Entry).Text
			lx, _ := strconv.Atoi(ui.GetUi().EditorTabs.SearchAreasTab.Widgets["LeftX"].(*widget.Entry).Text)
			ty, _ := strconv.Atoi(ui.GetUi().EditorTabs.SearchAreasTab.Widgets["TopY"].(*widget.Entry).Text)
			rx, _ := strconv.Atoi(ui.GetUi().EditorTabs.SearchAreasTab.Widgets["RightX"].(*widget.Entry).Text)
			by, _ := strconv.Atoi(ui.GetUi().EditorTabs.SearchAreasTab.Widgets["BottomY"].(*widget.Entry).Text)
			sa := models.SearchArea{
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
			err := pro.SearchAreaRepo(config.MainMonitorSizeString).Set(sa.Name, &sa)
			if err != nil {
				dialog.ShowError(err, ui.GetUi().Window)
				return
			}
			ui.GetUi().EditorTabs.SearchAreasTab.Widgets["Name"].(*widget.Entry).SetText(sa.Name)
			ui.GetUi().EditorTabs.SearchAreasTab.Widgets[program+"-list"].(*widget.List).Select(0)
			ui.GetUi().EditorTabs.SearchAreasTab.Widgets[program+"-list"].Refresh()
		}

	}
	ui.GetUi().EditorUi.RemoveButton.OnTapped = func() {
		program := ui.GetUi().EditorUi.ProgramSelector.Text
		et := ui.GetUi().EditorTabs
		prot := et.ProgramsTab
		it := et.ItemsTab
		pt := et.PointsTab
		sat := et.SearchAreasTab
		prog, err := repositories.ProgramRepo().Get(program)
		if err != nil {
			log.Printf("Error getting program %s: %v", program, err)
			return
		}
		switch ui.GetUi().EditorUi.EditorTabs.Selected().Text {
		case "Programs":
			if err := repositories.ProgramRepo().Delete(prot.SelectedItem.(*models.Program).Name); err != nil {
				log.Printf("Error deleting program: %v", err)
			}

			// Update all UI components after deleting program
			refreshAllProgramRelatedUI()
			updateProgramSelectorOptions()

			prot.SelectedItem = &models.Program{}
			// prot.SelectedItem, _ = repositories.ProgramRepo().Get("")
			text := prot.Widgets["searchbar"].(*widget.Entry).Text
			prot.Widgets["searchbar"].(*widget.Entry).SetText("uuid")
			prot.Widgets["searchbar"].(*widget.Entry).SetText(text)
			prot.Widgets["list"].(*widget.List).UnselectAll()

			prot.Widgets["list"].(*widget.List).Select(0)
		case "Items":

			if err := prog.ItemRepo().Delete(it.SelectedItem.(*models.Item).Name); err != nil {
				log.Printf("Error deleting item %s: %v", it.SelectedItem.(*models.Item).Name, err)
			}
			it.SelectedItem = &models.Item{}
			text := it.Widgets[program+"-searchbar"].(*widget.Entry).Text
			it.Widgets[program+"-searchbar"].(*widget.Entry).SetText("uuid")
			it.Widgets[program+"-searchbar"].(*widget.Entry).SetText(text)
			it.Widgets[program+"-list"].(*widget.GridWrap).UnselectAll()

			it.Widgets[program+"-list"].(*widget.GridWrap).Select(0)
		case "Points":
			prog, err := repositories.ProgramRepo().Get(program)
			if err != nil {
				log.Printf("Error getting program %s: %v", program, err)
				return
			}
			err = prog.PointRepo(config.MainMonitorSizeString).Delete(pt.SelectedItem.(*models.Point).Name)
			pt.SelectedItem = &models.Point{}
			text := pt.Widgets[program+"-searchbar"].(*widget.Entry).Text
			pt.Widgets[program+"-searchbar"].(*widget.Entry).SetText("uuid")
			pt.Widgets[program+"-searchbar"].(*widget.Entry).SetText(text)
			pt.Widgets[program+"-list"].(*widget.List).UnselectAll()

			pt.Widgets[program+"-list"].(*widget.List).Select(0)
		case "Search Areas":
			prog, err := repositories.ProgramRepo().Get(program)
			if err != nil {
				log.Printf("Error getting program %s: %v", program, err)
				return
			}
			n := sat.SelectedItem.(*models.SearchArea).Name
			err = prog.SearchAreaRepo(config.MainMonitorSizeString).Delete(n)
			if err != nil {
				log.Printf("Error deleting searcharea %s: %v", n, err)
				return
			}

			sat.SelectedItem = &models.SearchArea{}
			text := sat.Widgets[program+"-searchbar"].(*widget.Entry).Text
			sat.Widgets[program+"-searchbar"].(*widget.Entry).SetText("uuid")
			sat.Widgets[program+"-searchbar"].(*widget.Entry).SetText(text)
			sat.Widgets[program+"-list"].(*widget.List).UnselectAll()

			sat.Widgets[program+"-list"].(*widget.List).Select(0)
		}
	}

}
