package binders

import (
	"Sqyre/internal/assets"
	"Sqyre/internal/config"
	"Sqyre/internal/models"
	"Sqyre/internal/models/actions"
	"Sqyre/internal/models/repositories"
	"Sqyre/internal/services"
	"Sqyre/ui"
	"Sqyre/ui/completionentry"
	"Sqyre/ui/custom_widgets"
	"fmt"
	"log"
	"slices"
	"sort"
	"strconv"
	"strings"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/container"
	"fyne.io/fyne/v2/theme"
	"fyne.io/fyne/v2/widget"
)

func setItemsWidgets(i models.Item) {
	it := ui.GetUi().EditorTabs.ItemsTab.Widgets

	it["Name"].(*widget.Entry).SetText(i.Name)
	it["Cols"].(*widget.Entry).SetText(strconv.Itoa(i.GridSize[0]))
	it["Rows"].(*widget.Entry).SetText(strconv.Itoa(i.GridSize[1]))
	it["StackMax"].(*widget.Entry).SetText(strconv.Itoa(i.StackMax))

	updateMaskDisplay(i.Mask)

	// Update tags display
	updateTagsDisplay(&i)

	// Update IconVariantEditor with selected item
	if editor, ok := it["iconVariantEditor"].(*custom_widgets.IconVariantEditor); ok {
		programName := ui.GetUi().ProgramSelector.Text
		iconService := services.IconVariantServiceInstance()
		baseName := iconService.GetBaseItemName(i.Name)

		// Set variant change callback - only refresh when variants actually change
		editor.SetOnVariantChange(func() {
			// Only refresh the specific program's accordion item, not all items
			RefreshProgramAccordionItem(programName)
		})

		// Update both program and item at once to avoid double refresh
		editor.SetProgramAndItem(programName, baseName)
	}
}

// updateTagsDisplay updates the tags grid container with the current item's tags
func updateTagsDisplay(item *models.Item) {
	it := ui.GetUi().EditorTabs.ItemsTab.Widgets
	tagsContainer, ok := it["Tags"].(*fyne.Container)
	if !ok {
		return
	}

	// Clear existing tags
	tagsContainer.Objects = []fyne.CanvasObject{}

	// Add each tag as a label with a remove button
	for _, tag := range item.Tags {
		tagLabel := widget.NewLabel(tag)
		removeButton := widget.NewButtonWithIcon("", theme.DeleteIcon(), func() {
			removeTag(item, tag)
		})
		removeButton.Importance = widget.LowImportance

		// Create horizontal container for tag label and remove button
		tagContainer := container.NewHBox(tagLabel, removeButton)
		tagsContainer.Add(ui.WrapTagChip(tagContainer))
	}

	tagsContainer.Refresh()
}

// removeTag removes a tag from the current item and saves it
func removeTag(item *models.Item, tagToRemove string) {
	// Remove the tag from the slice
	newTags := []string{}
	for _, tag := range item.Tags {
		if tag != tagToRemove {
			newTags = append(newTags, tag)
		}
	}
	item.Tags = newTags

	// Save the item
	p := ui.GetUi().ProgramSelector.Text
	program, err := repositories.ProgramRepo().Get(p)
	if err != nil {
		log.Printf("Error getting program %s: %v", p, err)
		return
	}

	if err := program.ItemRepo().Set(item.Name, item); err != nil {
		log.Printf("Error saving item %s: %v", item.Name, err)
		return
	}

	if err := repositories.ProgramRepo().Set(program.Name, program); err != nil {
		log.Printf("Error saving program %s: %v", p, err)
		return
	}

	// Reload the item from the repository to ensure SelectedItem is in sync
	updatedItem, err := program.ItemRepo().Get(item.Name)
	if err != nil {
		log.Printf("Error reloading item %s: %v", item.Name, err)
		// Still update display with the modified item
		updateTagsDisplay(item)
		return
	}

	// Update the SelectedItem to the reloaded item
	ui.GetUi().EditorTabs.ItemsTab.SelectedItem = updatedItem

	// Update the tags display
	updateTagsDisplay(updatedItem)

	// Refresh the tag entry's completion options to ensure deleted tags are removed from suggestions
	it := ui.GetUi().EditorTabs.ItemsTab.Widgets
	if tagEntry, ok := it["tagEntry"].(*completionentry.CompletionEntry); ok {
		currentText := tagEntry.Text
		// If there's text in the entry, refresh the completion options
		if currentText != "" {
			// Get fresh tags and filter them
			allTags := getAllExistingTags()
			searchLower := strings.ToLower(currentText)
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
			// Update the completion options with fresh data
			tagEntry.SetOptions(matchingTags)
			if len(matchingTags) > 0 {
				tagEntry.ShowCompletion()
			} else {
				tagEntry.HideCompletion()
			}
		}
	}
}

