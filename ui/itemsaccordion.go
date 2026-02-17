package ui

import (
	"Squire/internal/assets"
	"Squire/internal/config"
	"Squire/internal/models"
	"Squire/internal/services"
	"image/color"
	"slices"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/canvas"
	"fyne.io/fyne/v2/container"
	"fyne.io/fyne/v2/theme"
	"fyne.io/fyne/v2/widget"
	ttwidget "github.com/dweymouth/fyne-tooltip/widget"
	"github.com/lithammer/fuzzysearch/fuzzy"
)

// ItemsAccordionOptions configures how the program items accordion behaves.
// Used by both the editor Items tab and the Image Search action dialog.
type ItemsAccordionOptions struct {
	Program *models.Program

	// GetSelectedTargets returns the current list of selected item full names (Program|BaseName)
	// for highlighting. Return nil or empty to hide selection highlight.
	GetSelectedTargets func() []string

	// OnItemSelected is called when the user selects an item in the grid.
	// baseItemName is the base name (no variant). Full name is Program.Name + delim + baseItemName.
	OnItemSelected func(baseItemName string)

	// RegisterWidgets, if non-nil, is called with the searchbar and grid so the editor
	// can store them (e.g. for program-specific search).
	RegisterWidgets func(programName string, searchbar *widget.Entry, list *widget.GridWrap)

	// OnSelectionChanged is called when the user uses "select all visible" / "deselect all visible".
	// It receives the new full list of selected target full names. If nil, the select-all button is hidden.
	OnSelectionChanged func(newTargets []string)
}

