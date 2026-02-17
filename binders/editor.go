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
	"log"
	"os"
	"strconv"
	"strings"

	"Squire/ui/completionentry"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/container"
	"fyne.io/fyne/v2/dialog"
	"fyne.io/fyne/v2/widget"
	"github.com/go-vgo/robotgo"
	hook "github.com/luhrMan/gohook"
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
	et.ProgramsTab.SelectedItem = repositories.ProgramRepo().New()
	// Note: For nested models, we need a program context to get repositories
	// These will be set to proper instances when a program is selected
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

	et.ItemsTab.Widgets["Form"].(*widget.Form).OnSubmit = func() {
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
		}
	}
	et.PointsTab.Widgets["Form"].(*widget.Form).OnSubmit = func() {
		w := et.PointsTab.Widgets
		n := w["Name"].(*widget.Entry).Text
		xText := w["X"].(*widget.Entry).Text
		yText := w["Y"].(*widget.Entry).Text
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
						log.Printf("Point: Preview update panic recovered after form update - %v (point: %s)", r, v.Name)
					}
				}()
				ui.GetUi().UpdatePointPreview(v)
			}()

			t := w[program.Name+"-searchbar"].(*widget.Entry).Text
			w[program.Name+"-searchbar"].(*widget.Entry).SetText("random string of text for refreshing because poop")
			w[program.Name+"-searchbar"].(*widget.Entry).SetText(t)
		}
	}

	// Set up record button handler for Points tab
	if recordButton, ok := et.PointsTab.Widgets["recordButton"].(*widget.Button); ok {
		recordButton.OnTapped = func() {
			var dlg dialog.Dialog
			content := container.NewVBox(
				widget.NewLabel("Left click anwhere to record X and Y coordinates"),
				widget.NewLabel("Right click to cancel"),
			)

			dlg = dialog.NewCustomWithoutButtons(
				"Record Point Coordinates",
				content,
				ui.GetUi().Window,
			)
			// Set up a goroutine to detect a click outside the dialog and record the coordinates
			go func() {
				adjustedX, adjustedY := 0, 0
				hook.Register(hook.MouseDown, []string{}, func(e hook.Event) {
					switch e.Button {
					case hook.MouseMap["left"]:
						x, y := robotgo.Location()
						adjustedX = x - config.XOffset
						adjustedY = y - config.YOffset
						fyne.CurrentApp().SendNotification(&fyne.Notification{
							Title:   "Captured Point",
							Content: fmt.Sprintf("X: %d, Y: %d", adjustedX, adjustedY),
						})
						if xEntry, ok := et.PointsTab.Widgets["X"].(*widget.Entry); ok {
							fyne.DoAndWait(func() {
								xEntry.SetText(strconv.Itoa(adjustedX))
							})
						}
						if yEntry, ok := et.PointsTab.Widgets["Y"].(*widget.Entry); ok {
							fyne.DoAndWait(func() {
								yEntry.SetText(strconv.Itoa(adjustedY))
							})
						}
						if point, ok := et.PointsTab.SelectedItem.(*models.Point); ok {
							point.X = adjustedX
							point.Y = adjustedY
							func() {
								defer func() {
									if r := recover(); r != nil {
										log.Printf("Point: Preview update panic recovered after recording - %v (point: %s)", r, point.Name)
									}
								}()
								ui.GetUi().UpdatePointPreview(point)
							}()
						}
						hook.Unregister(hook.MouseDown, []string{})
						fyne.DoAndWait(func() {
							dlg.Dismiss()
						})
					default:
						hook.Unregister(hook.MouseDown, []string{})
						fyne.DoAndWait(func() {
							dlg.Dismiss()
						})
					}
				})
			}()

			dlg.Show()
		}
	}

	// Set up record button handler for Search Areas tab (two clicks: top-left, then bottom-right)
	if saRecordButton, ok := et.SearchAreasTab.Widgets["recordButton"].(*widget.Button); ok {
		saRecordButton.OnTapped = func() {
			var dlg dialog.Dialog
			content := container.NewVBox(
				widget.NewLabel("First click: top-left corner of the search area."),
				widget.NewLabel("Second click: bottom-right corner. Right click to cancel."),
			)

			dlg = dialog.NewCustomWithoutButtons(
				"Record Search Area",
				content,
				ui.GetUi().Window,
			)
			go func() {
				leftX, topY := 0, 0
				firstClickDone := false
				hook.Register(hook.MouseDown, []string{}, func(e hook.Event) {
					if e.Button != hook.MouseMap["left"] {
						hook.Unregister(hook.MouseDown, []string{})
						fyne.DoAndWait(func() { dlg.Dismiss() })
						return
					}
					x, y := robotgo.Location()
					adjX := x - config.XOffset
					adjY := y - config.YOffset
					if !firstClickDone {
						leftX = adjX
						topY = adjY
						firstClickDone = true
						fyne.CurrentApp().SendNotification(&fyne.Notification{
							Title:   "Search Area",
							Content: "Top-left set. Click bottom-right corner.",
						})
						return
					}
					// Second click: bottom-right corner (normalize in case clicks were reversed)
					rightX := adjX
					bottomY := adjY
					if leftX > rightX {
						leftX, rightX = rightX, leftX
					}
					if topY > bottomY {
						topY, bottomY = bottomY, topY
					}
					hook.Unregister(hook.MouseDown, []string{})
					fyne.DoAndWait(func() {
						if w, ok := et.SearchAreasTab.Widgets["LeftX"].(*widget.Entry); ok {
							w.SetText(strconv.Itoa(leftX))
						}
						if w, ok := et.SearchAreasTab.Widgets["TopY"].(*widget.Entry); ok {
							w.SetText(strconv.Itoa(topY))
						}
						if w, ok := et.SearchAreasTab.Widgets["RightX"].(*widget.Entry); ok {
							w.SetText(strconv.Itoa(rightX))
						}
						if w, ok := et.SearchAreasTab.Widgets["BottomY"].(*widget.Entry); ok {
							w.SetText(strconv.Itoa(bottomY))
						}
						if sa, ok := et.SearchAreasTab.SelectedItem.(*models.SearchArea); ok {
							sa.LeftX = leftX
							sa.TopY = topY
							sa.RightX = rightX
							sa.BottomY = bottomY
							func() {
								defer func() {
									if r := recover(); r != nil {
										log.Printf("SearchArea: Preview update panic recovered after recording - %v (area: %s)", r, sa.Name)
									}
								}()
								ui.GetUi().UpdateSearchAreaPreview(sa)
							}()
						}
						fyne.CurrentApp().SendNotification(&fyne.Notification{
							Title:   "Captured Search Area",
							Content: fmt.Sprintf("LeftX: %d, TopY: %d, RightX: %d, BottomY: %d", leftX, topY, rightX, bottomY),
						})
						dlg.Dismiss()
					})
				})
			}()

			dlg.Show()
		}
	}

	et.SearchAreasTab.Widgets["Form"].(*widget.Form).OnSubmit = func() {
		w := et.SearchAreasTab.Widgets
		n := w["Name"].(*widget.Entry).Text
		lxText := w["LeftX"].(*widget.Entry).Text
		tyText := w["TopY"].(*widget.Entry).Text
		rxText := w["RightX"].(*widget.Entry).Text
		byText := w["BottomY"].(*widget.Entry).Text
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
			// Set the selected item so tag operations work correctly
			ui.GetUi().EditorTabs.ItemsTab.SelectedItem = i
			v := i
			ui.GetUi().EditorTabs.ItemsTab.Widgets["Name"].(*widget.Entry).SetText(v.Name)
			t := ui.GetUi().EditorTabs.ItemsTab.Widgets[program+"-searchbar"].(*widget.Entry).Text
			ui.GetUi().EditorTabs.ItemsTab.Widgets[program+"-searchbar"].(*widget.Entry).SetText("random string of text for refreshing because poop")
			ui.GetUi().EditorTabs.ItemsTab.Widgets[program+"-searchbar"].(*widget.Entry).SetText(t)
			setItemsWidgets(*i)
			// RefreshItemsAccordionItems()
		case "Points":
			n := ui.GetUi().EditorTabs.PointsTab.Widgets["Name"].(*widget.Entry).Text
			xText := ui.GetUi().EditorTabs.PointsTab.Widgets["X"].(*widget.Entry).Text
			yText := ui.GetUi().EditorTabs.PointsTab.Widgets["Y"].(*widget.Entry).Text
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
			ui.GetUi().EditorTabs.PointsTab.Widgets["Name"].(*widget.Entry).SetText(p.Name)
			t := ui.GetUi().EditorTabs.PointsTab.Widgets[program+"-searchbar"].(*widget.Entry).Text
			ui.GetUi().EditorTabs.PointsTab.Widgets[program+"-searchbar"].(*widget.Entry).SetText("random string of text for refreshing because poop")
			ui.GetUi().EditorTabs.PointsTab.Widgets[program+"-searchbar"].(*widget.Entry).SetText(t)
			// refreshAllProgramRelatedUI()
		case "Search Areas":
			n := ui.GetUi().EditorTabs.SearchAreasTab.Widgets["Name"].(*widget.Entry).Text
			lxText := ui.GetUi().EditorTabs.SearchAreasTab.Widgets["LeftX"].(*widget.Entry).Text
			tyText := ui.GetUi().EditorTabs.SearchAreasTab.Widgets["TopY"].(*widget.Entry).Text
			rxText := ui.GetUi().EditorTabs.SearchAreasTab.Widgets["RightX"].(*widget.Entry).Text
			byText := ui.GetUi().EditorTabs.SearchAreasTab.Widgets["BottomY"].(*widget.Entry).Text
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
		}
	}

}