func RefreshItemsAccordionItems() {
	// Use the complete rebuild function to ensure icon cache is updated
	RebuildItemsAccordion()
}

// RefreshProgramAccordionItem refreshes only the accordion item for a specific program
func RefreshProgramAccordionItem(programName string) {
	// Refresh both the main action tabs accordion and the editor tabs accordion
	// refreshAccordionForProgram(ui.GetUi().ActionTabs.ImageSearchItemsAccordion, programName)

	if accordion, ok := ui.GetUi().EditorTabs.ItemsTab.Widgets["Accordion"].(*custom_widgets.AccordionWithHeaderWidgets); ok {
		refreshAccordionForProgram(accordion, programName)
	}
}

// itemsAccordionRowIndexForProgram finds the row for a program. Titles are "ProgramName (n)", not bare names.
func itemsAccordionRowIndexForProgram(accordion *custom_widgets.AccordionWithHeaderWidgets, programName string) int {
	prefix := programName + " ("
	for i, item := range accordion.Items {
		if strings.HasPrefix(item.Title, prefix) {
			return i
		}
	}
	return -1
}

// refreshAccordionForProgram rebuilds the accordion item for a specific program with updated icon cache
func refreshAccordionForProgram(accordion *custom_widgets.AccordionWithHeaderWidgets, programName string) {
	if i := itemsAccordionRowIndexForProgram(accordion, programName); i >= 0 {
		if program, err := repositories.ProgramRepo().Get(programName); err == nil {
			rebuildProgramAccordionItem(accordion, program, i)
		}
	}
}

// RebuildItemsAccordion completely rebuilds the items accordion to refresh icon cache
func RebuildItemsAccordion() {
	// Rebuild the main action tabs accordion
	// if ui.GetUi().ActionTabs.ImageSearchItemsAccordion != nil {
	// 	setAccordionItemsLists(ui.GetUi().ActionTabs.ImageSearchItemsAccordion)
	// }

	// Rebuild the editor tabs accordion
	if accordion, ok := ui.GetUi().EditorTabs.ItemsTab.Widgets["Accordion"].(*custom_widgets.AccordionWithHeaderWidgets); ok {
		setAccordionItemsLists(accordion)
	}
}

