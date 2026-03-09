package binders

import (
	"Sqyre/internal/config"
	"Sqyre/internal/models"
	"Sqyre/internal/models/repositories"
	"Sqyre/internal/services"
	"Sqyre/ui"
	"Sqyre/ui/custom_widgets"
	"errors"
	"log"
	"os"
	"path/filepath"
	"strconv"
	"strings"
	"sync"
	"time"

	"Sqyre/ui/completionentry"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/dialog"
	"fyne.io/fyne/v2/widget"
	"github.com/go-vgo/robotgo"
	hook "github.com/luhrMan/gohook"
)

var (
	programFields    = []string{"Name"}
	itemFields       = []string{"Name", "Cols", "Rows", "StackMax"}
	pointFields      = []string{"Name", "X", "Y"}
	searchAreaFields = []string{"Name", "LeftX", "TopY", "RightX", "BottomY"}
	maskFields       = []string{"Name", "shapeSelect", "CenterX", "CenterY", "Base", "Height", "Radius", "Inverse"}
)

func getWidgetText(w fyne.CanvasObject) string {
	switch e := w.(type) {
	case *widget.Entry:
		return e.Text
	case *custom_widgets.VarEntry:
		return e.Text
	case *widget.RadioGroup:
		return e.Selected
	case *widget.Check:
		if e.Checked {
			return "true"
		}
		return "false"
	}
	return ""
}

func markTabClean(tab *ui.EditorTab, fields []string) {
	tab.OriginalValues = make(map[string]string)
	for _, f := range fields {
		tab.OriginalValues[f] = getWidgetText(tab.Widgets[f])
	}
	if tab.UpdateButton != nil {
		tab.UpdateButton.Disable()
	}
}

func checkTabDirty(tab *ui.EditorTab, fields []string) {
	if tab.UpdateButton == nil || tab.OriginalValues == nil {
		return
	}
	for _, f := range fields {
		if getWidgetText(tab.Widgets[f]) != tab.OriginalValues[f] {
			tab.UpdateButton.Enable()
			return
		}
	}
	tab.UpdateButton.Disable()
}

func setupDirtyTracking(tab *ui.EditorTab, fields []string) {
	for _, f := range fields {
		w := tab.Widgets[f]
		switch e := w.(type) {
		case *widget.Entry:
			prev := e.OnChanged
			e.OnChanged = func(s string) {
				if prev != nil {
					prev(s)
				}
				checkTabDirty(tab, fields)
			}
		case *custom_widgets.VarEntry:
			prev := e.OnChanged
			e.OnChanged = func(s string) {
				if prev != nil {
					prev(s)
				}
				checkTabDirty(tab, fields)
			}
		case *widget.RadioGroup:
			prev := e.OnChanged
			e.OnChanged = func(s string) {
				if prev != nil {
					prev(s)
				}
				checkTabDirty(tab, fields)
			}
		case *widget.Check:
			prev := e.OnChanged
			e.OnChanged = func(checked bool) {
				if prev != nil {
					prev(checked)
				}
				checkTabDirty(tab, fields)
			}
		}
	}
}

func setupAllDirtyTracking() {
	et := ui.GetUi().EditorTabs
	setupDirtyTracking(et.ProgramsTab, programFields)
	setupDirtyTracking(et.ItemsTab, itemFields)
	setupDirtyTracking(et.PointsTab, pointFields)
	setupDirtyTracking(et.SearchAreasTab, searchAreaFields)
	setupDirtyTracking(et.MasksTab, maskFields)
}

func markProgramsClean() {
	markTabClean(ui.GetUi().EditorTabs.ProgramsTab, programFields)
}

func markItemsClean() {
	markTabClean(ui.GetUi().EditorTabs.ItemsTab, itemFields)
}

func markPointsClean() {
	markTabClean(ui.GetUi().EditorTabs.PointsTab, pointFields)
}

func markSearchAreasClean() {
	markTabClean(ui.GetUi().EditorTabs.SearchAreasTab, searchAreaFields)
}

