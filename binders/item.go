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

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/canvas"
	"fyne.io/fyne/v2/container"
	"fyne.io/fyne/v2/theme"
	"fyne.io/fyne/v2/widget"
	ttwidget "github.com/dweymouth/fyne-tooltip/widget"
	"github.com/lithammer/fuzzysearch/fuzzy"
)

func setItemsWidgets(i models.Item) {
	it := ui.GetUi().EditorTabs.ItemsTab.Widgets

	it["Name"].(*widget.Entry).SetText(i.Name)
	it["Cols"].(*widget.Entry).SetText(strconv.Itoa(i.GridSize[0]))
	it["Rows"].(*widget.Entry).SetText(strconv.Itoa(i.GridSize[1]))
	// it["Tags"].(*widget.Entry).Bind(c.(binding.String))
	it["StackMax"].(*widget.Entry).SetText(strconv.Itoa(i.StackMax))

	// Update IconVariantEditor with selected item
	if editor, ok := it["iconVariantEditor"].(*custom_widgets.IconVariantEditor); ok {
		programName := ui.GetUi().ProgramSelector.Text
		iconService := services.NewIconVariantService()
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

func RefreshItemsAccordionItems() {
	for _, ai := range ui.GetUi().ActionTabs.ImageSearchItemsAccordion.Items {
		ai.Detail.Refresh()
	}
}

// RefreshProgramAccordionItem refreshes only the accordion item for a specific program
func RefreshProgramAccordionItem(programName string) {
	for _, ai := range ui.GetUi().ActionTabs.ImageSearchItemsAccordion.Items {
		if ai.Title == programName {
			ai.Detail.Refresh()
			break // Only refresh the matching program
		}
	}
}

func setAccordionItemsLists(acc *widget.Accordion) {
	acc.Items = []*widget.AccordionItem{}

	var (
		ats         = ui.GetUi().ActionTabs
		iconService = services.NewIconVariantService()
	)
	
	// Pre-cache variant information for all items to avoid repeated filesystem I/O
	type itemIconInfo struct {
		iconPath string
		exists   bool
	}
	iconCache := make(map[string]itemIconInfo)
	
	for _, p := range repositories.ProgramRepo().GetAll() {
		// Pre-compute icon paths and item mappings for this program
		baseNames := groupItemsByBaseName(p.ItemRepo().GetAllKeys(), iconService)
		
		// Build base name to full item name mapping for fast lookup
		baseNameToItemName := make(map[string]string)
		allItems := p.ItemRepo().GetAllKeys()
		for _, itemName := range allItems {
			baseName := iconService.GetBaseItemName(itemName)
			if _, exists := baseNameToItemName[baseName]; !exists {
				// Store first variant found for this base name
				baseNameToItemName[baseName] = itemName
			}
		}
		
		// Create program-specific cache to avoid collisions
		programName := p.Name  // Capture program name for closure
		for _, baseName := range baseNames {
			cacheKey := programName + "|" + baseName
			variants, err := iconService.GetVariants(programName, baseName)
			if err == nil && len(variants) > 0 {
				// Use first variant
				path := programName + config.ProgramDelimiter + baseName
				if variants[0] != "" {
					path = path + config.ProgramDelimiter + variants[0]
				}
				path = path + config.PNG
				iconCache[cacheKey] = itemIconInfo{iconPath: path, exists: true}
			} else {
				// Fallback to legacy path
				path := programName + config.ProgramDelimiter + baseName + config.PNG
				iconCache[cacheKey] = itemIconInfo{iconPath: path, exists: true}
			}
		}
		
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
				rect.SetMinSize(fyne.NewSquareSize(45))
				rect.CornerRadius = 5

				icon := canvas.NewImageFromResource(theme.BrokenImageIcon())
				icon.SetMinSize(fyne.NewSquareSize(40))
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

				ist, _ := ats.BoundImageSearch.GetValue("Targets")
				t := ist.([]string)
				if ui.GetUi().MainUi.Visible() {
					// Check if this base item is selected (in targets)
					isSelected := false
					fullItemName := programName + config.ProgramDelimiter + baseItemName
					if slices.Contains(t, fullItemName) {
						isSelected = true
					}
					if isSelected {
						rect.FillColor = color.RGBA{R: 0, G: 128, B: 0, A: 128}
					} else {
						rect.FillColor = color.RGBA{}
					}
				}

				// Load icon from pre-computed cache (no filesystem I/O)
				cacheKey := programName + "|" + baseItemName
				if iconInfo, exists := iconCache[cacheKey]; exists {
					// Create a new canvas.Image for this specific icon
					// Don't reuse the template image as Fyne may cache decoded data
					if resource := assets.GetFyneResource(iconInfo.iconPath); resource != nil {
						newIcon := canvas.NewImageFromResource(resource)
						newIcon.SetMinSize(fyne.NewSquareSize(40))
						newIcon.FillMode = canvas.ImageFillOriginal
						
						// Replace the icon in the container
						iconContainer := stack.Objects[1].(*fyne.Container)
						iconContainer.Objects[0] = newIcon
					} else {
						icon.Resource = theme.BrokenImageIcon()
					}
				} else {
					icon.Resource = theme.BrokenImageIcon()
				}
				o.Refresh()
			},
		)
		lists.items.OnSelected = func(id widget.GridWrapItemID) {
			program, err := repositories.ProgramRepo().Get(programName)
			if err != nil {
				log.Printf("Error getting program %s: %v", programName, err)
				return
			}
			ui.GetUi().ProgramSelector.SetText(program.Name)
			baseItemName := lists.filtered[id]

			// Use pre-computed mapping for fast lookup (O(1) instead of O(n))
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
			if ui.GetUi().MainUi.Visible() {
				if v, ok := ui.GetUi().Mui.MTabs.SelectedTab().Macro.Root.GetAction(ui.GetUi().Mui.MTabs.SelectedTab().SelectedNode).(*actions.ImageSearch); ok {
					t := v.Targets
					name := programName + config.ProgramDelimiter + item.Name
					if !slices.Contains(t, name) {
						t = append(t, name)
					} else {
						i := slices.Index(t, name)
						if i != -1 {
							t = slices.Delete(t, i, i+1)
						}
					}
					v.Targets = t
					ui.GetUi().Mui.MTabs.SelectedTab().Tree.RefreshItem(v.GetUID())
				}
				lists.items.UnselectAll()
			}
			
			// Update the item editor widgets
			setItemsWidgets(*item)
		}

		lists.searchbar = &widget.Entry{
			PlaceHolder: "Search here",
			OnChanged: func(s string) {
				defaultList := groupItemsByBaseName(p.ItemRepo().GetAllKeys(), iconService)
				defer lists.items.ScrollToTop()
				defer lists.items.Refresh()

				if s == "" {
					lists.filtered = defaultList
					return
				}
				lists.filtered = []string{}
				for _, i := range defaultList {
					if fuzzy.MatchFold(s, i) {
						lists.filtered = append(lists.filtered, i)
					}
				}
			},
		}
		programItemsListWidget := *widget.NewAccordionItem(
			programName,
			container.NewBorder(
				lists.searchbar,
				nil, nil, nil,
				lists.items,
			),
		)
		ui.GetUi().EditorTabs.ItemsTab.Widgets[programName+"-searchbar"] = lists.searchbar
		ui.GetUi().EditorTabs.ItemsTab.Widgets[programName+"-list"] = lists.items

		acc.Append(&programItemsListWidget)
	}
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
