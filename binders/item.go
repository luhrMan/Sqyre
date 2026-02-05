package binders

import (
	"Squire/internal/assets"
	"Squire/internal/config"
	"Squire/internal/models"
	"Squire/internal/models/actions"
	"Squire/internal/models/repositories"
	"Squire/internal/services"
	"Squire/ui"
	"Squire/ui/completionentry"
	"Squire/ui/custom_widgets"
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
		tagsContainer.Add(tagContainer)
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

	if accordion, ok := ui.GetUi().EditorTabs.ItemsTab.Widgets["Accordion"].(*widget.Accordion); ok {
		refreshAccordionForProgram(accordion, programName)
	}
}

// refreshAccordionForProgram rebuilds the accordion item for a specific program with updated icon cache
func refreshAccordionForProgram(accordion *widget.Accordion, programName string) {
	for i, item := range accordion.Items {
		if item.Title == programName {
			// Get the program and rebuild just this accordion item
			if program, err := repositories.ProgramRepo().Get(programName); err == nil {
				// Rebuild the accordion item for this specific program
				rebuildProgramAccordionItem(accordion, program, i)
			}
			break
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
	if accordion, ok := ui.GetUi().EditorTabs.ItemsTab.Widgets["Accordion"].(*widget.Accordion); ok {
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
	if accordion, ok := ui.GetUi().EditorTabs.ItemsTab.Widgets["Accordion"].(*widget.Accordion); ok {
		// Find and rebuild only the specific program's accordion item
		for i, item := range accordion.Items {
			if item.Title == programName {
				// Get the program and rebuild just this accordion item
				if program, err := repositories.ProgramRepo().Get(programName); err == nil {
					// Rebuild the accordion item for this specific program
					rebuildProgramAccordionItem(accordion, program, i)
				}
				break
			}
		}
	}
}

// rebuildProgramAccordionItem rebuilds a specific program's accordion item with updated icon cache
func rebuildProgramAccordionItem(accordion *widget.Accordion, program *models.Program, itemIndex int) {
	// Create the accordion item content using the shared function
	accordionItem := createProgramAccordionItem(program)

	// Replace the accordion item content
	accordion.Items[itemIndex].Detail = accordionItem.Detail
	accordion.Items[itemIndex].Detail.Refresh()
}

func setAccordionItemsLists(acc *widget.Accordion) {
	acc.Items = []*widget.AccordionItem{}

	for _, p := range repositories.ProgramRepo().GetAll() {
		accordionItem := createProgramAccordionItem(p)
		acc.Append(accordionItem)
	}
}

// createProgramAccordionItem creates an accordion item for the editor Items tab using shared UI.
func createProgramAccordionItem(program *models.Program) *widget.AccordionItem {
	programName := program.Name
	iconService := services.IconVariantServiceInstance()
	baseNameToItemName := make(map[string]string)
	for _, itemName := range program.ItemRepo().GetAllKeys() {
		baseName := iconService.GetBaseItemName(itemName)
		if _, exists := baseNameToItemName[baseName]; !exists {
			baseNameToItemName[baseName] = itemName
		}
	}

	return ui.CreateProgramAccordionItem(ui.ItemsAccordionOptions{
		Program: program,
		GetSelectedTargets: func() []string {
			if !ui.GetUi().MainUi.Navigation.Visible() {
				return nil
			}
			if v, ok := ui.GetUi().Mui.MTabs.SelectedTab().Macro.Root.GetAction(ui.GetUi().Mui.MTabs.SelectedTab().SelectedNode).(*actions.ImageSearch); ok {
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
				if v, ok := ui.GetUi().Mui.MTabs.SelectedTab().Macro.Root.GetAction(ui.GetUi().Mui.MTabs.SelectedTab().SelectedNode).(*actions.ImageSearch); ok {
					name := programName + config.ProgramDelimiter + baseItemName
					if i := slices.Index(v.Targets, name); i != -1 {
						v.Targets = slices.Delete(v.Targets, i, i+1)
					} else {
						v.Targets = append(v.Targets, name)
					}
					ui.GetUi().Mui.MTabs.SelectedTab().Tree.RefreshItem(v.GetUID())
				}
			}
			setItemsWidgets(*item)
		},
		RegisterWidgets: func(pname string, searchbar *widget.Entry, list *widget.GridWrap) {
			ui.GetUi().EditorTabs.ItemsTab.Widgets[pname+"-searchbar"] = searchbar
			ui.GetUi().EditorTabs.ItemsTab.Widgets[pname+"-list"] = list
		},
		OnSelectionChanged: func(newTargets []string) {
			if !ui.GetUi().MainUi.Navigation.Visible() {
				return
			}
			if v, ok := ui.GetUi().Mui.MTabs.SelectedTab().Macro.Root.GetAction(ui.GetUi().Mui.MTabs.SelectedTab().SelectedNode).(*actions.ImageSearch); ok {
				v.Targets = newTargets
				ui.GetUi().Mui.MTabs.SelectedTab().Tree.RefreshItem(v.GetUID())
			}
		},
	})
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