func markMasksClean() {
	markTabClean(ui.GetUi().EditorTabs.MasksTab, maskFields)
}

func SetEditorUi() {
	setEditorLists()
	setEditorForms()
	setEditorButtons()
	setMasksForms()
	setMasksButtons()
	setMaskSelectionButtons()
	updateProgramSelectorOptions()
	setupAllDirtyTracking()
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
		programList.Refresh()
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

	// // Refresh action tab accordions
	// ats := ui.GetUi().ActionTabs
	// if ats.ImageSearchItemsAccordion != nil {
	// 	setAccordionItemsLists(ats.ImageSearchItemsAccordion)
	// }
	// if ats.PointsAccordion != nil {
	// 	setAccordionPointsLists(ats.PointsAccordion)
	// }
	// if ats.ImageSearchSAAccordion != nil {
	// 	setAccordionSearchAreasLists(ats.ImageSearchSAAccordion)
	// }
	// if ats.OcrSAAccordion != nil {
	// 	setAccordionSearchAreasLists(ats.OcrSAAccordion)
	// }
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
	et.MasksTab.SelectedItem = &models.Mask{}
	et.AutoPicTab.SelectedItem = &models.SearchArea{}
}

func setEditorForms() {
	et := ui.GetUi().EditorTabs
	et.ProgramsTab.UpdateButton.OnTapped = func() {
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

			refreshAllProgramRelatedUI()
			updateProgramSelectorOptions()
			markProgramsClean()
		}
	}
	// Set up tag entry handler for adding new tags with completion
	if tagEntry, ok := et.ItemsTab.Widgets["tagEntry"].(*completionentry.CompletionEntry); ok {
		// Function to submit a tag (used by both Enter key and button)
		submitTag := func() {
			tagText := tagEntry.Text
			// Hide completion popup when submitting
			tagEntry.HideCompletion()

			if tagText == "" {
				return
			}

			// Trim whitespace
			tagText = strings.TrimSpace(tagText)
			if tagText == "" {
				return
			}

			if v, ok := et.ItemsTab.SelectedItem.(*models.Item); ok {
				// Check if tag already exists (avoid duplicates)
				for _, existingTag := range v.Tags {
					if existingTag == tagText {
						return // Tag already exists, do nothing
					}
				}

				// Add the tag
				v.Tags = append(v.Tags, tagText)

				// Save the item
				p := ui.GetUi().ProgramSelector.Text
				program, err := repositories.ProgramRepo().Get(p)
				if err != nil {
					log.Printf("Error getting program %s: %v", p, err)
					return
				}

				if err := program.ItemRepo().Set(v.Name, v); err != nil {
					log.Printf("Error saving item %s: %v", v.Name, err)
					dialog.ShowError(errors.New("failed to save item"), ui.GetUi().Window)
					return
				}

				if err := repositories.ProgramRepo().Set(program.Name, program); err != nil {
					log.Printf("Error saving program %s: %v", p, err)
					return
				}

				// Refresh the tags display
				updateTagsDisplay(v)

				// Clear the entry field
				tagEntry.SetText("")
			}
		}

		// Set up OnChanged to show completion suggestions
		tagEntry.OnChanged = func(text string) {
			if text == "" {
				tagEntry.HideCompletion()
				return
			}

			// Get all existing tags
			allTags := getAllExistingTags()

			// Filter tags that match the search text (case-insensitive)
			searchLower := strings.ToLower(text)
			matchingTags := []string{}
			for _, tag := range allTags {
				if strings.Contains(strings.ToLower(tag), searchLower) {
					matchingTags = append(matchingTags, tag)
				}
			}

			// Limit to 10 suggestions
			if len(matchingTags) > 10 {
				matchingTags = matchingTags[:10]
			}

			if len(matchingTags) == 0 {
				tagEntry.HideCompletion()
				return
			}

			// Set options and show completion
			tagEntry.SetOptions(matchingTags)
			tagEntry.ShowCompletion()
		}

		tagEntry.OnSubmitted = func(tagText string) {
			submitTag()
		}

		// Set up the submit button handler
		if tagSubmitButton, ok := et.ItemsTab.Widgets["tagSubmitButton"].(*widget.Button); ok {
			tagSubmitButton.OnTapped = submitTag
		}
	}

	et.ItemsTab.UpdateButton.OnTapped = func() {
		w := et.ItemsTab.Widgets
		n := w["Name"].(*widget.Entry).Text
		x, _ := strconv.Atoi(w["Cols"].(*widget.Entry).Text)
		y, _ := strconv.Atoi(w["Rows"].(*widget.Entry).Text)
		sm, _ := strconv.Atoi(w["StackMax"].(*widget.Entry).Text)
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
			markItemsClean()
		}
	}
	et.PointsTab.UpdateButton.OnTapped = func() {
		w := et.PointsTab.Widgets
		n := w["Name"].(*widget.Entry).Text
		xText := custom_widgets.EntryText(w["X"])
		yText := custom_widgets.EntryText(w["Y"])
		var xVal, yVal any
		if x, err := strconv.Atoi(xText); err == nil {
			xVal = x
		} else {
			xVal = xText
		}
		if y, err := strconv.Atoi(yText); err == nil {
			yVal = y
		} else {
			yVal = yText
		}
		if v, ok := et.PointsTab.SelectedItem.(*models.Point); ok {
			p := ui.GetUi().ProgramSelector.Text
			program, err := repositories.ProgramRepo().Get(p)
			if err != nil {
				log.Printf("Error getting program %s: %v", p, err)
				return
			}
			oldkey := v.Name
			v.Name = n
			v.X = xVal
			v.Y = yVal

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

			// Update point preview after form submission
			func() {
				defer func() {
					if r := recover(); r != nil {
						services.LogPanicToFile(r, "Point: Preview update (point: "+v.Name+")")
					}
				}()
				ui.GetUi().UpdatePointPreview(v)
			}()

			t := w[program.Name+"-searchbar"].(*widget.Entry).Text
			w[program.Name+"-searchbar"].(*widget.Entry).SetText("random string of text for refreshing because poop")
			w[program.Name+"-searchbar"].(*widget.Entry).SetText(t)
			markPointsClean()
		}
	}

	// Set up record button handler for Points tab
	if recordButton, ok := et.PointsTab.Widgets["recordButton"].(*widget.Button); ok {
		recordButton.OnTapped = func() {
			dismissOverlay := ui.ShowRecordingOverlay(
				"Record Point Coordinates",
				"Left click anywhere to record X and Y coordinates",
				"Right click to cancel",
			)

			services.GoSafe(func() {
				hook.Register(hook.MouseDown, []string{}, func(e hook.Event) {
					switch e.Button {
					case hook.MouseMap["left"]:
						x, y := robotgo.Location()
						fyne.DoAndWait(func() {
							custom_widgets.SetEntryText(et.PointsTab.Widgets["X"], strconv.Itoa(x))
							custom_widgets.SetEntryText(et.PointsTab.Widgets["Y"], strconv.Itoa(y))
							dismissOverlay()
						})
					default:
						fyne.DoAndWait(func() {
							dismissOverlay()
						})
					}
					go hook.Unregister(hook.MouseDown, []string{})
				})
			})
		}
	}

	// Set up record button handler for Search Areas tab (two clicks: top-left, then bottom-right)
	if saRecordButton, ok := et.SearchAreasTab.Widgets["recordButton"].(*widget.Button); ok {
		saRecordButton.OnTapped = func() {
			dismissOverlay, setSelectionRect := ui.ShowSearchAreaRecordingOverlay(
				"Record Search Area",
				"First click: top-left corner of the search area.",
				"Second click: bottom-right corner. Right click to cancel.",
			)

		services.GoSafe(func() {
			var mu sync.Mutex
			leftX, topY := 0, 0
			firstClickDone := false
			stopPoll := make(chan struct{})
			var stopOnce sync.Once
			stopPolling := func() { stopOnce.Do(func() { close(stopPoll) }) }

			services.GoSafe(func() {
				for {
					select {
					case <-stopPoll:
						return
					default:
						mu.Lock()
						done := firstClickDone
						lx, ty := leftX, topY
						mu.Unlock()
						if !done {
							setSelectionRect(0, 0, 0, 0)
						} else {
							x, y := robotgo.Location()
							rx, by := x, y
							if lx > rx {
								lx, rx = rx, lx
							}
							if ty > by {
								ty, by = by, ty
							}
							setSelectionRect(lx, ty, rx, by)
						}
					}
					select {
					case <-stopPoll:
						return
					case <-time.After(50 * time.Millisecond):
					}
				}
			})

			hook.Register(hook.MouseDown, []string{}, func(e hook.Event) {
				if e.Button != hook.MouseMap["left"] {
					stopPolling()
					fyne.DoAndWait(func() { dismissOverlay() })
					go hook.Unregister(hook.MouseDown, []string{})
					return
				}
				x, y := robotgo.Location()
				adjX, adjY := x, y
				mu.Lock()
				if !firstClickDone {
					leftX, topY = adjX, adjY
					firstClickDone = true
					mu.Unlock()
					return
				}
				rightX, bottomY := adjX, adjY
				lx, ty := leftX, topY
				mu.Unlock()
				if lx > rightX {
					lx, rightX = rightX, lx
				}
				if ty > bottomY {
					ty, bottomY = bottomY, ty
				}
				leftX, topY = lx, ty
				stopPolling()
		fyne.DoAndWait(func() {
			custom_widgets.SetEntryText(et.SearchAreasTab.Widgets["LeftX"], strconv.Itoa(leftX))
			custom_widgets.SetEntryText(et.SearchAreasTab.Widgets["TopY"], strconv.Itoa(topY))
			custom_widgets.SetEntryText(et.SearchAreasTab.Widgets["RightX"], strconv.Itoa(rightX))
			custom_widgets.SetEntryText(et.SearchAreasTab.Widgets["BottomY"], strconv.Itoa(bottomY))
					dismissOverlay()
				})
				go hook.Unregister(hook.MouseDown, []string{})
			})
		})
		}
	}

	et.SearchAreasTab.UpdateButton.OnTapped = func() {
		w := et.SearchAreasTab.Widgets
		n := w["Name"].(*widget.Entry).Text
		lxText := custom_widgets.EntryText(w["LeftX"])
		tyText := custom_widgets.EntryText(w["TopY"])
		rxText := custom_widgets.EntryText(w["RightX"])
		byText := custom_widgets.EntryText(w["BottomY"])
		var lxVal, tyVal, rxVal, byVal any
		if v, err := strconv.Atoi(lxText); err == nil {
			lxVal = v
		} else {
			lxVal = lxText
		}
		if v, err := strconv.Atoi(tyText); err == nil {
			tyVal = v
		} else {
			tyVal = tyText
		}
		if v, err := strconv.Atoi(rxText); err == nil {
			rxVal = v
		} else {
			rxVal = rxText
		}
		if v, err := strconv.Atoi(byText); err == nil {
			byVal = v
		} else {
			byVal = byText
		}
		if v, ok := et.SearchAreasTab.SelectedItem.(*models.SearchArea); ok {
			p := ui.GetUi().ProgramSelector.Text
			program, err := repositories.ProgramRepo().Get(p)
			if err != nil {
				log.Printf("Error getting program %s: %v", p, err)
				return
			}
			oldkey := v.Name
			v.Name = n
			v.LeftX = lxVal
			v.TopY = tyVal
			v.RightX = rxVal
			v.BottomY = byVal

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
						services.LogPanicToFile(r, "SearchArea: Preview update (area: "+v.Name+")")
					}
				}()
				ui.GetUi().UpdateSearchAreaPreview(v)
			}()
			t := w[program.Name+"-searchbar"].(*widget.Entry).Text
			w[program.Name+"-searchbar"].(*widget.Entry).SetText("random string of text for refreshing because poop")
			w[program.Name+"-searchbar"].(*widget.Entry).SetText(t)
			markSearchAreasClean()
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
			pro := getProgram(n)
			if pro != nil {
				ui.GetUi().EditorTabs.ProgramsTab.SelectedItem = pro
			}
			refreshAllProgramRelatedUI()
			updateProgramSelectorOptions()

			ui.GetUi().EditorTabs.ProgramsTab.Widgets["Name"].(*widget.Entry).SetText(n)
			markProgramsClean()
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
			// Set the selected item so tag operations work correctly
			ui.GetUi().EditorTabs.ItemsTab.SelectedItem = i
			v := i
			ui.GetUi().EditorTabs.ItemsTab.Widgets["Name"].(*widget.Entry).SetText(v.Name)
			t := ui.GetUi().EditorTabs.ItemsTab.Widgets[program+"-searchbar"].(*widget.Entry).Text
			ui.GetUi().EditorTabs.ItemsTab.Widgets[program+"-searchbar"].(*widget.Entry).SetText("random string of text for refreshing because poop")
			ui.GetUi().EditorTabs.ItemsTab.Widgets[program+"-searchbar"].(*widget.Entry).SetText(t)
			setItemsWidgets(*i)
			markItemsClean()
		case "Points":
			n := ui.GetUi().EditorTabs.PointsTab.Widgets["Name"].(*widget.Entry).Text
			xText := custom_widgets.EntryText(ui.GetUi().EditorTabs.PointsTab.Widgets["X"])
			yText := custom_widgets.EntryText(ui.GetUi().EditorTabs.PointsTab.Widgets["Y"])
			var xVal, yVal any
			if x, err := strconv.Atoi(xText); err == nil {
				xVal = x
			} else {
				xVal = xText
			}
			if y, err := strconv.Atoi(yText); err == nil {
				yVal = y
			} else {
				yVal = yText
			}

			pro := getProgram(program)
			if pro == nil {
				return
			}

			// Create new point using repository New() function
			p := pro.PointRepo(config.MainMonitorSizeString).New()
			p.Name = n
			p.X = xVal
			p.Y = yVal

			err := pro.PointRepo(config.MainMonitorSizeString).Set(p.Name, p)
			if err != nil {
				dialog.ShowError(err, ui.GetUi().Window)
				return
			}
			ui.GetUi().EditorTabs.PointsTab.SelectedItem = p
			setPointWidgets(*p)
			t := ui.GetUi().EditorTabs.PointsTab.Widgets[program+"-searchbar"].(*widget.Entry).Text
			ui.GetUi().EditorTabs.PointsTab.Widgets[program+"-searchbar"].(*widget.Entry).SetText("random string of text for refreshing because poop")
			ui.GetUi().EditorTabs.PointsTab.Widgets[program+"-searchbar"].(*widget.Entry).SetText(t)
			markPointsClean()
		case "Masks":
			w := ui.GetUi().EditorTabs.MasksTab.Widgets
			n := w["Name"].(*widget.Entry).Text

			pro := getProgram(program)
			if pro == nil {
				return
			}

			m := pro.MaskRepo().New()
			m.Name = n
			m.Shape = readMaskShapeFromUI()
			m.CenterX = custom_widgets.EntryText(w["CenterX"])
			m.CenterY = custom_widgets.EntryText(w["CenterY"])
			m.Base = custom_widgets.EntryText(w["Base"])
			m.Height = custom_widgets.EntryText(w["Height"])
			m.Radius = custom_widgets.EntryText(w["Radius"])

			err := pro.MaskRepo().Set(m.Name, m)
			if err != nil {
				dialog.ShowError(err, ui.GetUi().Window)
				return
			}
			ui.GetUi().EditorTabs.MasksTab.SelectedItem = m
			setMaskWidgets(*m, program)
			t := ui.GetUi().EditorTabs.MasksTab.Widgets[program+"-searchbar"].(*widget.Entry).Text
			ui.GetUi().EditorTabs.MasksTab.Widgets[program+"-searchbar"].(*widget.Entry).SetText("refresh")
			ui.GetUi().EditorTabs.MasksTab.Widgets[program+"-searchbar"].(*widget.Entry).SetText(t)
			markMasksClean()
		case "Search Areas":
			n := ui.GetUi().EditorTabs.SearchAreasTab.Widgets["Name"].(*widget.Entry).Text
			lxText := custom_widgets.EntryText(ui.GetUi().EditorTabs.SearchAreasTab.Widgets["LeftX"])
			tyText := custom_widgets.EntryText(ui.GetUi().EditorTabs.SearchAreasTab.Widgets["TopY"])
			rxText := custom_widgets.EntryText(ui.GetUi().EditorTabs.SearchAreasTab.Widgets["RightX"])
			byText := custom_widgets.EntryText(ui.GetUi().EditorTabs.SearchAreasTab.Widgets["BottomY"])
			var lxVal, tyVal, rxVal, byVal any
			if v, err := strconv.Atoi(lxText); err == nil {
				lxVal = v
			} else {
				lxVal = lxText
			}
			if v, err := strconv.Atoi(tyText); err == nil {
				tyVal = v
			} else {
				tyVal = tyText
			}
			if v, err := strconv.Atoi(rxText); err == nil {
				rxVal = v
			} else {
				rxVal = rxText
			}
			if v, err := strconv.Atoi(byText); err == nil {
				byVal = v
			} else {
				byVal = byText
			}

			pro := getProgram(program)
			if pro == nil {
				return
			}

			sa := pro.SearchAreaRepo(config.MainMonitorSizeString).New()
			sa.Name = n
			sa.LeftX = lxVal
			sa.TopY = tyVal
			sa.RightX = rxVal
			sa.BottomY = byVal

			err := pro.SearchAreaRepo(config.MainMonitorSizeString).Set(sa.Name, sa)
			if err != nil {
				dialog.ShowError(err, ui.GetUi().Window)
				return
			}
			// Select the newly added search area so it can be edited with Update
			ui.GetUi().EditorTabs.SearchAreasTab.SelectedItem = sa
			setSearchAreaWidgets(*sa)
			t := ui.GetUi().EditorTabs.SearchAreasTab.Widgets[program+"-searchbar"].(*widget.Entry).Text
			ui.GetUi().EditorTabs.SearchAreasTab.Widgets[program+"-searchbar"].(*widget.Entry).SetText("random string of text for refreshing because poop")
			ui.GetUi().EditorTabs.SearchAreasTab.Widgets[program+"-searchbar"].(*widget.Entry).SetText(t)
			markSearchAreasClean()
		}

	}
	ui.GetUi().EditorUi.RemoveButton.OnTapped = func() {
		var (
			program                = ui.GetUi().EditorUi.ProgramSelector.Text
			et                     = ui.GetUi().EditorTabs
			prot, it, pt, sat, mkt = et.ProgramsTab, et.ItemsTab, et.PointsTab, et.SearchAreasTab, et.MasksTab
			prog, err              = repositories.ProgramRepo().Get(program)
		)
		_ = mkt
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
		case "Masks":
			n := mkt.SelectedItem.(*models.Mask).Name
			err = prog.MaskRepo().Delete(n)
			if err != nil {
				log.Printf("Error deleting mask %s: %v", n, err)
				return
			}

			// Remove mask image file if it exists
			masksPath := config.GetMasksPath()
			imgPath := filepath.Join(masksPath, program, n+config.PNG)
			if removeErr := os.Remove(imgPath); removeErr != nil && !os.IsNotExist(removeErr) {
				log.Printf("Warning: Failed to remove mask image %s: %v", imgPath, removeErr)
			}

			mkt.SelectedItem = &models.Mask{}
			ui.GetUi().SetMaskImageMode(false)
			ui.GetUi().ClearMaskPreviewImage()
			if searchbar, ok := mkt.Widgets[program+"-searchbar"].(*widget.Entry); ok {
				text := searchbar.Text
				searchbar.SetText("refresh")
				searchbar.SetText(text)
			}
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
		}
	}

}
