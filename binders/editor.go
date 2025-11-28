package binders

import (
	"Squire/internal/config"
	"Squire/internal/models"
	"Squire/internal/models/repositories"
	"Squire/internal/services"
	"Squire/ui"
	"Squire/ui/custom_widgets"
	"errors"
	"fmt"
	"io"
	"log"
	"os"
	"path/filepath"
	"strconv"
	"strings"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/container"
	"fyne.io/fyne/v2/dialog"
	"fyne.io/fyne/v2/storage"
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
	if accordion, ok := et.MasksTab.Widgets["Accordion"].(*widget.Accordion); ok {
		setAccordionMasksLists(accordion)
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
	setAccordionMasksLists(
		et.MasksTab.Widgets["Accordion"].(*widget.Accordion),
	)
	et.ProgramsTab.SelectedItem = repositories.ProgramRepo().New()
	// Note: For nested models, we need a program context to get repositories
	// These will be set to proper instances when a program is selected
	et.ItemsTab.SelectedItem = &models.Item{}
	et.PointsTab.SelectedItem = &models.Point{}
	et.SearchAreasTab.SelectedItem = &models.SearchArea{}
	et.AutoPicTab.SelectedItem = &models.SearchArea{}
	et.MasksTab.SelectedItem = nil
}

func setEditorForms() {
	et := ui.GetUi().EditorTabs
	et.ProgramsTab.Widgets["Form"].(*widget.Form).OnSubmit = func() {
		w := et.ProgramsTab.Widgets
		n := w["Name"].(*widget.Entry).Text
		if si, ok := et.ProgramsTab.SelectedItem.(*models.Program); ok {
			v := si
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
			oldkey := v.Name
			v.Name = n
			v.X = x
			v.Y = y

			if err := program.PointRepo(config.MainMonitorSizeString).Set(v.Name, v); err != nil {
				log.Printf("Error saving point %s: %v", v.Name, err)
				dialog.ShowError(errors.New("failed to save point"), ui.GetUi().Window)
				return
			}

			if oldkey != v.Name {
				if err := program.PointRepo(config.MainMonitorSizeString).Delete(oldkey); err != nil {
					log.Printf("Error deleting point %s: %v", oldkey, err)
					dialog.ShowError(errors.New("failed to delete point"), ui.GetUi().Window)
					return
				}
			}

			if err := repositories.ProgramRepo().Set(program.Name, program); err != nil {
				log.Printf("Error saving program %s: %v", p, err)
				return
			}
			t := w[program.Name+"-searchbar"].(*widget.Entry).Text
			w[program.Name+"-searchbar"].(*widget.Entry).SetText("random string of text for refreshing because poop")
			w[program.Name+"-searchbar"].(*widget.Entry).SetText(t)
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
			oldkey := v.Name
			v.Name = n
			v.LeftX = lx
			v.TopY = ty
			v.RightX = rx
			v.BottomY = by

			if err := program.SearchAreaRepo(config.MainMonitorSizeString).Set(v.Name, v); err != nil {
				log.Printf("Error saving search area %s: %v", v.Name, err)
				dialog.ShowError(errors.New("failed to save search area"), ui.GetUi().Window)
				return
			}
			if oldkey != v.Name {
				if err := program.SearchAreaRepo(config.MainMonitorSizeString).Delete(oldkey); err != nil {
					log.Printf("Error deleting search area %s: %v", oldkey, err)
					dialog.ShowError(errors.New("failed to delete search area"), ui.GetUi().Window)
					return
				}
			}

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
			t := w[program.Name+"-searchbar"].(*widget.Entry).Text
			w[program.Name+"-searchbar"].(*widget.Entry).SetText("random string of text for refreshing because poop")
			w[program.Name+"-searchbar"].(*widget.Entry).SetText(t)
		}
	}

}

func setEditorButtons() {
	// add := func(widgets []string, repo repositories.Repository[any]) {

	// }

	ui.GetUi().EditorUi.AddButton.OnTapped = func() {
		program := ui.GetUi().EditorUi.ProgramSelector.Text

		getProgram := func(pn string) *models.Program {
			pro, err := repositories.ProgramRepo().Get(pn)
			if err != nil {
				pro = repositories.ProgramRepo().New()
				pro.Name = pn
				if err := repositories.ProgramRepo().Set(pro.Name, pro); err != nil {
					dialog.ShowError(err, ui.GetUi().Window)
					return nil
				}
				log.Println("editor binder: new program created", pn)
				setEditorLists()
			}
			return pro
		}

		switch ui.GetUi().EditorUi.EditorTabs.Selected().Text {
		case "Programs":
			n := ui.GetUi().EditorTabs.ProgramsTab.Widgets["Name"].(*widget.Entry).Text
			getProgram(n)
			// Update all UI components after adding new program
			refreshAllProgramRelatedUI()
			updateProgramSelectorOptions()

			ui.GetUi().EditorTabs.ProgramsTab.Widgets["Name"].(*widget.Entry).SetText(n)
		case "Items":
			n := ui.GetUi().EditorTabs.ItemsTab.Widgets["Name"].(*widget.Entry).Text
			x, _ := strconv.Atoi(ui.GetUi().EditorTabs.ItemsTab.Widgets["Cols"].(*widget.Entry).Text)
			y, _ := strconv.Atoi(ui.GetUi().EditorTabs.ItemsTab.Widgets["Rows"].(*widget.Entry).Text)
			sm, _ := strconv.Atoi(ui.GetUi().EditorTabs.ItemsTab.Widgets["StackMax"].(*widget.Entry).Text)

			pro := getProgram(program)
			if pro == nil {
				return
			}
			// Check if item already exists
			_, err := pro.ItemRepo().Get(n)
			if err == nil {
				dialog.ShowError(errors.New("an item with that name already exists"), ui.GetUi().Window)
				return
			}
			// Create new item using repository New() function
			i := pro.ItemRepo().New()
			i.Name = n
			i.GridSize = [2]int{x, y}
			i.StackMax = sm
			if err := pro.ItemRepo().Set(i.Name, i); err != nil {
				dialog.ShowError(err, ui.GetUi().Window)
				return
			}
			v := i
			ui.GetUi().EditorTabs.ItemsTab.Widgets["Name"].(*widget.Entry).SetText(v.Name)
			t := ui.GetUi().EditorTabs.ItemsTab.Widgets[program+"-searchbar"].(*widget.Entry).Text
			ui.GetUi().EditorTabs.ItemsTab.Widgets[program+"-searchbar"].(*widget.Entry).SetText("random string of text for refreshing because poop")
			ui.GetUi().EditorTabs.ItemsTab.Widgets[program+"-searchbar"].(*widget.Entry).SetText(t)
			setItemsWidgets(*i)
			// RefreshItemsAccordionItems()
		case "Points":
			n := ui.GetUi().EditorTabs.PointsTab.Widgets["Name"].(*widget.Entry).Text
			x, _ := strconv.Atoi(ui.GetUi().EditorTabs.PointsTab.Widgets["X"].(*widget.Entry).Text)
			y, _ := strconv.Atoi(ui.GetUi().EditorTabs.PointsTab.Widgets["Y"].(*widget.Entry).Text)

			pro := getProgram(program)
			if pro == nil {
				return
			}

			// Create new point using repository New() function
			p := pro.PointRepo(config.MainMonitorSizeString).New()
			p.Name = n
			p.X = x
			p.Y = y

			err := pro.PointRepo(config.MainMonitorSizeString).Set(p.Name, p)
			if err != nil {
				dialog.ShowError(err, ui.GetUi().Window)
				return
			}
			ui.GetUi().EditorTabs.PointsTab.Widgets["Name"].(*widget.Entry).SetText(p.Name)
			t := ui.GetUi().EditorTabs.PointsTab.Widgets[program+"-searchbar"].(*widget.Entry).Text
			ui.GetUi().EditorTabs.PointsTab.Widgets[program+"-searchbar"].(*widget.Entry).SetText("random string of text for refreshing because poop")
			ui.GetUi().EditorTabs.PointsTab.Widgets[program+"-searchbar"].(*widget.Entry).SetText(t)
			// refreshAllProgramRelatedUI()
		case "Search Areas":
			n := ui.GetUi().EditorTabs.SearchAreasTab.Widgets["Name"].(*widget.Entry).Text
			lx, _ := strconv.Atoi(ui.GetUi().EditorTabs.SearchAreasTab.Widgets["LeftX"].(*widget.Entry).Text)
			ty, _ := strconv.Atoi(ui.GetUi().EditorTabs.SearchAreasTab.Widgets["TopY"].(*widget.Entry).Text)
			rx, _ := strconv.Atoi(ui.GetUi().EditorTabs.SearchAreasTab.Widgets["RightX"].(*widget.Entry).Text)
			by, _ := strconv.Atoi(ui.GetUi().EditorTabs.SearchAreasTab.Widgets["BottomY"].(*widget.Entry).Text)

			pro := getProgram(program)
			if pro == nil {
				return
			}

			sa := pro.SearchAreaRepo(config.MainMonitorSizeString).New()
			sa.Name = n
			sa.LeftX = lx
			sa.TopY = ty
			sa.RightX = rx
			sa.BottomY = by

			err := pro.SearchAreaRepo(config.MainMonitorSizeString).Set(sa.Name, sa)
			if err != nil {
				dialog.ShowError(err, ui.GetUi().Window)
				return
			}
			ui.GetUi().EditorTabs.SearchAreasTab.Widgets["Name"].(*widget.Entry).SetText(sa.Name)
			t := ui.GetUi().EditorTabs.SearchAreasTab.Widgets[program+"-searchbar"].(*widget.Entry).Text
			ui.GetUi().EditorTabs.SearchAreasTab.Widgets[program+"-searchbar"].(*widget.Entry).SetText("random string of text for refreshing because poop")
			ui.GetUi().EditorTabs.SearchAreasTab.Widgets[program+"-searchbar"].(*widget.Entry).SetText(t)
		case "Masks":
			// Show file dialog for mask upload when Masks tab is active
			fileDialog := dialog.NewFileOpen(func(reader fyne.URIReadCloser, err error) {
				if err != nil {
					dialog.ShowError(fmt.Errorf("file selection error: %v", err), ui.GetUi().Window)
					return
				}
				if reader == nil {
					return
				}
				defer reader.Close()
				
				// Handle mask upload
				handleMaskUpload(reader, program)
			}, ui.GetUi().Window)
			
			// Set file filter for supported image formats
			fileDialog.SetFilter(storage.NewExtensionFileFilter([]string{".png", ".jpg", ".jpeg"}))
			fileDialog.Show()
		}

	}
	ui.GetUi().EditorUi.RemoveButton.OnTapped = func() {
		var (
			program           = ui.GetUi().EditorUi.ProgramSelector.Text
			et                = ui.GetUi().EditorTabs
			prot, it, pt, sat = et.ProgramsTab, et.ItemsTab, et.PointsTab, et.SearchAreasTab
			prog, err         = repositories.ProgramRepo().Get(program)
		)
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

			prot.SelectedItem = repositories.ProgramRepo().New()
			// text := prot.Widgets["searchbar"].(*widget.Entry).Text
			// prot.Widgets["searchbar"].(*widget.Entry).SetText("uuid")
			// prot.Widgets["searchbar"].(*widget.Entry).SetText(text)
		case "Items":
			if err := prog.ItemRepo().Delete(it.SelectedItem.(*models.Item).Name); err != nil {
				log.Printf("Error deleting item %s: %v", it.SelectedItem.(*models.Item).Name, err)
			}
			// Create new item using repository New() function if program exists
			if prog != nil {
				it.SelectedItem = prog.ItemRepo().New()
			} else {
				it.SelectedItem = &models.Item{}
			}
			text := it.Widgets[program+"-searchbar"].(*widget.Entry).Text
			it.Widgets[program+"-searchbar"].(*widget.Entry).SetText("uuid")
			it.Widgets[program+"-searchbar"].(*widget.Entry).SetText(text)
		case "Points":
			if err := prog.PointRepo(config.MainMonitorSizeString).Delete(pt.SelectedItem.(*models.Point).Name); err != nil {
				log.Printf("Error deleting point %s: %v", pt.SelectedItem.(*models.Point).Name, err)
			}
			// Create new point using repository New() function if program exists
			if prog != nil {
				pt.SelectedItem = prog.PointRepo(config.MainMonitorSizeString).New()
			} else {
				pt.SelectedItem = &models.Point{}
			}
			text := pt.Widgets[program+"-searchbar"].(*widget.Entry).Text
			pt.Widgets[program+"-searchbar"].(*widget.Entry).SetText("uuid")
			pt.Widgets[program+"-searchbar"].(*widget.Entry).SetText(text)
		case "Search Areas":
			n := sat.SelectedItem.(*models.SearchArea).Name
			err = prog.SearchAreaRepo(config.MainMonitorSizeString).Delete(n)
			if err != nil {
				log.Printf("Error deleting searcharea %s: %v", n, err)
				return
			}

			// Create new search area using repository New() function if program exists
			if prog != nil {
				sat.SelectedItem = prog.SearchAreaRepo(config.MainMonitorSizeString).New()
			} else {
				sat.SelectedItem = &models.SearchArea{}
			}
			text := sat.Widgets[program+"-searchbar"].(*widget.Entry).Text
			sat.Widgets[program+"-searchbar"].(*widget.Entry).SetText("uuid")
			sat.Widgets[program+"-searchbar"].(*widget.Entry).SetText(text)
		case "Masks":
			// Handle mask deletion
			if selectedMask, ok := et.MasksTab.SelectedItem.(*services.MaskInfo); ok && selectedMask != nil {
				// Delete the mask file from the file system
				if err := os.Remove(selectedMask.Path); err != nil {
					log.Printf("Error deleting mask file %s: %v", selectedMask.Path, err)
					dialog.ShowError(fmt.Errorf("Failed to delete mask file '%s': %v", selectedMask.Name, err), ui.GetUi().Window)
					return
				}
				
				log.Printf("Successfully deleted mask: %s from program: %s (path: %s)", 
					selectedMask.Name, selectedMask.Program, selectedMask.Path)
				
				// Clear the selected item
				et.MasksTab.SelectedItem = nil
				
				// Clear the preview image
				maskAccordionService := services.MaskAccordionServiceInstance()
				maskAccordionService.ClearMaskPreview(et.MasksTab.PreviewImage)
				
				// Refresh the masks accordion to reflect the deletion
				refreshMasksAccordion()
				
				// Show success message
				dialog.ShowInformation("Mask Deleted", 
					fmt.Sprintf("Mask '%s' has been deleted successfully from program '%s'", 
						selectedMask.Name, selectedMask.Program), ui.GetUi().Window)
			} else {
				dialog.ShowError(errors.New("No mask selected for deletion"), ui.GetUi().Window)
			}
		}
	}

}