// RefreshItemInGrid refreshes a specific item in the grid by invalidating its cache and forcing a grid refresh
func RefreshItemInGrid(programName, oldItemName, newItemName string) {
	iconService := services.IconVariantServiceInstance()

	// Invalidate cache for both old and new item names if they're different
	if oldItemName != newItemName {
		// Get variants for the old item name and invalidate their cache
		if oldVariants, err := iconService.GetVariants(programName, oldItemName); err == nil {
			for _, variant := range oldVariants {
				oldCacheKey := programName + config.ProgramDelimiter + oldItemName
				if variant != "" {
					oldCacheKey += config.ProgramDelimiter + variant
				}
				oldCacheKey += config.PNG
				assets.InvalidateFyneResourceCache(oldCacheKey)
			}
		}
	}

	// Invalidate cache for the new item name (or current item if name didn't change)
	if newVariants, err := iconService.GetVariants(programName, newItemName); err == nil {
		for _, variant := range newVariants {
			newCacheKey := programName + config.ProgramDelimiter + newItemName
			if variant != "" {
				newCacheKey += config.ProgramDelimiter + variant
			}
			newCacheKey += config.PNG
			assets.InvalidateFyneResourceCache(newCacheKey)
		}
	}

	// Force refresh the GridWrap by triggering a rebuild of the specific program's accordion
	// This is necessary because the GridWrap uses a pre-computed iconCache that needs to be updated
	if accordion, ok := ui.GetUi().EditorTabs.ItemsTab.Widgets["Accordion"].(*custom_widgets.AccordionWithHeaderWidgets); ok {
		if i := itemsAccordionRowIndexForProgram(accordion, programName); i >= 0 {
			if program, err := repositories.ProgramRepo().Get(programName); err == nil {
				rebuildProgramAccordionItem(accordion, program, i)
			}
		}
	}
}

// rebuildProgramAccordionItem rebuilds a specific program's accordion item with updated icon cache
func rebuildProgramAccordionItem(accordion *custom_widgets.AccordionWithHeaderWidgets, program *models.Program, itemIndex int) {
	filterText := ""
	if et := ui.GetUi().EditorTabs.ItemsTab; et.Widgets["searchbar"] != nil {
		if sb, ok := et.Widgets["searchbar"].(*widget.Entry); ok {
			filterText = sb.Text
		}
	}
	newItem, header := ui.CreateProgramAccordionItem(editorItemsAccordionOptions(program, filterText))
	it := accordion.Items[itemIndex]
	wasOpen := it.Open
	it.Title = newItem.Title
	it.Detail = newItem.Detail
	it.Open = wasOpen
	accordion.UpdateHeaderAt(itemIndex, header)
}

func setAccordionItemsLists(acc *custom_widgets.AccordionWithHeaderWidgets) {
	et := ui.GetUi().EditorTabs.ItemsTab
	filterText := ""
	if sb, ok := et.Widgets["searchbar"].(*widget.Entry); ok {
		filterText = sb.Text
		sb.OnChanged = func(string) { setAccordionItemsLists(acc) }
	}
	ui.PopulateItemsSearchAccordion(acc, filterText, func(p *models.Program) ui.ItemsAccordionOptions {
		return editorItemsAccordionOptions(p, filterText)
	})
}

// editorItemsAccordionOptions builds CreateProgramAccordionItem options for the editor tab (single-item
// pick + load form). Image Search action dialog uses the same grid/filter via PopulateItemsSearchAccordion
// with OnSelectionChanged / tri-state header instead.
func editorItemsAccordionOptions(program *models.Program, filterText string) ui.ItemsAccordionOptions {
	programName := program.Name
	iconService := services.IconVariantServiceInstance()
	baseNameToItemName := make(map[string]string)
	for _, itemName := range program.ItemRepo().GetAllKeys() {
		baseName := iconService.GetBaseItemName(itemName)
		if _, exists := baseNameToItemName[baseName]; !exists {
			baseNameToItemName[baseName] = itemName
		}
	}

	return ui.ItemsAccordionOptions{
		Program:    program,
		FilterText: filterText,
		GetSelectedTargets: func() []string {
			if !ui.GetUi().MainUi.Navigation.Visible() {
				return nil
			}
			st := ui.GetUi().Mui.MTabs.SelectedTab()
			if st == nil {
				return nil
			}
			if v, ok := st.Macro.Root.GetAction(st.SelectedNode).(*actions.ImageSearch); ok {
				return v.Targets
			}
			return nil
		},
		OnItemSelected: func(baseItemName string) {
			program, err := repositories.ProgramRepo().Get(programName)
			if err != nil {
				log.Printf("Error getting program %s: %v", programName, err)
				return
			}
			ui.GetUi().ProgramSelector.SetText(program.Name)
			itemName, exists := baseNameToItemName[baseItemName]
			if !exists {
				itemName = baseItemName
			}
			item, err := program.ItemRepo().Get(itemName)
			if err != nil {
				log.Printf("Error getting item %s: %v", baseItemName, err)
				return
			}
			ui.GetUi().EditorTabs.ItemsTab.SelectedItem = item
			if ui.GetUi().MainUi.Navigation.Visible() {
				st := ui.GetUi().Mui.MTabs.SelectedTab()
				if st != nil {
					if v, ok := st.Macro.Root.GetAction(st.SelectedNode).(*actions.ImageSearch); ok {
						name := programName + config.ProgramDelimiter + baseItemName
						if i := slices.Index(v.Targets, name); i != -1 {
							v.Targets = slices.Delete(v.Targets, i, i+1)
						} else {
							v.Targets = append(v.Targets, name)
						}
						st.Tree.RefreshItem(v.GetUID())
					}
				}
			}
			setItemsWidgets(*item)
			markItemsClean()
		},
		RegisterWidgets: func(pname string, list *widget.GridWrap) {
			ui.GetUi().EditorTabs.ItemsTab.Widgets[pname+"-list"] = list
		},
	}
}