// CreateProgramAccordionItem builds a single accordion item (one program's item grid)
// with shared behavior: icon cache, search (including tag search), selection highlight,
// and selection callback. Use the same code path for editor and action dialog.
func CreateProgramAccordionItem(opts ItemsAccordionOptions) *widget.AccordionItem {
	if opts.Program == nil {
		return widget.NewAccordionItem("", container.NewStack())
	}

	program := opts.Program
	iconService := services.IconVariantServiceInstance()
	programName := program.Name

	type itemIconInfo struct {
		iconPath string
		exists   bool
	}
	iconCache := make(map[string]itemIconInfo)

	baseNames := iconService.GroupItemsByBaseName(program.ItemRepo().GetAllKeys())

	baseNameToItemName := make(map[string]string)
	for _, itemName := range program.ItemRepo().GetAllKeys() {
		baseName := iconService.GetBaseItemName(itemName)
		if _, exists := baseNameToItemName[baseName]; !exists {
			baseNameToItemName[baseName] = itemName
		}
	}

	for _, baseName := range baseNames {
		cacheKey := programName + config.ProgramDelimiter + baseName
		variants, err := iconService.GetVariants(programName, baseName)
		if err == nil && len(variants) > 0 {
			var selectedVariant string
			for _, variant := range variants {
				if variant == "Original" {
					selectedVariant = variant
					break
				}
			}
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

	lists := struct {
		searchbar *widget.Entry
		items     *widget.GridWrap
		filtered  []string
	}{
		searchbar: widget.NewEntry(),
		filtered:  baseNames,
	}

	lists.items = widget.NewGridWrap(
		func() int { return len(lists.filtered) },
		func() fyne.CanvasObject {
			rect := canvas.NewRectangle(color.RGBA{})
			rect.SetMinSize(fyne.NewSquareSize(75))
			rect.CornerRadius = 5
			icon := canvas.NewImageFromResource(theme.BrokenImageIcon())
			icon.SetMinSize(fyne.NewSquareSize(70))
			icon.FillMode = canvas.ImageFillOriginal
			return container.NewStack(rect, container.NewPadded(icon), ttwidget.NewLabel(""))
		},
		func(id widget.GridWrapItemID, o fyne.CanvasObject) {
			baseItemName := lists.filtered[id]
			stack := o.(*fyne.Container)
			rect := stack.Objects[0].(*canvas.Rectangle)
			icon := stack.Objects[1].(*fyne.Container).Objects[0].(*canvas.Image)
			tt := stack.Objects[2].(*ttwidget.Label)
			tt.SetToolTip(baseItemName)

			var t []string
			if opts.GetSelectedTargets != nil {
				t = opts.GetSelectedTargets()
			}
			fullItemName := programName + config.ProgramDelimiter + baseItemName
			if slices.Contains(t, fullItemName) {
				rect.FillColor = color.RGBA{R: 0, G: 128, B: 0, A: 128}
			} else {
				rect.FillColor = color.RGBA{}
			}

			cacheKey := programName + config.ProgramDelimiter + baseItemName
			if iconInfo, exists := iconCache[cacheKey]; exists {
				if resource := assets.GetFyneResource(iconInfo.iconPath); resource != nil {
					newIcon := canvas.NewImageFromResource(resource)
					newIcon.SetMinSize(fyne.NewSquareSize(40))
					newIcon.FillMode = canvas.ImageFillOriginal
					iconContainer := stack.Objects[1].(*fyne.Container)
					iconContainer.Objects[0] = newIcon
				} else {
					icon.Resource = assets.AppIcon
				}
			} else {
				icon.Resource = assets.AppIcon
			}
			o.Refresh()
		},
	)

	lists.items.OnSelected = func(id widget.GridWrapItemID) {
		baseItemName := lists.filtered[id]
		if opts.OnItemSelected != nil {
			opts.OnItemSelected(baseItemName)
		}
		lists.items.UnselectAll()
	}

	lists.searchbar.PlaceHolder = "Search here"
	lists.searchbar.OnChanged = func(s string) {
		defaultList := iconService.GroupItemsByBaseName(program.ItemRepo().GetAllKeys())
		if s == "" {
			lists.filtered = defaultList
		} else {
			lists.filtered = []string{}
			for _, baseName := range defaultList {
				if fuzzy.MatchFold(s, baseName) {
					lists.filtered = append(lists.filtered, baseName)
					continue
				}
				itemName, exists := baseNameToItemName[baseName]
				if exists {
					item, err := program.ItemRepo().Get(itemName)
					if err == nil {
						for _, tag := range item.Tags {
							if fuzzy.MatchFold(s, tag) {
								lists.filtered = append(lists.filtered, baseName)
								break
							}
						}
					}
				} else {
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
		}
		lists.items.ScrollToTop()
		lists.items.UnselectAll()
		lists.items.Refresh()
	}

	// Search bar row: entry + optional "All" button (select/deselect all visible)
	searchBarRow := container.NewBorder(nil, nil, nil, nil, lists.searchbar)
	if opts.OnSelectionChanged != nil {
		selectAllBtn := ttwidget.NewButton("All", func() {
			current := []string{}
			if opts.GetSelectedTargets != nil {
				current = opts.GetSelectedTargets()
			}
			filteredFull := make([]string, 0, len(lists.filtered))
			for _, baseName := range lists.filtered {
				filteredFull = append(filteredFull, programName+config.ProgramDelimiter+baseName)
			}
			allSelected := len(filteredFull) > 0
			for _, full := range filteredFull {
				if !slices.Contains(current, full) {
					allSelected = false
					break
				}
			}
			var newTargets []string
			if allSelected {
				newTargets = make([]string, 0, len(current))
				for _, t := range current {
					if !slices.Contains(filteredFull, t) {
						newTargets = append(newTargets, t)
					}
				}
			} else {
				seen := make(map[string]bool)
				for _, t := range current {
					seen[t] = true
				}
				for _, full := range filteredFull {
					seen[full] = true
				}
				newTargets = make([]string, 0, len(seen))
				for t := range seen {
					newTargets = append(newTargets, t)
				}
			}
			slices.Sort(newTargets)
			opts.OnSelectionChanged(newTargets)
			lists.items.Refresh()
		})
		selectAllBtn.Importance = widget.MediumImportance
		selectAllBtn.SetToolTip("Select all visible items, or deselect all if all visible are selected")
		searchBarRow = container.NewBorder(nil, nil, nil, selectAllBtn, lists.searchbar)
	}

	if opts.RegisterWidgets != nil {
		opts.RegisterWidgets(programName, lists.searchbar, lists.items)
	}

	return widget.NewAccordionItem(
		programName,
		container.NewBorder(
			searchBarRow,
			nil, nil, nil,
			lists.items,
		),
	)
}
