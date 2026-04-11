package editor

import (
	"Sqyre/internal/config"
	sqdesktop "Sqyre/internal/desktop"
	"Sqyre/internal/models"
	"Sqyre/internal/models/repositories"
	"Sqyre/internal/services"
	"Sqyre/ui/custom_widgets"
	"errors"
	"fmt"
	"log"
	"os"
	"strconv"
	"strings"
	"sync"
	"time"

	"Sqyre/ui/completionentry"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/driver/desktop"
	"fyne.io/fyne/v2/widget"
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

func markTabClean(tab *EditorTab, fields []string) {
	tab.OriginalValues = make(map[string]string)
	for _, f := range fields {
		tab.OriginalValues[f] = getWidgetText(tab.Widgets[f])
	}
	if tab.UpdateButton != nil {
		tab.UpdateButton.Disable()
	}
}

func checkTabDirty(tab *EditorTab, fields []string) {
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

func setupDirtyTracking(tab *EditorTab, fields []string) {
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
	et := shell().EditorTabs
	setupDirtyTracking(et.ProgramsTab, programFields)
	setupDirtyTracking(et.ItemsTab, itemFields)
	setupDirtyTracking(et.PointsTab, pointFields)
	setupDirtyTracking(et.SearchAreasTab, searchAreaFields)
	setupDirtyTracking(et.MasksTab, maskFields)
}

func markProgramsClean() {
	markTabClean(shell().EditorTabs.ProgramsTab, programFields)
}

func markItemsClean() {
	markTabClean(shell().EditorTabs.ItemsTab, itemFields)
}

func markPointsClean() {
	markTabClean(shell().EditorTabs.PointsTab, pointFields)
}

func markSearchAreasClean() {
	markTabClean(shell().EditorTabs.SearchAreasTab, searchAreaFields)
}

func markMasksClean() {
	markTabClean(shell().EditorTabs.MasksTab, maskFields)
}

// setEditorPreviewRefreshButtons wires preview refresh actions from current form entries (or disk/repo for masks/AutoPic).
func setEditorPreviewRefreshButtons() {
	et := shell().EditorTabs

	if et.PointsTab.PreviewRefreshButton != nil {
		et.PointsTab.PreviewRefreshButton.OnTapped = func() {
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
			func() {
				defer func() {
					if r := recover(); r != nil {
						services.LogPanicToFile(r, "Point: Preview refresh (point: "+n+")")
					}
				}()
				shell().UpdatePointPreview(&models.Point{Name: n, X: xVal, Y: yVal})
			}()
		}
	}

	if et.SearchAreasTab.PreviewRefreshButton != nil {
		et.SearchAreasTab.PreviewRefreshButton.OnTapped = func() {
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
			func() {
				defer func() {
					if r := recover(); r != nil {
						services.LogPanicToFile(r, "SearchArea: Preview refresh (area: "+n+")")
					}
				}()
				shell().UpdateSearchAreaPreview(&models.SearchArea{
					Name:    n,
					LeftX:   lxVal,
					TopY:    tyVal,
					RightX:  rxVal,
					BottomY: byVal,
				})
			}()
		}
	}

	if et.MasksTab.PreviewRefreshButton != nil {
		et.MasksTab.PreviewRefreshButton.OnTapped = func() {
			p := shell().EditorTabs.MasksTab.ProgramSelector.Selected
			if p == "" {
				return
			}
			n := et.MasksTab.Widgets["Name"].(*widget.Entry).Text
			if n == "" {
				return
			}
			if HasMaskImage(p, n) {
				shell().UpdateMaskPreview(p, n)
			} else {
				shell().ClearMaskPreviewImage()
			}
		}
	}

	if et.AutoPicTab.PreviewRefreshButton != nil {
		et.AutoPicTab.PreviewRefreshButton.OnTapped = func() {
			sa, ok := et.AutoPicTab.SelectedItem.(*models.SearchArea)
			if !ok || sa == nil || sa.Name == "" {
				return
			}
			func() {
				defer func() {
					if r := recover(); r != nil {
						services.LogPanicToFile(r, "AutoPic: Preview refresh (area: "+sa.Name+")")
					}
				}()
				shell().UpdateAutoPicPreview(sa)
			}()
		}
	}
}

// selectFirstProgramInEditorIfAny selects the first program (sorted keys) in the list and
// program selector when the editor UI is first wired up.
func selectFirstProgramInEditorIfAny() {
	if len(repositories.ProgramRepo().GetAllKeys()) == 0 {
		return
	}
	et := shell().EditorTabs
	if programList, ok := et.ProgramsTab.Widgets["list"].(*widget.List); ok {
		programList.Select(0)
	}
}

// updateProgramSelectorOptions refreshes every per-tab program selector with current programs.
func updateProgramSelectorOptions() {
	opts := repositories.ProgramRepo().GetAllKeys()
	et := shell().EditorTabs
	for _, tab := range []*EditorTab{
		et.ItemsTab, et.PointsTab,
		et.SearchAreasTab, et.MasksTab,
	} {
		if tab.ProgramSelector != nil {
			tab.ProgramSelector.Options = opts
			tab.ProgramSelector.Refresh()
		}
	}
}

// refreshAllProgramRelatedUI refreshes all accordions and program list when programs are modified
func refreshAllProgramRelatedUI() {
	// Refresh program list
	et := shell().EditorTabs
	if programList, ok := et.ProgramsTab.Widgets["list"].(*widget.List); ok {
		setProgramList(programList)
		programList.Refresh()
	}

	// Refresh editor tab accordions
	if accordion, ok := et.ItemsTab.Widgets["Accordion"].(*custom_widgets.AccordionWithHeaderWidgets); ok {
		setAccordionItemsLists(accordion)
	}
	if accordion, ok := et.PointsTab.Widgets["Accordion"].(*widget.Accordion); ok {
		setAccordionPointsLists(accordion)
	}
	syncEditorSearchAreaAccordions()
	if accordion, ok := et.MasksTab.Widgets["Accordion"].(*widget.Accordion); ok {
		setAccordionMasksLists(accordion)
	}
}

func setEditorLists() {
	et := shell().EditorTabs
	setProgramList(
		et.ProgramsTab.Widgets["list"].(*widget.List),
	)
	setAccordionItemsLists(
		et.ItemsTab.Widgets["Accordion"].(*custom_widgets.AccordionWithHeaderWidgets),
	)
	setAccordionPointsLists(
		et.PointsTab.Widgets["Accordion"].(*widget.Accordion),
	)
	syncEditorSearchAreaAccordions()
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
	shell().RefreshEditorActionBar()
}

func setEditorForms() {
	et := shell().EditorTabs
	et.ProgramsTab.UpdateButton.OnTapped = func() {
		w := et.ProgramsTab.Widgets
		n := w["Name"].(*widget.Entry).Text
		if si, ok := et.ProgramsTab.SelectedItem.(*models.Program); ok {
			applyProgramUpdate := func() {
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

			if si.Name != n {
				if shouldConfirmOverwrite("program", n, func(name string) bool {
					_, err := repositories.ProgramRepo().Get(name)
					return err == nil
				}, activeWire.Window, applyProgramUpdate) {
					return
				}
			}
			applyProgramUpdate()
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
				p := shell().EditorTabs.ItemsTab.ProgramSelector.Selected
				program, err := repositories.ProgramRepo().Get(p)
				if err != nil {
					log.Printf("Error getting program %s: %v", p, err)
					return
				}

				if err := program.ItemRepo().Set(v.Name, v); err != nil {
					log.Printf("Error saving item %s: %v", v.Name, err)
					activeWire.ShowErrorWithEscape(errors.New("failed to save item"), activeWire.Window)
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
			p := shell().EditorTabs.ItemsTab.ProgramSelector.Selected
			if p == "" {
				activeWire.ShowErrorWithEscape(errors.New("program cannot be empty"), activeWire.Window)
				return
			}
			program, err := repositories.ProgramRepo().Get(p)
			if err != nil {
				log.Printf("Error getting program %s: %v", p, err)
				return
			}

			applyItemUpdate := func() {
				oldItemName := v.Name

				if v.Name != n {
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
						activeWire.ShowErrorWithEscape(errors.New("failed to update item name"), activeWire.Window)
						return
					}
				}

				v.Name = n
				v.GridSize = [2]int{x, y}
				v.StackMax = sm

				// Save the item with the new name
				if err := program.ItemRepo().Set(v.Name, v); err != nil {
					log.Printf("Error saving item %s: %v", v.Name, err)
					activeWire.ShowErrorWithEscape(errors.New("failed to save item"), activeWire.Window)
					return
				}

				if err := repositories.ProgramRepo().Set(program.Name, program); err != nil {
					log.Printf("Error saving program %s: %v", p, err)
					return
				}

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

			if v.Name != n {
				if shouldConfirmOverwrite("item", n, func(name string) bool {
					_, err := program.ItemRepo().Get(name)
					return err == nil
				}, activeWire.Window, applyItemUpdate) {
					return
				}
			}
			applyItemUpdate()
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
			p := shell().EditorTabs.PointsTab.ProgramSelector.Selected
			if p == "" {
				activeWire.ShowErrorWithEscape(errors.New("program cannot be empty"), activeWire.Window)
				return
			}
			program, err := repositories.ProgramRepo().Get(p)
			if err != nil {
				log.Printf("Error getting program %s: %v", p, err)
				return
			}
			applyPointUpdate := func() {
				oldkey := v.Name
				v.Name = n
				v.X = xVal
				v.Y = yVal

				if err := program.PointRepo(config.MainMonitorSizeString).Set(v.Name, v); err != nil {
					log.Printf("Error saving point %s: %v", v.Name, err)
					activeWire.ShowErrorWithEscape(errors.New("failed to save point"), activeWire.Window)
					return
				}

				if oldkey != v.Name {
					if err := program.PointRepo(config.MainMonitorSizeString).Delete(oldkey); err != nil {
						log.Printf("Error deleting point %s: %v", oldkey, err)
						activeWire.ShowErrorWithEscape(errors.New("failed to delete point"), activeWire.Window)
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
					shell().UpdatePointPreview(v)
				}()

				if acc, ok := et.PointsTab.Widgets["Accordion"].(*widget.Accordion); ok {
					refreshPointsAccordionProgramRow(acc, p)
				}
				markPointsClean()
			}

			if v.Name != n {
				if shouldConfirmOverwrite("point", n, func(name string) bool {
					_, err := program.PointRepo(config.MainMonitorSizeString).Get(name)
					return err == nil
				}, activeWire.Window, applyPointUpdate) {
					return
				}
			}
			applyPointUpdate()
		}
	}

	// Set up record button handler for Points tab
	if recordButton, ok := et.PointsTab.Widgets["recordButton"].(*widget.Button); ok {
		recordButton.OnTapped = func() {
			var dismissOverlay func()
			dismissOverlay = activeWire.ShowRecordingOverlay(
				nil,
				func(ev *desktop.MouseEvent) {
					fyne.DoAndWait(func() {
						switch ev.Button {
						case desktop.MouseButtonPrimary:
							x, y := sqdesktop.Default.Location()
							custom_widgets.SetEntryText(et.PointsTab.Widgets["X"], strconv.Itoa(x))
							custom_widgets.SetEntryText(et.PointsTab.Widgets["Y"], strconv.Itoa(y))
							dismissOverlay()
							if v, ok := et.PointsTab.SelectedItem.(*models.Point); ok {
								func() {
									defer func() {
										if r := recover(); r != nil {
											services.LogPanicToFile(r, "Point: Preview update after record (point: "+v.Name+")")
										}
									}()
									shell().UpdatePointPreview(&models.Point{Name: v.Name, X: x, Y: y})
								}()
							}
						default:
							dismissOverlay()
						}
					})
				},
			)
		}
	}

	// Set up record button handler for Search Areas tab (two clicks: top-left, then bottom-right)
	if saRecordButton, ok := et.SearchAreasTab.Widgets["recordButton"].(*widget.Button); ok {
		saRecordButton.OnTapped = func() {
			stopPoll := make(chan struct{})
			var stopOnce sync.Once
			stopPolling := func() { stopOnce.Do(func() { close(stopPoll) }) }

			var mu sync.Mutex
			leftX, topY := 0, 0
			firstClickDone := false

			var dismissOverlay func()
			var setSelectionRect func(leftX, topY, rightX, bottomY int)
			dismissOverlay, setSelectionRect = activeWire.ShowSearchAreaRecordingOverlay(
				func() {
					stopPolling()
				},
				func(ev *desktop.MouseEvent) {
					fyne.DoAndWait(func() {
						if ev.Button != desktop.MouseButtonPrimary {
							dismissOverlay()
							return
						}
						adjX, adjY := sqdesktop.Default.Location()
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
						custom_widgets.SetEntryText(et.SearchAreasTab.Widgets["LeftX"], strconv.Itoa(leftX))
						custom_widgets.SetEntryText(et.SearchAreasTab.Widgets["TopY"], strconv.Itoa(topY))
						custom_widgets.SetEntryText(et.SearchAreasTab.Widgets["RightX"], strconv.Itoa(rightX))
						custom_widgets.SetEntryText(et.SearchAreasTab.Widgets["BottomY"], strconv.Itoa(bottomY))
						dismissOverlay()
						if v, ok := et.SearchAreasTab.SelectedItem.(*models.SearchArea); ok {
							func() {
								defer func() {
									if r := recover(); r != nil {
										services.LogPanicToFile(r, "SearchArea: Preview update after record (area: "+v.Name+")")
									}
								}()
								shell().UpdateSearchAreaPreview(&models.SearchArea{
									Name:    v.Name,
									LeftX:   leftX,
									TopY:    topY,
									RightX:  rightX,
									BottomY: bottomY,
								})
							}()
						}
					})
				},
			)

			services.GoSafe(func() {
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
								x, y := sqdesktop.Default.Location()
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
			p := shell().EditorTabs.SearchAreasTab.ProgramSelector.Selected
			if p == "" {
				activeWire.ShowErrorWithEscape(errors.New("program cannot be empty"), activeWire.Window)
				return
			}
			program, err := repositories.ProgramRepo().Get(p)
			if err != nil {
				log.Printf("Error getting program %s: %v", p, err)
				return
			}
			applySearchAreaUpdate := func() {
				oldkey := v.Name
				v.Name = n
				v.LeftX = lxVal
				v.TopY = tyVal
				v.RightX = rxVal
				v.BottomY = byVal

				if err := program.SearchAreaRepo(config.MainMonitorSizeString).Set(v.Name, v); err != nil {
					log.Printf("Error saving search area %s: %v", v.Name, err)
					activeWire.ShowErrorWithEscape(errors.New("failed to save search area"), activeWire.Window)
					return
				}
				if oldkey != v.Name {
					if err := program.SearchAreaRepo(config.MainMonitorSizeString).Delete(oldkey); err != nil {
						log.Printf("Error deleting search area %s: %v", oldkey, err)
						activeWire.ShowErrorWithEscape(errors.New("failed to delete search area"), activeWire.Window)
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
					shell().UpdateSearchAreaPreview(v)
				}()
				refreshEditorSearchAreaAccordionsForProgram(p)
				markSearchAreasClean()
			}

			if v.Name != n {
				if shouldConfirmOverwrite("search area", n, func(name string) bool {
					_, err := program.SearchAreaRepo(config.MainMonitorSizeString).Get(name)
					return err == nil
				}, activeWire.Window, applySearchAreaUpdate) {
					return
				}
			}
			applySearchAreaUpdate()
		}
	}

}

func shouldConfirmOverwrite(entityType, targetName string, existsFn func(name string) bool, parent fyne.Window, onConfirm func()) bool {
	if !existsFn(targetName) {
		return false
	}
	activeWire.ShowConfirmWithEscape(
		"Confirm Overwrite",
		fmt.Sprintf("A %s named \"%s\" already exists. Overwrite it?", entityType, targetName),
		func(confirmed bool) {
			if !confirmed {
				return
			}
			onConfirm()
		},
		parent,
	)
	return true
}

// getOrCreateProgram retrieves a program by name or creates it if it doesn't exist.
func getOrCreateProgram(pn string) *models.Program {
	pro, err := repositories.ProgramRepo().Get(pn)
	if err != nil {
		pro = repositories.ProgramRepo().New()
		pro.Name = pn
		if err := repositories.ProgramRepo().Set(pro.Name, pro); err != nil {
			activeWire.ShowErrorWithEscape(err, activeWire.Window)
			return nil
		}
		log.Println("editor binder: new program created", pn)
		setEditorLists()
	}
	return pro
}

// getSelectedEntityName returns the display name of the currently selected entity on the active tab.
func getSelectedEntityName() string {
	et := shell().EditorTabs
	switch shell().EditorTabs.Selected().Text {
	case "Programs":
		if v, ok := et.ProgramsTab.SelectedItem.(*models.Program); ok {
			return v.Name
		}
	case "Items":
		if v, ok := et.ItemsTab.SelectedItem.(*models.Item); ok {
			return v.Name
		}
	case "Points":
		if v, ok := et.PointsTab.SelectedItem.(*models.Point); ok {
			return v.Name
		}
	case "Search Areas":
		if v, ok := et.SearchAreasTab.SelectedItem.(*models.SearchArea); ok {
			return v.Name
		}
	case "Masks":
		if v, ok := et.MasksTab.SelectedItem.(*models.Mask); ok {
			return v.Name
		}
	}
	return ""
}

// parseIntOrString attempts to parse s as an int; if it fails, returns s as-is.
func parseIntOrString(s string) any {
	if v, err := strconv.Atoi(s); err == nil {
		return v
	}
	return s
}

func setEditorButtons() {
	shell().AddButton.OnTapped = func() {
		var cfg createDialogConfig
		switch shell().EditorTabs.Selected().Text {
		case "Programs":
			cfg = programCreateConfig()
		case "Items":
			cfg = itemCreateConfig()
		case "Points":
			cfg = pointCreateConfig()
		case "Masks":
			cfg = maskCreateConfig()
		case "Search Areas":
			cfg = searchAreaCreateConfig()
		default:
			return
		}
		showCreateDialog(cfg, activeWire.Window)
	}
	shell().RemoveButton.OnTapped = func() {
		tabName := shell().EditorTabs.Selected().Text
		entityName := getSelectedEntityName()
		if entityName == "" {
			return
		}

		activeWire.ShowConfirmWithEscape(
			"Confirm Delete",
			fmt.Sprintf("Are you sure you want to delete %s \"%s\"?",
				strings.ToLower(tabName), entityName),
			func(confirmed bool) {
				if !confirmed {
					return
				}
				performDeleteForTab()
			},
			activeWire.Window,
		)
	}

}