// updateMaskDisplay updates the mask label and details label on the Items tab.
func updateMaskDisplay(maskName string) {
	it := ui.GetUi().EditorTabs.ItemsTab.Widgets
	maskLabel, _ := it["maskLabel"].(*widget.Label)
	maskDetailsLabel, _ := it["maskDetailsLabel"].(*widget.Label)

	if maskName == "" {
		if maskLabel != nil {
			maskLabel.SetText("None")
		}
		if maskDetailsLabel != nil {
			maskDetailsLabel.SetText("")
		}
		return
	}

	if maskLabel != nil {
		maskLabel.SetText(maskName)
	}

	if maskDetailsLabel == nil {
		return
	}

	prog := ui.GetUi().ProgramSelector.Text
	program, err := repositories.ProgramRepo().Get(prog)
	if err != nil {
		maskDetailsLabel.SetText("")
		return
	}

	mask, err := program.MaskRepo().Get(maskName)
	if err != nil {
		maskDetailsLabel.SetText("")
		return
	}

	if ui.HasMaskImage(prog, maskName) {
		maskDetailsLabel.SetText("Image mask")
		return
	}

	center := fmt.Sprintf("X: %s%%  Y: %s%%", mask.CenterX, mask.CenterY)
	var equation string
	switch mask.Shape {
	case "circle":
		equation = fmt.Sprintf("π × %s²", mask.Radius)
	default:
		equation = fmt.Sprintf("%s × %s", mask.Base, mask.Height)
	}
	maskDetailsLabel.SetText(fmt.Sprintf("%s  •  %s", center, equation))
}

