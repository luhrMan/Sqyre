package binders

import (
	"Squire/internal/assets"
	"Squire/internal/config"
	"Squire/internal/models"
	"Squire/internal/models/actions"
	"Squire/internal/models/repositories"
	"Squire/internal/services"
	"Squire/ui"
	"Squire/ui/custom_widgets"
	"image/color"
	"log"
	"slices"
	"sort"
	"strconv"
	"strings"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/canvas"
	"fyne.io/fyne/v2/container"
	"fyne.io/fyne/v2/theme"
	"fyne.io/fyne/v2/widget"
	xwidget "fyne.io/x/fyne/widget"
	ttwidget "github.com/dweymouth/fyne-tooltip/widget"
	"github.com/lithammer/fuzzysearch/fuzzy"
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
	if tagEntry, ok := it["tagEntry"].(*xwidget.CompletionEntry); ok {
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

// createProgramAccordionItem creates an accordion item for a specific program with icon cache
func createProgramAccordionItem(program *models.Program) *widget.AccordionItem {
	var (
		// ats         = ui.GetUi().ActionTabs
		iconService = services.IconVariantServiceInstance()
	)

	// Pre-cache variant information for this specific program
	type itemIconInfo struct {
		iconPath string
		exists   bool
	}
	iconCache := make(map[string]itemIconInfo)

	// Pre-compute icon paths and item mappings for this program
	baseNames := groupItemsByBaseName(program.ItemRepo().GetAllKeys(), iconService)

	// Build base name to full item name mapping for fast lookup
	baseNameToItemName := make(map[string]string)
	allItems := program.ItemRepo().GetAllKeys()
	for _, itemName := range allItems {
		baseName := iconService.GetBaseItemName(itemName)
		if _, exists := baseNameToItemName[baseName]; !exists {
			// Store first variant found for this base name
			baseNameToItemName[baseName] = itemName
		}
	}

	// Create program-specific cache to avoid collisions
	programName := program.Name
	for _, baseName := range baseNames {
		cacheKey := programName + "|" + baseName
		variants, err := iconService.GetVariants(programName, baseName)
		if err == nil && len(variants) > 0 {
			// Always use "Original" variant for the item grid
			var selectedVariant string
			for _, variant := range variants {
				if variant == "Original" {
					selectedVariant = variant
					break
				}
			}

			// If "Original" not found, fall back to first variant (shouldn't happen with new system)
			if selectedVariant == "" {
				selectedVariant = variants[0]
			}

			path := programName + config.ProgramDelimiter + baseName
			if selectedVariant != "" {
				path = path + config.ProgramDelimiter + selectedVariant
			}
			path = path + config.PNG
			iconCache[cacheKey] = itemIconInfo{iconPath: path, exists: true}
		}
	}

	// Create the accordion item content
	lists := struct {
		searchbar *widget.Entry
		items     *widget.GridWrap
		filtered  []string
	}{
		searchbar: new(widget.Entry),
		items:     new(widget.GridWrap),
		filtered:  baseNames,
	}

	lists.items = widget.NewGridWrap(
		func() int {
			return len(lists.filtered)
		},
		func() fyne.CanvasObject {
			rect := canvas.NewRectangle(color.RGBA{})
			rect.SetMinSize(fyne.NewSquareSize(75))
			rect.CornerRadius = 5

			icon := canvas.NewImageFromResource(theme.BrokenImageIcon())
			icon.SetMinSize(fyne.NewSquareSize(70))
			icon.FillMode = canvas.ImageFillOriginal

			stack := container.NewStack(rect, container.NewPadded(icon), ttwidget.NewLabel(""))
			return stack
		},
		func(id widget.GridWrapItemID, o fyne.CanvasObject) {
			baseItemName := lists.filtered[id]

			stack := o.(*fyne.Container)
			rect := stack.Objects[0].(*canvas.Rectangle)
			icon := stack.Objects[1].(*fyne.Container).Objects[0].(*canvas.Image)
			tt := stack.Objects[2].(*ttwidget.Label)
			tt.SetToolTip(baseItemName)

			// Get targets from the action node directly (bindings removed)
			var t []string
			if ui.GetUi().MainUi.Navigation.Visible() {
				if v, ok := ui.GetUi().Mui.MTabs.SelectedTab().Macro.Root.GetAction(ui.GetUi().Mui.MTabs.SelectedTab().SelectedNode).(*actions.ImageSearch); ok {
					t = v.Targets
				}
				// Check if this base item is selected (in targets)
				fullItemName := programName + config.ProgramDelimiter + baseItemName
				if slices.Contains(t, fullItemName) {
					rect.FillColor = color.RGBA{R: 0, G: 128, B: 0, A: 128}
				} else {
					rect.FillColor = color.RGBA{}
				}
			}

			// Load icon from pre-computed cache
			cacheKey := programName + "|" + baseItemName
			if iconInfo, exists := iconCache[cacheKey]; exists {
				// Create a new canvas.Image for this specific icon
				if resource := assets.GetFyneResource(iconInfo.iconPath); resource != nil {
					newIcon := canvas.NewImageFromResource(resource)
					newIcon.SetMinSize(fyne.NewSquareSize(40))
					newIcon.FillMode = canvas.ImageFillOriginal

					// Replace the icon in the container
					iconContainer := stack.Objects[1].(*fyne.Container)
					iconContainer.Objects[0] = newIcon
				} else {
					icon.Resource = assets.AppIcon
					// icon.Resource = theme.BrokenImageIcon()
				}
			} else {
				icon.Resource = assets.AppIcon
				// icon.Resource = theme.BrokenImageIcon()
			}
			o.Refresh()
		},
	)

	// Set up the item selection handler
	lists.items.OnSelected = func(id widget.GridWrapItemID) {
		program, err := repositories.ProgramRepo().Get(programName)
		if err != nil {
			log.Printf("Error getting program %s: %v", programName, err)
			return
		}
		ui.GetUi().ProgramSelector.SetText(program.Name)
		baseItemName := lists.filtered[id]

		// Use pre-computed mapping for fast lookup
		var item *models.Item
		itemName, exists := baseNameToItemName[baseItemName]
		if exists {
			item, err = program.ItemRepo().Get(itemName)
		} else {
			// Fallback: try base name directly
			item, err = program.ItemRepo().Get(baseItemName)
		}

		if err != nil {
			log.Printf("Error getting item %s: %v", baseItemName, err)
			return
		}

		ui.GetUi().EditorTabs.ItemsTab.SelectedItem = item

		// Update image search targets if in main UI
		if ui.GetUi().MainUi.Navigation.Visible() {
			if v, ok := ui.GetUi().Mui.MTabs.SelectedTab().Macro.Root.GetAction(ui.GetUi().Mui.MTabs.SelectedTab().SelectedNode).(*actions.ImageSearch); ok {
				t := v.Targets
				name := programName + config.ProgramDelimiter + item.Name
				if i := slices.Index(t, name); i != -1 {
					// Item exists, remove it
					t = slices.Delete(t, i, i+1)
				} else {
					// Item doesn't exist, add it
					t = append(t, name)
				}
				v.Targets = t
				ui.GetUi().Mui.MTabs.SelectedTab().Tree.RefreshItem(v.GetUID())
			}
			lists.items.UnselectAll()
		}

		// Update the item editor widgets
		setItemsWidgets(*item)
	}

	// Set up the search functionality
	lists.searchbar = &widget.Entry{
		PlaceHolder: "Search here",
		OnChanged: func(s string) {
			defaultList := groupItemsByBaseName(program.ItemRepo().GetAllKeys(), iconService)
			defer lists.items.ScrollToTop()
			defer lists.items.Refresh()

			if s == "" {
				lists.filtered = defaultList
				return
			}
			lists.filtered = []string{}
			for _, baseName := range defaultList {
				// Check if search term matches the base item name
				if fuzzy.MatchFold(s, baseName) {
					lists.filtered = append(lists.filtered, baseName)
					continue
				}

				// Check if search term matches any of the item's tags
				itemName, exists := baseNameToItemName[baseName]
				if exists {
					item, err := program.ItemRepo().Get(itemName)
					if err == nil {
						// Check each tag for a match
						for _, tag := range item.Tags {
							if fuzzy.MatchFold(s, tag) {
								lists.filtered = append(lists.filtered, baseName)
								break // Found a matching tag, no need to check more
							}
						}
					}
				} else {
					// Fallback: try base name directly if not in mapping
					item, err := program.ItemRepo().Get(baseName)
					if err == nil {
						for _, tag := range item.Tags {
							if fuzzy.MatchFold(s, tag) {
								lists.filtered = append(lists.filtered, baseName)
								break
							}
						}
					}
				}
			}
		},
	}

	// Update the widgets map
	ui.GetUi().EditorTabs.ItemsTab.Widgets[programName+"-searchbar"] = lists.searchbar
	ui.GetUi().EditorTabs.ItemsTab.Widgets[programName+"-list"] = lists.items

	// Create and return the accordion item
	return widget.NewAccordionItem(
		programName,
		container.NewBorder(
			lists.searchbar,
			nil, nil, nil,
			lists.items,
		),
	)
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

// groupItemsByBaseName groups items by their base name (text before ProgramDelimiter)
// to prevent duplicate entries in the filtered list. Returns a sorted list of unique
// base item names.
func groupItemsByBaseName(itemNames []string, iconService *services.IconVariantService) []string {
	baseNameMap := make(map[string]bool)

	// Extract unique base names
	for _, itemName := range itemNames {
		baseName := iconService.GetBaseItemName(itemName)
		baseNameMap[baseName] = true
	}

	// Convert map to sorted slice
	uniqueBaseNames := make([]string, 0, len(baseNameMap))
	for baseName := range baseNameMap {
		uniqueBaseNames = append(uniqueBaseNames, baseName)
	}

	// Sort alphabetically by base item name
	sort.Strings(uniqueBaseNames)

	return uniqueBaseNames
}
