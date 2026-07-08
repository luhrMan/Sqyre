package editor

import (
	"log"
	"slices"
	"strings"

	"Sqyre/internal/assets"
	"Sqyre/internal/config"
	"Sqyre/internal/models"
	"Sqyre/internal/models/actions"
	"Sqyre/internal/models/repositories"
	"Sqyre/internal/services"
	"Sqyre/ui/custom_widgets"

	"fyne.io/fyne/v2/widget"
)

func RefreshItemsAccordionItems() {
	if !IsBuilt() {
		return
	}
	RefreshItemsAccordionSelectionHighlights()
}

// RefreshItemsAccordionSelectionHighlights updates selection highlights on open item
// grids without rebuilding the entire accordion (e.g. when macro tab context changes).
func RefreshItemsAccordionSelectionHighlights() {
	if !IsBuilt() {
		return
	}
	if accordion, ok := shell().EditorTabs.ItemsTab.Widgets["Accordion"].(*custom_widgets.AccordionWithHeaderWidgets); ok {
		refreshOpenAccordionItemGrids(accordion.Items)
	}
}

// RefreshProgramAccordionItem refreshes only the accordion item for a specific program
func RefreshProgramAccordionItem(programName string) {
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
	it := accordion.Items[itemIndex]
	scrollOffset := float32(0)
	if oldGW := custom_widgets.FindGridWrap(it.Detail); oldGW != nil {
		scrollOffset = oldGW.GetScrollOffset()
	}
	newItem, header := CreateProgramAccordionItem(editorItemsAccordionOptions(program, filterText))
	wasOpen := it.Open
	it.Title = newItem.Title
	it.Detail = newItem.Detail
	it.Open = wasOpen
	accordion.UpdateHeaderAt(itemIndex, header)
	if gw := custom_widgets.FindGridWrap(it.Detail); gw != nil {
		custom_widgets.RefreshGridWrapPreservingScroll(gw)
		if scrollOffset > 0 {
			gw.ScrollToOffset(scrollOffset)
		}
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
	for _, itemName := range ProgramItemRepo(program).GetAllKeys() {
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
			item, err := ProgramItemRepo(program).Get(itemName)
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