// showMaskSelectionPopup displays a modal popup with mask accordions for the user to select a mask.
func showMaskSelectionPopup() {
	var popup *widget.PopUp

	acc := widget.NewAccordion()
	for _, p := range repositories.ProgramRepo().GetAllSortedByName() {
		programName := p.Name
		allKeys := p.MaskRepo().GetAllKeys()
		filtered := make([]string, len(allKeys))
		copy(filtered, allKeys)
		sortMaskKeysByDisplayName(p, filtered)

		searchbar := widget.NewEntry()
		searchbar.PlaceHolder = "Search masks"

		maskList := widget.NewList(
			func() int { return len(filtered) },
			func() fyne.CanvasObject { return widget.NewLabel("template") },
			func(id widget.ListItemID, co fyne.CanvasObject) {
				if id < len(filtered) {
					co.(*widget.Label).SetText(filtered[id])
				}
			},
		)

		maskList.OnSelected = func(id widget.ListItemID) {
			if id >= len(filtered) {
				return
			}
			maskName := filtered[id]

			if v, ok := ui.GetUi().EditorTabs.ItemsTab.SelectedItem.(*models.Item); ok {
				v.Mask = maskName

				prog := ui.GetUi().ProgramSelector.Text
				program, err := repositories.ProgramRepo().Get(prog)
				if err != nil {
					log.Printf("Error getting program %s: %v", prog, err)
					return
				}
				if err := program.ItemRepo().Set(v.Name, v); err != nil {
					log.Printf("Error saving item %s: %v", v.Name, err)
					return
				}
				if err := repositories.ProgramRepo().Set(program.Name, program); err != nil {
					log.Printf("Error saving program %s: %v", prog, err)
					return
				}

				updateMaskDisplay(maskName)
			}
			popup.Hide()
		}

		searchbar.OnChanged = func(s string) {
			defaultList := p.MaskRepo().GetAllKeys()
			if s == "" {
				filtered = defaultList
			} else {
				filtered = filtered[:0]
				sLower := strings.ToLower(s)
				for _, k := range defaultList {
					if strings.Contains(strings.ToLower(k), sLower) {
						filtered = append(filtered, k)
					}
				}
			}
			sortMaskKeysByDisplayName(p, filtered)
			maskList.Refresh()
			maskList.ScrollToTop()
		}

		acc.Append(widget.NewAccordionItem(
			fmt.Sprintf("%s (%d)", programName, len(allKeys)),
			container.NewBorder(searchbar, nil, nil, nil, maskList),
		))
	}

	closeButton := widget.NewButton("Close", func() { popup.Hide() })

	popUpContent := container.NewBorder(
		closeButton, nil, nil, nil,
		acc,
	)
	popup = widget.NewModalPopUp(popUpContent, ui.GetUi().Window.Canvas())
	popup.Resize(fyne.NewSize(400, 500))
	popup.Show()
}

// setMaskSelectionButtons wires up the mask select and clear buttons on the Items tab.
func setMaskSelectionButtons() {
	it := ui.GetUi().EditorTabs.ItemsTab.Widgets

	if btn, ok := it["maskSelectButton"].(*widget.Button); ok {
		btn.OnTapped = func() {
			if v, ok := ui.GetUi().EditorTabs.ItemsTab.SelectedItem.(*models.Item); ok {
				if v.Name == "" {
					return
				}
			}
			showMaskSelectionPopup()
		}
	}

	if btn, ok := it["maskClearButton"].(*widget.Button); ok {
		btn.OnTapped = func() {
			if v, ok := ui.GetUi().EditorTabs.ItemsTab.SelectedItem.(*models.Item); ok {
				v.Mask = ""

				prog := ui.GetUi().ProgramSelector.Text
				program, err := repositories.ProgramRepo().Get(prog)
				if err != nil {
					log.Printf("Error getting program %s: %v", prog, err)
					return
				}
				if err := program.ItemRepo().Set(v.Name, v); err != nil {
					log.Printf("Error saving item %s: %v", v.Name, err)
					return
				}
				if err := repositories.ProgramRepo().Set(program.Name, program); err != nil {
					log.Printf("Error saving program %s: %v", prog, err)
					return
				}

				updateMaskDisplay("")
			}
		}
	}
}

// getAllExistingTags collects all unique tags from all items across all programs
func getAllExistingTags() []string {
	tagMap := make(map[string]bool)

	for _, program := range repositories.ProgramRepo().GetAll() {
		for _, itemName := range program.ItemRepo().GetAllKeys() {
			item, err := program.ItemRepo().Get(itemName)
			if err == nil {
				for _, tag := range item.Tags {
					tagMap[tag] = true
				}
			}
		}
	}

	// Convert map to sorted slice
	tags := make([]string, 0, len(tagMap))
	for tag := range tagMap {
		tags = append(tags, tag)
	}
	sort.Strings(tags)

	return tags
}