// setAccordionMasksLists populates the masks accordion with program-based mask lists
func setAccordionMasksLists(accordion *widget.Accordion) {
	maskAccordionService := services.MaskAccordionServiceInstance()
	
	// Get the widgets map and preview image from the masks tab
	masksTab := ui.GetUi().EditorTabs.MasksTab
	widgetsMap := masksTab.Widgets
	previewImage := masksTab.PreviewImage
	
	// Initialize the preview image with proper placeholder state
	maskAccordionService.InitializeMaskPreview(previewImage)
	
	// Set up the selection callback to update the SelectedItem
	maskAccordionService.SetSelectionCallback(func(mask *services.MaskInfo) {
		masksTab.SelectedItem = mask
	})
	
	err := maskAccordionService.PopulateMasksAccordion(accordion, widgetsMap, previewImage)
	if err != nil {
		log.Printf("Error populating masks accordion: %v", err)
		// Handle empty state and clear preview
		maskAccordionService.HandleEmptyPrograms(accordion)
		maskAccordionService.ClearMaskPreview(previewImage)
	}
}

// handleMaskUpload processes the uploaded mask file with comprehensive error handling
func handleMaskUpload(reader fyne.URIReadCloser, programName string) {
	// Validate reader
	if reader == nil {
		dialog.ShowError(errors.New("No file selected for upload"), ui.GetUi().Window)
		log.Printf("Error: Mask upload failed - no file selected")
		return
	}
	
	// Validate URI
	if reader.URI() == nil {
		dialog.ShowError(errors.New("Invalid file selection"), ui.GetUi().Window)
		log.Printf("Error: Mask upload failed - invalid URI")
		return
	}
	
	// Get the file path and validate format
	filePath := reader.URI().Path()
	if filePath == "" {
		dialog.ShowError(errors.New("Invalid file path"), ui.GetUi().Window)
		log.Printf("Error: Mask upload failed - empty file path")
		return
	}
	
	fileExt := strings.ToLower(filepath.Ext(filePath))
	
	// Validate file format using the same constants as the mask discovery service
	supportedFormats := []string{config.PNG, ".jpg", ".jpeg"}
	isSupported := false
	for _, format := range supportedFormats {
		if fileExt == format {
			isSupported = true
			break
		}
	}
	
	if !isSupported {
		dialog.ShowError(fmt.Errorf("Unsupported file format: %s\n\nSupported formats: PNG, JPG, JPEG", fileExt), ui.GetUi().Window)
		log.Printf("Error: Mask upload failed - unsupported format: %s", fileExt)
		return
	}
	
	// Validate program name
	if programName == "" {
		dialog.ShowError(errors.New("No program selected\n\nPlease select a program from the Program Selector before uploading masks"), ui.GetUi().Window)
		log.Printf("Error: Mask upload failed - no program selected")
		return
	}
	
	// Sanitize program name to prevent directory traversal
	originalProgramName := programName
	programName = filepath.Base(programName)
	if programName == "." || programName == ".." || programName == "" {
		dialog.ShowError(fmt.Errorf("Invalid program name: '%s'", originalProgramName), ui.GetUi().Window)
		log.Printf("Error: Mask upload failed - invalid program name: %s", originalProgramName)
		return
	}
	
	// Get masks directory path with error handling
	masksPath := config.GetMasksPath()
	if masksPath == "" {
		dialog.ShowError(errors.New("Failed to determine masks directory path"), ui.GetUi().Window)
		log.Printf("Error: Mask upload failed - empty masks path")
		return
	}
	
	// Create destination directory using GetMasksPath()
	programDir := filepath.Join(masksPath, programName)
	
	// Ensure the program directory is within the masks directory (security check)
	if !strings.HasPrefix(programDir, masksPath) {
		dialog.ShowError(errors.New("Security error: Invalid program directory path"), ui.GetUi().Window)
		log.Printf("Error: Mask upload failed - directory traversal attempt: %s", programDir)
		return
	}
	
	// Create program directory with enhanced error handling
	if err := os.MkdirAll(programDir, 0755); err != nil {
		var errorMsg string
		if os.IsPermission(err) {
			errorMsg = fmt.Sprintf("Insufficient permissions to create directory for program '%s'\n\nPlease check file system permissions", programName)
		} else if strings.Contains(err.Error(), "no space left") {
			errorMsg = "Insufficient disk space to create program directory"
		} else {
			errorMsg = fmt.Sprintf("Failed to create directory for program '%s': %v", programName, err)
		}
		
		dialog.ShowError(errors.New(errorMsg), ui.GetUi().Window)
		log.Printf("Error: Mask upload failed - directory creation error: %v", err)
		return
	}
	
	// Get filename and sanitize it
	fileName := filepath.Base(filePath)
	if fileName == "" || fileName == "." || fileName == ".." {
		dialog.ShowError(errors.New("Invalid file name"), ui.GetUi().Window)
		log.Printf("Error: Mask upload failed - invalid filename: %s", fileName)
		return
	}
	
	// Validate filename length and characters
	if len(fileName) > 255 {
		dialog.ShowError(errors.New("File name too long (maximum 255 characters)"), ui.GetUi().Window)
		log.Printf("Error: Mask upload failed - filename too long: %d characters", len(fileName))
		return
	}
	
	// Create destination path
	destPath := filepath.Join(programDir, fileName)
	
	// Ensure destination path is within the program directory (security check)
	if !strings.HasPrefix(destPath, programDir) {
		dialog.ShowError(errors.New("Security error: Invalid destination path"), ui.GetUi().Window)
		log.Printf("Error: Mask upload failed - path traversal attempt: %s", destPath)
		return
	}
	
	// Enhanced duplicate name detection with detailed conflict information
	maskName := strings.TrimSuffix(fileName, fileExt)
	maskDiscovery := services.MaskDiscoveryServiceInstance()
	
	conflictInfo, err := maskDiscovery.GetMaskConflictInfo(programName, maskName)
	if err != nil {
		dialog.ShowError(fmt.Errorf("Failed to check for duplicate mask names: %v", err), ui.GetUi().Window)
		log.Printf("Error: Mask upload failed - duplicate check error: %v", err)
		return
	}
	
	if conflictInfo.HasConflict {
		// Show detailed conflict resolution dialog
		showMaskConflictDialog(conflictInfo, reader, destPath, fileName, fileExt, programName)
		return
	}
	
	// Copy file to destination with enhanced error handling
	if err := copyFileWithValidation(reader, destPath, fileName, programName); err != nil {
		// Error message is already shown in copyFileWithValidation
		log.Printf("Error: Mask upload failed during file copy: %v", err)
		return
	}
	
	// Get mask name (filename without extension) for logging and display
	log.Printf("Successfully uploaded mask: %s (file: %s) to program: %s", maskName, fileName, programName)
	
	// Refresh the masks accordion to show the newly uploaded mask
	refreshMasksAccordion()
	
	// Clear any file dialog state by ensuring focus returns to main window
	ui.GetUi().Window.Canvas().Focus(nil)
	
	// Show success message with mask name (without extension)
	dialog.ShowInformation("Upload Successful", 
		fmt.Sprintf("Mask '%s' has been uploaded successfully to program '%s'", maskName, programName), ui.GetUi().Window)
}

