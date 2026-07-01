package editor

import (
	"Sqyre/internal/assets"
	"Sqyre/internal/config"
	"Sqyre/internal/models"
	"Sqyre/internal/models/actions"
	"Sqyre/internal/models/repositories"
	"Sqyre/internal/services"
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
	it := shell().EditorTabs.ItemsTab.Widgets

	it["Name"].(*widget.Entry).SetText(i.Name)
	it["Cols"].(*widget.Entry).SetText(strconv.Itoa(i.GridSize[0]))
	it["Rows"].(*widget.Entry).SetText(strconv.Itoa(i.GridSize[1]))
	it["StackMax"].(*widget.Entry).SetText(strconv.Itoa(i.StackMax))

	updateMaskDisplay(i.Mask)

	// Update tags display
	updateTagsDisplay(&i)

	// Update IconVariantEditor with selected item
	if editor, ok := it["iconVariantEditor"].(*custom_widgets.IconVariantEditor); ok {
		programName := shell().EditorTabs.ItemsTab.ProgramSelector.Selected
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
	shell().RefreshEditorActionBar()
}

// updateTagsDisplay updates the tags grid container with the current item's tags
func updateTagsDisplay(item *models.Item) {
	it := shell().EditorTabs.ItemsTab.Widgets
	tagsContainer, ok := it["Tags"].(*fyne.Container)
	if !ok {
		return
	}

	// Clear existing tags
	tagsContainer.Objects = []fyne.CanvasObject{}

	// Add each tag as a label with a remove button
	for _, tag := range item.Tags {
		tagsContainer.Add(newTagChip(item, tag))
	}

	tagsContainer.Refresh()
}

// newTagChip builds a single tag chip (label + remove button) for item.
func newTagChip(item *models.Item, tag string) fyne.CanvasObject {
	tagLabel := widget.NewLabel(tag)
	removeButton := widget.NewButtonWithIcon("", theme.DeleteIcon(), func() {
		removeTag(item, tag)
	})
	removeButton.Importance = widget.LowImportance
	return wrapTagChip(container.NewHBox(tagLabel, removeButton))
}

// appendTagChip adds a single chip for tag to the Items tab tags container without
// rebuilding the existing chips (used on the hot "add tag" path).
func appendTagChip(item *models.Item, tag string) {
	it := shell().EditorTabs.ItemsTab.Widgets
	tagsContainer, ok := it["Tags"].(*fyne.Container)
	if !ok {
		return
	}
	tagsContainer.Add(newTagChip(item, tag))
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
	p := shell().EditorTabs.ItemsTab.ProgramSelector.Selected
	program, err := repositories.ProgramRepo().Get(p)
	if err != nil {
		log.Printf("Error getting program %s: %v", p, err)
		return
	}

	if err := program.ItemRepo().Set(item.Name, item); err != nil {
		log.Printf("Error saving item %s: %v", item.Name, err)
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
	shell().EditorTabs.ItemsTab.SelectedItem = updatedItem
	shell().RefreshEditorActionBar()

	// Update the tags display
	updateTagsDisplay(updatedItem)

	// Refresh the tag entry's completion options to ensure deleted tags are removed from suggestions
	it := shell().EditorTabs.ItemsTab.Widgets
	if tagEntry, ok := it["tagEntry"].(*completionentry.CompletionEntry); ok {
		currentText := tagEntry.Text
		// If there's text in the entry, refresh the completion options
		if currentText != "" {
			programName := shell().EditorTabs.ItemsTab.ProgramSelector.Selected
			var item *models.Item
			if v, ok := shell().EditorTabs.ItemsTab.SelectedItem.(*models.Item); ok {
				item = v
			}
			matchingTags := tagCompletionOptions(programName, currentText, item, 10)
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
	if !IsBuilt() {
		return
	}
	// Use the complete rebuild function to ensure icon cache is updated
	RebuildItemsAccordion()
}

// RefreshProgramAccordionItem refreshes only the accordion item for a specific program
func RefreshProgramAccordionItem(programName string) {
	// Refresh both the main action tabs accordion and the editor tabs accordion
	// refreshAccordionForProgram(GetUi().ActionTabs.ImageSearchItemsAccordion, programName)

	if accordion, ok := shell().EditorTabs.ItemsTab.Widgets["Accordion"].(*custom_widgets.AccordionWithHeaderWidgets); ok {
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
	// if GetUi().ActionTabs.ImageSearchItemsAccordion != nil {
	// 	setAccordionItemsLists(GetUi().ActionTabs.ImageSearchItemsAccordion)
	// }

	// Rebuild the editor tabs accordion
	if accordion, ok := shell().EditorTabs.ItemsTab.Widgets["Accordion"].(*custom_widgets.AccordionWithHeaderWidgets); ok {
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
	if accordion, ok := shell().EditorTabs.ItemsTab.Widgets["Accordion"].(*custom_widgets.AccordionWithHeaderWidgets); ok {
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
	if et := shell().EditorTabs.ItemsTab; et.Widgets["searchbar"] != nil {
		if sb, ok := et.Widgets["searchbar"].(*widget.Entry); ok {
			filterText = sb.Text
		}
	}
	newItem, header := CreateProgramAccordionItem(editorItemsAccordionOptions(program, filterText))
	it := accordion.Items[itemIndex]
	wasOpen := it.Open
	it.Title = newItem.Title
	it.Detail = newItem.Detail
	it.Open = wasOpen
	accordion.UpdateHeaderAt(itemIndex, header)
	if gw := custom_widgets.FindGridWrap(newItem.Detail); gw != nil {
		gw.Refresh()
	}
}

func setAccordionItemsLists(acc *custom_widgets.AccordionWithHeaderWidgets) {
	et := shell().EditorTabs.ItemsTab
	filterText := ""
	if sb, ok := et.Widgets["searchbar"].(*widget.Entry); ok {
		filterText = sb.Text
		sb.OnChanged = func(string) {
			et.SearchDebouncer().Call(func() { setAccordionItemsLists(acc) })
		}
	}
	PopulateItemsSearchAccordion(acc, filterText, func(p *models.Program) ItemsAccordionOptions {
		return editorItemsAccordionOptions(p, filterText)
	})
}

// editorItemsAccordionOptions builds CreateProgramAccordionItem options for the editor tab (single-item
// pick + load form). Image Search action dialog uses the same grid/filter via PopulateItemsSearchAccordion
// with OnSelectionChanged / tri-state header instead.
func editorItemsAccordionOptions(program *models.Program, filterText string) ItemsAccordionOptions {
	programName := program.Name
	iconService := services.IconVariantServiceInstance()
	baseNameToItemName := make(map[string]string)
	for _, itemName := range program.ItemRepo().GetAllKeys() {
		baseName := iconService.GetBaseItemName(itemName)
		if _, exists := baseNameToItemName[baseName]; !exists {
			baseNameToItemName[baseName] = itemName
		}
	}

	return ItemsAccordionOptions{
		Program:    program,
		FilterText: filterText,
		GetSelectedTargets: func() []string {
			if !activeWire.NavigationVisible() {
				return nil
			}
			st := activeWire.MacroMTabs().SelectedTab()
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
			shell().EditorTabs.ItemsTab.ProgramSelector.SetSelected(program.Name)
			itemName, exists := baseNameToItemName[baseItemName]
			if !exists {
				itemName = baseItemName
			}
			item, err := program.ItemRepo().Get(itemName)
			if err != nil {
				log.Printf("Error getting item %s: %v", baseItemName, err)
				return
			}
			shell().EditorTabs.ItemsTab.SelectedItem = item
			if activeWire.NavigationVisible() {
				st := activeWire.MacroMTabs().SelectedTab()
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
			shell().EditorTabs.ItemsTab.Widgets[pname+"-list"] = list
		},
	}
}

// updateMaskDisplay updates the mask label and details label on the Items tab.
func updateMaskDisplay(maskName string) {
	it := shell().EditorTabs.ItemsTab.Widgets
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

	prog := shell().EditorTabs.ItemsTab.ProgramSelector.Selected
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

	if HasMaskImage(prog, maskName) {
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
	var hide func()

	acc := widget.NewAccordion()
	for _, p := range repositories.ProgramRepo().GetAllSortedByName() {
		programName := p.Name
		allKeys := p.MaskRepo().GetAllKeys()
		filtered := make([]string, len(allKeys))
		copy(filtered, allKeys)
		sortMaskKeysByDisplayName(p, filtered)

		searchDebounce := custom_widgets.NewDebouncer(custom_widgets.DefaultSearchDebounce)
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

			if v, ok := shell().EditorTabs.ItemsTab.SelectedItem.(*models.Item); ok {
				v.Mask = maskName

				prog := shell().EditorTabs.ItemsTab.ProgramSelector.Selected
				program, err := repositories.ProgramRepo().Get(prog)
				if err != nil {
					log.Printf("Error getting program %s: %v", prog, err)
					return
				}
				if err := program.ItemRepo().Set(v.Name, v); err != nil {
					log.Printf("Error saving item %s: %v", v.Name, err)
					return
				}

				updateMaskDisplay(maskName)
			}
			hide()
		}

		searchbar.OnChanged = func(s string) {
			searchDebounce.Call(func() {
				defaultList := p.MaskRepo().GetAllKeys()
				if s == "" {
					filtered = defaultList
				} else {
					next := make([]string, 0, len(defaultList))
					sLower := strings.ToLower(s)
					for _, k := range defaultList {
						if strings.Contains(strings.ToLower(k), sLower) {
							next = append(next, k)
						}
					}
					filtered = next
				}
				sortMaskKeysByDisplayName(p, filtered)
				custom_widgets.RefreshListPreservingScroll(maskList)
			})
		}

		acc.Append(widget.NewAccordionItem(
			fmt.Sprintf("%s (%d)", programName, len(allKeys)),
			container.NewBorder(searchbar, nil, nil, nil, maskList),
		))
	}

	closeButton := widget.NewButton("Close", func() { hide() })

	popUpContent := container.NewBorder(
		closeButton, nil, nil, nil,
		acc,
	)
	popup = widget.NewModalPopUp(popUpContent, activeWire.Window.Canvas())
	dlg := activeWire.AddPopupEscapeClose(popup, activeWire.Window)
	hide = dlg.Hide
	dlg.Resize(fyne.NewSize(400, 500))
	dlg.Show()
}

// setMaskSelectionButtons wires up the mask select and clear buttons on the Items tab.
func setMaskSelectionButtons() {
	it := shell().EditorTabs.ItemsTab.Widgets

	if btn, ok := it["maskSelectButton"].(*widget.Button); ok {
		btn.OnTapped = func() {
			if v, ok := shell().EditorTabs.ItemsTab.SelectedItem.(*models.Item); ok {
				if v.Name == "" {
					return
				}
			}
			showMaskSelectionPopup()
		}
	}

	if btn, ok := it["maskClearButton"].(*widget.Button); ok {
		btn.OnTapped = func() {
			if v, ok := shell().EditorTabs.ItemsTab.SelectedItem.(*models.Item); ok {
				v.Mask = ""

				prog := shell().EditorTabs.ItemsTab.ProgramSelector.Selected
				program, err := repositories.ProgramRepo().Get(prog)
				if err != nil {
					log.Printf("Error getting program %s: %v", prog, err)
					return
				}
				if err := program.ItemRepo().Set(v.Name, v); err != nil {
					log.Printf("Error saving item %s: %v", v.Name, err)
					return
				}

				updateMaskDisplay("")
			}
		}
	}
}

// getProgramTags collects all unique tags from items in the given program.
func getProgramTags(programName string) []string {
	if programName == "" {
		return nil
	}
	program, err := repositories.ProgramRepo().Get(programName)
	if err != nil {
		return nil
	}

	tagMap := make(map[string]bool)
	for _, itemName := range program.ItemRepo().GetAllKeys() {
		item, err := program.ItemRepo().Get(itemName)
		if err != nil {
			continue
		}
		for _, tag := range item.Tags {
			tagMap[tag] = true
		}
	}

	tags := make([]string, 0, len(tagMap))
	for tag := range tagMap {
		tags = append(tags, tag)
	}
	sort.Strings(tags)
	return tags
}

// tagCompletionOptions returns program tags matching search, excluding tags already on item.
// limit <= 0 means no limit.
func tagCompletionOptions(programName, search string, item *models.Item, limit int) []string {
	tags := getProgramTags(programName)
	if item != nil {
		tags = excludeTagsOnItem(tags, item.Tags)
	}
	if search == "" {
		if limit > 0 && len(tags) > limit {
			return tags[:limit]
		}
		return tags
	}

	searchLower := strings.ToLower(search)
	matching := make([]string, 0, len(tags))
	for _, tag := range tags {
		if strings.Contains(strings.ToLower(tag), searchLower) {
			matching = append(matching, tag)
		}
	}
	if limit > 0 && len(matching) > limit {
		return matching[:limit]
	}
	return matching
}

func excludeTagsOnItem(tags []string, onItem []string) []string {
	if len(onItem) == 0 {
		return tags
	}
	filtered := make([]string, 0, len(tags))
	for _, tag := range tags {
		if !slices.Contains(onItem, tag) {
			filtered = append(filtered, tag)
		}
	}
	return filtered
}

