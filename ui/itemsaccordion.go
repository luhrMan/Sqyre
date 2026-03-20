package ui

import (
	"Sqyre/internal/assets"
	"Sqyre/internal/config"
	"Sqyre/internal/models"
	"Sqyre/internal/services"
	"fmt"
	"image/color"
	"slices"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/canvas"
	"fyne.io/fyne/v2/container"
	"fyne.io/fyne/v2/theme"
	"fyne.io/fyne/v2/widget"
	ttwidget "github.com/dweymouth/fyne-tooltip/widget"
	"github.com/lithammer/fuzzysearch/fuzzy"
	"Sqyre/ui/custom_widgets"
)

// HeaderButtonOption describes an optional button shown at the right end of an accordion item's
// header area (rendered as the top-right of the item content, since Fyne's accordion header is title-only).
type HeaderButtonOption struct {
	Label    string
	Tooltip  string // optional
	OnTapped func()
}

// ItemsAccordionOptions configures how the program items accordion behaves.
// Used by both the editor Items tab and the Image Search action dialog.
type ItemsAccordionOptions struct {
	Program *models.Program

	// FilterText is the tab-level search filter; applied to baseNames (and tags). Empty = no filter.
	FilterText string

	// GetSelectedTargets returns the current list of selected item full names (Program|BaseName)
	// for highlighting. Return nil or empty to hide selection highlight.
	GetSelectedTargets func() []string

	// OnItemSelected is called when the user selects an item in the grid.
	// baseItemName is the base name (no variant). Full name is Program.Name + delim + baseItemName.
	OnItemSelected func(baseItemName string)

	// RegisterWidgets, if non-nil, is called with the grid so the editor can store it (e.g. programName+"-list").
	RegisterWidgets func(programName string, list *widget.GridWrap)

	// OnSelectionChanged is called when the user uses "select all visible" / "deselect all visible".
	// It receives the new full list of selected target full names. If nil, the select-all button is hidden.
	OnSelectionChanged func(newTargets []string)

	// AllButtonInHeader, when true, returns the tri-state (empty/half/full) control as the second return value
	// for use in the accordion header row. Requires OnSelectionChanged != nil.
	AllButtonInHeader bool

	// OnSelectionMaybeChanged, if set, is called when selection might have changed (e.g. user clicked an item)
	// so the accordion can refresh and update tri-state displays.
	OnSelectionMaybeChanged func()

	// RegisterRefreshTarget, if non-nil, is called with the item grid so the builder can refresh it when selection changes (e.g. from the Selected Items preview).
	RegisterRefreshTarget func(grid *widget.GridWrap)

	// HeaderButton, if non-nil, adds a button at the right end of this accordion item's header area.
	HeaderButton *HeaderButtonOption
}

// CreateProgramAccordionItem builds a single accordion item (one program's item grid)
// with shared behavior: icon cache, search (including tag search), selection highlight,
// and selection callback. Use the same code path for editor and action dialog.
// When AllButtonInHeader is true, the second return value is the "All" button for use in the header row; otherwise it is nil.
func CreateProgramAccordionItem(opts ItemsAccordionOptions) (*widget.AccordionItem, fyne.CanvasObject) {
	if opts.Program == nil {
		return widget.NewAccordionItem("", container.NewStack()), nil
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

	// Apply tab-level filter to baseNames (by name and tags)
	filtered := baseNames
	if opts.FilterText != "" {
		filtered = []string{}
		for _, baseName := range baseNames {
			if fuzzy.MatchFold(opts.FilterText, baseName) {
				filtered = append(filtered, baseName)
				continue
			}
			itemName, exists := baseNameToItemName[baseName]
			if !exists {
				itemName = baseName
			}
			item, err := program.ItemRepo().Get(itemName)
			if err == nil {
				for _, tag := range item.Tags {
					if fuzzy.MatchFold(opts.FilterText, tag) {
						filtered = append(filtered, baseName)
						break
					}
				}
			}
		}
		slices.Sort(filtered)
	}

	lists := struct {
		items    *widget.GridWrap
		filtered []string
	}{
		filtered: filtered,
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
		if opts.OnSelectionMaybeChanged != nil {
			opts.OnSelectionMaybeChanged()
		}
	}

	// Optional tri-state (empty/half/full) or legacy "All" button: in content row (editor) or returned for header (action dialog).
	var contentTop fyne.CanvasObject
	var allButtonForHeader fyne.CanvasObject
	if opts.OnSelectionChanged != nil {
		doSelectAllToggle := func() {
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
		}
		getState := func() int {
			current := []string{}
			if opts.GetSelectedTargets != nil {
				current = opts.GetSelectedTargets()
			}
			filteredFull := make([]string, 0, len(lists.filtered))
			for _, baseName := range lists.filtered {
				filteredFull = append(filteredFull, programName+config.ProgramDelimiter+baseName)
			}
			if len(filteredFull) == 0 {
				return 0
			}
			selected := 0
			for _, full := range filteredFull {
				if slices.Contains(current, full) {
					selected++
				}
			}
			if selected == 0 {
				return 0
			}
			if selected == len(filteredFull) {
				return 2
			}
			return 1
		}
		if opts.AllButtonInHeader {
			triState := custom_widgets.NewTriStateSelectAll(getState, doSelectAllToggle)
			allButtonForHeader = triState
		} else {
			selectAllBtn := ttwidget.NewButton("All", doSelectAllToggle)
			selectAllBtn.Importance = widget.MediumImportance
			selectAllBtn.SetToolTip("Select all visible items, or deselect all if all visible are selected")
			contentTop = container.NewBorder(nil, nil, nil, selectAllBtn, nil)
		}
	}
	if opts.HeaderButton != nil {
		hb := opts.HeaderButton
		headerBtn := ttwidget.NewButton(hb.Label, hb.OnTapped)
		if hb.Tooltip != "" {
			headerBtn.SetToolTip(hb.Tooltip)
		}
		headerBtn.Importance = widget.MediumImportance
		headerButtonRow := container.NewBorder(nil, nil, nil, headerBtn, nil)
		if contentTop != nil {
			contentTop = container.NewVBox(headerButtonRow, contentTop)
		} else {
			contentTop = headerButtonRow
		}
	}

	if opts.RegisterWidgets != nil {
		opts.RegisterWidgets(programName, lists.items)
	}
	if opts.RegisterRefreshTarget != nil {
		opts.RegisterRefreshTarget(lists.items)
	}

	item := widget.NewAccordionItem(
		fmt.Sprintf("%s (%d)", programName, len(lists.filtered)),
		container.NewBorder(
			contentTop,
			nil, nil, nil,
			lists.items,
		),
	)
	return item, allButtonForHeader
}