// copyFileWithValidation copies the content from the reader to the destination file with comprehensive error handling
func copyFileWithValidation(reader fyne.URIReadCloser, destPath, fileName, programName string) error {
	// Create destination file with enhanced error handling
	destFile, err := os.Create(destPath)
	if err != nil {
		var errorMsg string
		if os.IsPermission(err) {
			errorMsg = fmt.Sprintf("Insufficient permissions to create mask file in program '%s'\n\nPlease check file system permissions", programName)
		} else if strings.Contains(err.Error(), "no space left") {
			errorMsg = "Insufficient disk space to save mask file"
		} else if strings.Contains(err.Error(), "file exists") {
			errorMsg = fmt.Sprintf("Cannot create mask file - file already exists: %s", fileName)
		} else {
			errorMsg = fmt.Sprintf("Failed to create mask file '%s': %v", fileName, err)
		}
		
		dialog.ShowError(errors.New(errorMsg), ui.GetUi().Window)
		return fmt.Errorf("failed to create destination file: %w", err)
	}
	
	// Ensure file is closed and cleaned up on error
	defer func() {
		if closeErr := destFile.Close(); closeErr != nil {
			log.Printf("Warning: failed to close destination file %s: %v", destPath, closeErr)
		}
	}()
	
	// Copy content with size limit to prevent disk space issues
	const maxFileSize = 50 * 1024 * 1024 // 50MB limit for mask files
	limitedReader := io.LimitReader(reader, maxFileSize)
	
	bytesWritten, err := io.Copy(destFile, limitedReader)
	if err != nil {
		// Clean up the partially created file on error
		if removeErr := os.Remove(destPath); removeErr != nil {
			log.Printf("Warning: failed to clean up partial file %s: %v", destPath, removeErr)
		}
		
		var errorMsg string
		if os.IsPermission(err) {
			errorMsg = fmt.Sprintf("Insufficient permissions to write mask file '%s'", fileName)
		} else if strings.Contains(err.Error(), "no space left") {
			errorMsg = "Insufficient disk space to save mask file"
		} else if strings.Contains(err.Error(), "device full") {
			errorMsg = "Storage device is full - cannot save mask file"
		} else {
			errorMsg = fmt.Sprintf("Failed to copy mask file '%s': %v", fileName, err)
		}
		
		dialog.ShowError(errors.New(errorMsg), ui.GetUi().Window)
		return fmt.Errorf("failed to copy file content: %w", err)
	}
	
	// Check if we hit the size limit
	if bytesWritten == maxFileSize {
		// Clean up the file since it might be truncated
		if removeErr := os.Remove(destPath); removeErr != nil {
			log.Printf("Warning: failed to clean up oversized file %s: %v", destPath, removeErr)
		}
		
		dialog.ShowError(fmt.Errorf("File too large\n\nThe mask file '%s' exceeds the maximum size limit of %d MB\n\nPlease use a smaller image file", 
			fileName, maxFileSize/(1024*1024)), ui.GetUi().Window)
		return fmt.Errorf("file too large (maximum size: %d MB)", maxFileSize/(1024*1024))
	}
	
	// Validate minimum file size (prevent empty files)
	if bytesWritten == 0 {
		// Clean up the empty file
		if removeErr := os.Remove(destPath); removeErr != nil {
			log.Printf("Warning: failed to clean up empty file %s: %v", destPath, removeErr)
		}
		
		dialog.ShowError(fmt.Errorf("Invalid file\n\nThe mask file '%s' is empty or corrupted\n\nPlease select a valid image file", fileName), ui.GetUi().Window)
		return fmt.Errorf("file is empty")
	}
	
	// Sync to ensure data is written to disk with error handling
	if err := destFile.Sync(); err != nil {
		log.Printf("Warning: failed to sync file to disk: %v", err)
		
		// This is not a fatal error, but we should warn the user
		if strings.Contains(err.Error(), "no space left") {
			// Clean up the file if sync failed due to disk space
			if removeErr := os.Remove(destPath); removeErr != nil {
				log.Printf("Warning: failed to clean up file after sync failure %s: %v", destPath, removeErr)
			}
			
			dialog.ShowError(errors.New("Insufficient disk space to complete mask file upload"), ui.GetUi().Window)
			return fmt.Errorf("failed to sync file to disk: %w", err)
		}
	}
	
	// Final validation - check that file was created successfully
	if info, err := os.Stat(destPath); err != nil {
		dialog.ShowError(fmt.Errorf("Upload verification failed\n\nCould not verify that mask file '%s' was saved correctly", fileName), ui.GetUi().Window)
		return fmt.Errorf("failed to verify file creation: %w", err)
	} else if info.Size() != bytesWritten {
		dialog.ShowError(fmt.Errorf("Upload verification failed\n\nMask file '%s' may be corrupted (size mismatch)", fileName), ui.GetUi().Window)
		return fmt.Errorf("file size mismatch: expected %d, got %d", bytesWritten, info.Size())
	}
	
	log.Printf("Successfully copied mask file: %s (%d bytes) to %s", fileName, bytesWritten, destPath)
	return nil
}

// showMaskConflictDialog displays a dialog for handling mask name conflicts
func showMaskConflictDialog(conflictInfo *services.MaskConflictInfo, reader fyne.URIReadCloser, originalDestPath, fileName, fileExt, programName string) {
	// Format existing file information
	existingFileInfo := fmt.Sprintf("Existing file: %s\nSize: %.2f KB\nModified: %s", 
		conflictInfo.ExistingFileName,
		float64(conflictInfo.ExistingFileSize)/1024,
		conflictInfo.ExistingFileModTime.Format("2006-01-02 15:04:05"))
	
	// Create conflict message
	conflictMessage := fmt.Sprintf("A mask with the name '%s' already exists in program '%s'\n\n%s\n\nHow would you like to resolve this conflict?", 
		conflictInfo.MaskName, conflictInfo.ProgramName, existingFileInfo)
	
	// Create dialog content
	messageLabel := widget.NewLabel(conflictMessage)
	messageLabel.Wrapping = fyne.TextWrapWord
	
	// Create buttons for conflict resolution
	var conflictDialog *dialog.CustomDialog
	
	// Option 1: Replace existing file
	replaceButton := widget.NewButton("Replace Existing", func() {
		conflictDialog.Hide()
		
		// Proceed with upload, overwriting existing file
		if err := copyFileWithValidation(reader, originalDestPath, fileName, programName); err != nil {
			log.Printf("Error: Mask upload failed during replacement: %v", err)
			return
		}
		
		log.Printf("Successfully replaced existing mask: %s in program: %s", conflictInfo.MaskName, programName)
		
		// Refresh UI and show success
		refreshMasksAccordion()
		ui.GetUi().Window.Canvas().Focus(nil)
		dialog.ShowInformation("Upload Successful", 
			fmt.Sprintf("Mask '%s' has been replaced in program '%s'", conflictInfo.MaskName, programName), ui.GetUi().Window)
	})
	replaceButton.Importance = widget.DangerImportance
	
	// Option 2: Use suggested name
	var renameButton *widget.Button
	if conflictInfo.SuggestedName != "" {
		renameButton = widget.NewButton(fmt.Sprintf("Rename to '%s'", conflictInfo.SuggestedName), func() {
			conflictDialog.Hide()
			
			// Create new destination path with suggested name
			newFileName := conflictInfo.SuggestedName + fileExt
			newDestPath := filepath.Join(filepath.Dir(originalDestPath), newFileName)
			
			// Proceed with upload using new name
			if err := copyFileWithValidation(reader, newDestPath, newFileName, programName); err != nil {
				log.Printf("Error: Mask upload failed during rename: %v", err)
				return
			}
			
			log.Printf("Successfully uploaded mask with new name: %s (original: %s) in program: %s", 
				conflictInfo.SuggestedName, conflictInfo.MaskName, programName)
			
			// Refresh UI and show success
			refreshMasksAccordion()
			ui.GetUi().Window.Canvas().Focus(nil)
			dialog.ShowInformation("Upload Successful", 
				fmt.Sprintf("Mask uploaded as '%s' in program '%s'", conflictInfo.SuggestedName, programName), ui.GetUi().Window)
		})
		renameButton.Importance = widget.SuccessImportance
	}
	
	// Option 3: Cancel upload
	cancelButton := widget.NewButton("Cancel Upload", func() {
		conflictDialog.Hide()
		log.Printf("Mask upload cancelled by user due to name conflict: %s in program: %s", 
			conflictInfo.MaskName, programName)
	})
	
	// Create button container
	var buttons *fyne.Container
	if renameButton != nil {
		buttons = container.NewHBox(
			cancelButton,
			widget.NewSeparator(),
			renameButton,
			replaceButton,
		)
	} else {
		buttons = container.NewHBox(
			cancelButton,
			widget.NewSeparator(),
			replaceButton,
		)
	}
	
	// Create dialog content
	content := container.NewVBox(
		messageLabel,
		widget.NewSeparator(),
		buttons,
	)
	
	// Show conflict resolution dialog
	conflictDialog = dialog.NewCustom("Mask Name Conflict", "", content, ui.GetUi().Window)
	conflictDialog.Resize(fyne.NewSize(500, 300))
	conflictDialog.Show()
	
	log.Printf("Showing mask conflict dialog for: %s in program: %s", conflictInfo.MaskName, programName)
}

// refreshMasksAccordion refreshes the masks accordion to show newly uploaded masks
func refreshMasksAccordion() {
	masksTab := ui.GetUi().EditorTabs.MasksTab
	if accordion, ok := masksTab.Widgets["Accordion"].(*widget.Accordion); ok {
		// Clear current selection to ensure clean state
		masksTab.SelectedItem = nil
		
		// Clear preview image
		maskAccordionService := services.MaskAccordionServiceInstance()
		maskAccordionService.ClearMaskPreview(masksTab.PreviewImage)
		
		// Repopulate the accordion with updated mask data
		setAccordionMasksLists(accordion)
		
		// Refresh the accordion widget to ensure UI updates
		accordion.Refresh()
		
		log.Printf("Masks accordion refreshed successfully")
	} else {
		log.Printf("Warning: Could not find masks accordion widget for refresh")
	}
}