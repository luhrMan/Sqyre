package ui

import (
	"Squire/internal/assets"
	"Squire/internal/config"
	"Squire/internal/models"
	"Squire/internal/models/actions"
	"Squire/internal/models/repositories"
	"Squire/internal/services"
	"Squire/ui/custom_widgets"
	"fmt"
	"image"
	"image/color"
	"slices"
	"strconv"
	"strings"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/canvas"
	"fyne.io/fyne/v2/container"
	"fyne.io/fyne/v2/dialog"
	"fyne.io/fyne/v2/layout"
	"fyne.io/fyne/v2/theme"
	"fyne.io/fyne/v2/widget"
	ttwidget "github.com/dweymouth/fyne-tooltip/widget"
	"github.com/go-vgo/robotgo"
	"github.com/lithammer/fuzzysearch/fuzzy"
	"gocv.io/x/gocv"
)

// ShowActionDialog displays a dialog for editing an action
func ShowActionDialog(action actions.ActionInterface, onSave func(actions.ActionInterface)) {
	u := GetUi()
	if u == nil {
		return
	}

	// If an action dialog is already open, hide and clear it before opening a new one
	if u.MainUi != nil && u.MainUi.ActionDialog != nil {
		// u.MainUi.ActionDialog.Hide()
		u.MainUi.ActionDialog = nil
	}

	// Create dialog content based on action type
	var content fyne.CanvasObject
	var saveFunc func()

	switch node := action.(type) {
	case *actions.Wait:
		content, saveFunc = createWaitDialogContent(node)
		content.Resize(fyne.NewSize(300, 100))
	case *actions.Move:
		content, saveFunc = createMoveDialogContent(node)
		content.Resize(fyne.NewSize(1000, 600))
	case *actions.Click:
		content, saveFunc = createClickDialogContent(node)
		content.Resize(fyne.NewSize(300, 100))
	case *actions.Key:
		content, saveFunc = createKeyDialogContent(node)
		content.Resize(fyne.NewSize(300, 100))
	case *actions.Loop:
		content, saveFunc = createLoopDialogContent(node)
		content.Resize(fyne.NewSize(300, 100))
	case *actions.ImageSearch:
		content, saveFunc = createImageSearchDialogContent(node)
		content.Resize(fyne.NewSize(1000, 1000))
	case *actions.Ocr:
		content, saveFunc = createOcrDialogContent(node)
		content.Resize(fyne.NewSize(300, 100))
	case *actions.SetVariable:
		content, saveFunc = createSetVariableDialogContent(node)
		content.Resize(fyne.NewSize(300, 100))
	case *actions.Calculate:
		content, saveFunc = createCalculateDialogContent(node)
		content.Resize(fyne.NewSize(300, 100))
	case *actions.DataList:
		content, saveFunc = createDataListDialogContent(node)
		content.Resize(fyne.NewSize(300, 100))
	case *actions.SaveVariable:
		content, saveFunc = createSaveVariableDialogContent(node)
		content.Resize(fyne.NewSize(300, 100))
	default:
		content = widget.NewLabel("Unknown action type")
		saveFunc = func() {}
	}
	// Show custom dialog with save/cancel buttons
	showCustomActionDialog(u, action, content, saveFunc, onSave)
}

func showCustomActionDialog(u *Ui, action actions.ActionInterface, content fyne.CanvasObject, saveFunc func(), onSave func(actions.ActionInterface)) {
	d := u.MainUi.ActionDialog
	saveButton := ttwidget.NewButton("Save", func() {
		saveFunc()
		if onSave != nil {
			onSave(action)
		}
		d.Hide()
		// Clear the reference on the MainUi when the dialog is closed
		if u != nil && u.MainUi != nil && u.MainUi.ActionDialog == d {
			u.MainUi.ActionDialog = nil
		}
	})
	saveButton.SetToolTip("Save changes to this action")

	cancelButton := ttwidget.NewButton("Cancel", func() {
		d.Hide()
		// Clear the reference on the MainUi when the dialog is closed
		if u != nil && u.MainUi != nil && u.MainUi.ActionDialog == d {
			u.MainUi.ActionDialog = nil
		}
	})
	cancelButton.SetToolTip("Cancel and discard changes")

	buttons := container.NewHBox(
		layout.NewSpacer(),
		cancelButton,
		saveButton,
	)

	dialogContent := container.NewBorder(
		nil,
		buttons,
		nil,
		nil,
		content,
	)

	d = dialog.NewCustomWithoutButtons("Edit Action"+" - "+action.GetType(), dialogContent, u.Window)
	// Store the dialog on MainUi so other parts of the app can reference it
	if u != nil && u.MainUi != nil {
		u.MainUi.ActionDialog = d
	}
	parentSize := u.Window.Canvas().Size()
	width := parentSize.Width - 200
	height := parentSize.Height - 200

	// Get content's preferred size
	contentMinSize := content.Size()
	// Add padding for dialog chrome: title bar (~40px), buttons (~50px), padding (~20px total)
	dialogPadding := fyne.NewSize(40, 110) // width padding, height padding
	contentPreferredSize := fyne.NewSize(
		contentMinSize.Width+dialogPadding.Width,
		contentMinSize.Height+dialogPadding.Height,
	)

	// Use the smaller of calculated window size or content preferred size
	if contentPreferredSize.Width < width {
		width = contentPreferredSize.Width
	}
	if contentPreferredSize.Height < height {
		height = contentPreferredSize.Height
	}

	// Ensure minimum size
	if width < 200 {
		width = 200
	}
	if height < 200 {
		height = 200
	}
	d.Resize(fyne.NewSize(width, height))

	d.Show()
}

// Dialog content creators - these create independent widgets for editing
func createWaitDialogContent(action *actions.Wait) (fyne.CanvasObject, func()) {
	timeEntry := widget.NewEntry()
	timeEntry.SetText(fmt.Sprintf("%d", action.Time))
	timeSlider := widget.NewSlider(0.0, 1000.0)
	timeSlider.SetValue(float64(action.Time))
	timeSlider.OnChanged = func(f float64) {
		timeEntry.SetText(fmt.Sprintf("%.0f", f))
	}
	timeEntry.OnChanged = func(s string) {
		if val, err := strconv.ParseFloat(s, 64); err == nil {
			timeSlider.SetValue(val)
		}
	}

	content := widget.NewForm(
		widget.NewFormItem("ms", container.NewGridWithColumns(2,
			timeEntry, timeSlider,
		)),
	)

	saveFunc := func() {
		if val, err := strconv.Atoi(timeEntry.Text); err == nil {
			action.Time = val
		}
	}

	return content, saveFunc
}

// pointCoordToInt returns an int for preview drawing; literal ints are used, variable refs (string) yield 0.
func pointCoordToInt(v interface{}) int {
	switch val := v.(type) {
	case int:
		return val
	case float64:
		return int(val)
	default:
		return 0
	}
}

func createMoveDialogContent(action *actions.Move) (fyne.CanvasObject, func()) {
	// Temporary storage for the selected point (only applied on save)
	tempPoint := action.Point

	// Create preview image for point preview
	pointPreviewImage := canvas.NewImageFromImage(nil)
	pointPreviewImage.FillMode = canvas.ImageFillContain
	pointPreviewImage.SetMinSize(fyne.NewSize(400, 300))

	// Label showing X and Y expression for the selected point
	coordsLabel := widget.NewLabel(fmt.Sprintf("X: %v, Y: %v", tempPoint.X, tempPoint.Y))
	updateCoordsLabel := func(point *actions.Point) {
		if point != nil {
			coordsLabel.SetText(fmt.Sprintf("X: %v, Y: %v", point.X, point.Y))
		}
	}

	// Helper function to update preview image (uses pointCoordToInt so variable refs show no marker)
	updatePreview := func(point *actions.Point) {
		if point == nil {
			pointPreviewImage.Image = nil
			pointPreviewImage.Refresh()
			return
		}

		px := pointCoordToInt(point.X)
		py := pointCoordToInt(point.Y)

		// Validate coordinates are within reasonable bounds
		screenWidth := config.MonitorWidth
		screenHeight := config.MonitorHeight

		if px < 0 || py < 0 || px > screenWidth || py > screenHeight {
			pointPreviewImage.Image = nil
			pointPreviewImage.Refresh()
			return
		}

		// Attempt to capture the full screen with error recovery
		defer func() {
			if r := recover(); r != nil {
				pointPreviewImage.Image = nil
				pointPreviewImage.Refresh()
			}
		}()

		captureImg, err := robotgo.CaptureImg(config.XOffset, config.YOffset, screenWidth, screenHeight)
		if err != nil || captureImg == nil {
			pointPreviewImage.Image = nil
			pointPreviewImage.Refresh()
			return
		}

		// Convert to gocv Mat for drawing
		mat, err := gocv.ImageToMatRGB(captureImg)
		if err != nil {
			pointPreviewImage.Image = nil
			pointPreviewImage.Refresh()
			return
		}
		defer mat.Close()

		// Draw red marker at point coordinates
		// Draw a circle with crosshair for visibility
		center := image.Point{X: px, Y: py}
		redColor := color.RGBA{R: 255, A: 255}

		// Draw circle
		gocv.Circle(&mat, center, 8, redColor, 2)

		// Draw crosshair lines
		// Horizontal line
		gocv.Line(&mat, image.Point{X: px - 15, Y: py}, image.Point{X: px + 15, Y: py}, redColor, 2)
		// Vertical line
		gocv.Line(&mat, image.Point{X: px, Y: py - 15}, image.Point{X: px, Y: py + 15}, redColor, 2)

		// Convert back to image.Image
		previewImg, err := mat.ToImage()
		if err != nil {
			pointPreviewImage.Image = nil
			pointPreviewImage.Refresh()
			return
		}

		// Update preview image
		pointPreviewImage.Image = previewImg
		pointPreviewImage.Refresh()
	}

	// Create Points accordion
	pointsAccordion := widget.NewAccordion()
	for _, p := range repositories.ProgramRepo().GetAll() {
		lists := struct {
			searchBar *widget.Entry
			points    *widget.List
			filtered  []string
		}{
			filtered: p.PointRepo(config.MainMonitorSizeString).GetAllKeys(),
		}

		lists.points = widget.NewList(
			func() int {
				return len(lists.filtered)
			},
			func() fyne.CanvasObject {
				return widget.NewLabel("template")
			},
			func(id widget.ListItemID, co fyne.CanvasObject) {
				name := lists.filtered[id]
				label := co.(*widget.Label)
				program, err := repositories.ProgramRepo().Get(p.Name)
				if err != nil {
					return
				}
				point, err := program.PointRepo(config.MainMonitorSizeString).Get(name)
				if err != nil {
					return
				}
				label.SetText(point.Name)
			},
		)

		lists.points.OnSelected = func(id widget.ListItemID) {
			program, err := repositories.ProgramRepo().Get(p.Name)
			if err != nil {
				return
			}
			pointName := lists.filtered[id]
			point, err := program.PointRepo(config.MainMonitorSizeString).Get(pointName)
			if err != nil {
				return
			}
			// Update temporary point (will be applied on save)
			tempPoint = actions.Point{
				Name: point.Name,
				X:    point.X,
				Y:    point.Y,
			}
			updateCoordsLabel(&tempPoint)
			updatePreview(&tempPoint)
			lists.points.Unselect(id)
		}

		lists.searchBar = widget.NewEntry()
		lists.searchBar.PlaceHolder = "Search here"
		lists.searchBar.OnChanged = func(s string) {
			defaultList := p.PointRepo(config.MainMonitorSizeString).GetAllKeys()
			if s == "" {
				lists.filtered = defaultList
			} else {
				lists.filtered = []string{}
				for _, i := range defaultList {
					if fuzzy.MatchFold(s, i) {
						lists.filtered = append(lists.filtered, i)
					}
				}
			}
			lists.points.UnselectAll()
			lists.points.Refresh()
		}

		pointsAccordion.Append(widget.NewAccordionItem(
			p.Name,
			container.NewBorder(
				lists.searchBar,
				nil, nil, nil,
				lists.points,
			),
		))
	}

	// Update label and preview for initial point
	updateCoordsLabel(&tempPoint)
	updatePreview(&tempPoint)

	content := container.NewVBox(
		coordsLabel,
		container.NewHSplit(
			pointsAccordion,
			pointPreviewImage,
		),
	)

	saveFunc := func() {
		// Apply temporary point changes
		action.Point = tempPoint
	}

	return content, saveFunc
}

func createClickDialogContent(action *actions.Click) (fyne.CanvasObject, func()) {
	buttonCheck := custom_widgets.NewToggle(func(b bool) {})
	buttonCheck.SetToggled(action.Button)

	content := container.NewVBox(
		container.NewHBox(
			layout.NewSpacer(),
			widget.NewLabel("left"),
			buttonCheck,
			widget.NewLabel("right"),
			layout.NewSpacer(),
		),
	)

	saveFunc := func() {
		action.Button = buttonCheck.Toggled
	}

	return content, saveFunc
}

func createKeyDialogContent(action *actions.Key) (fyne.CanvasObject, func()) {
	keySelect := widget.NewSelect([]string{"ctrl", "alt", "shift", "win"}, nil)
	keySelect.SetSelected(action.Key)
	wToggle := custom_widgets.NewToggle(func(b bool) {})
	wToggle.SetToggled(action.State)

	content := container.NewVBox(
		container.NewHBox(
			layout.NewSpacer(),
			keySelect,
			widget.NewLabel("up"),
			wToggle,
			widget.NewLabel("down"),
			layout.NewSpacer(),
		),
	)

	saveFunc := func() {
		action.Key = keySelect.Selected
		action.State = wToggle.Toggled
	}

	return content, saveFunc
}

func createLoopDialogContent(action *actions.Loop) (fyne.CanvasObject, func()) {
	nameEntry := widget.NewEntry()
	nameEntry.SetText(action.Name)
	countEntry := widget.NewEntry()
	countEntry.SetText(fmt.Sprintf("%d", action.Count))
	countSlider := widget.NewSlider(1.0, 10.0)
	countSlider.SetValue(float64(action.Count))
	countLabel := widget.NewLabel(fmt.Sprintf("%d", action.Count))
	countSlider.OnChanged = func(f float64) {
		count := int(f)
		countEntry.SetText(fmt.Sprintf("%d", count))
		countLabel.SetText(fmt.Sprintf("%d", count))
	}
	countEntry.OnChanged = func(s string) {
		if val, err := strconv.Atoi(s); err == nil && val >= 1 && val <= 10 {
			countSlider.SetValue(float64(val))
		}
	}

	content := widget.NewForm(
		widget.NewFormItem("Name:", nameEntry),
		widget.NewFormItem("Loops:", container.NewBorder(
			nil, nil, countLabel, nil, countSlider,
		)),
	)

	saveFunc := func() {
		action.Name = nameEntry.Text
		if count, err := strconv.Atoi(countEntry.Text); err == nil {
			action.Count = count
		}
	}

	return content, saveFunc
}

func createImageSearchDialogContent(action *actions.ImageSearch) (fyne.CanvasObject, func()) {
	nameEntry := widget.NewEntry()
	nameEntry.SetText(action.Name)
	rowSplitEntry := widget.NewEntry()
	rowSplitEntry.SetText(fmt.Sprintf("%d", action.RowSplit))
	colSplitEntry := widget.NewEntry()
	colSplitEntry.SetText(fmt.Sprintf("%d", action.ColSplit))
	outputXVarEntry := widget.NewEntry()
	outputXVarEntry.SetText(action.OutputXVariable)
	outputYVarEntry := widget.NewEntry()
	outputYVarEntry.SetText(action.OutputYVariable)

	// Temporary storage for changes (only applied on save)
	tempSearchArea := action.SearchArea
	tempTargets := slices.Clone(action.Targets)

	// Create Search Areas accordion
	searchAreasAccordion := widget.NewAccordion()
	for _, p := range repositories.ProgramRepo().GetAll() {
		lists := struct {
			searchbar   *widget.Entry
			searchareas *widget.List
			filtered    []string
		}{
			filtered: p.SearchAreaRepo(config.MainMonitorSizeString).GetAllKeys(),
		}

		lists.searchbar = widget.NewEntry()
		lists.searchareas = widget.NewList(
			func() int {
				return len(lists.filtered)
			},
			func() fyne.CanvasObject {
				return widget.NewLabel("template")
			},
			func(id widget.ListItemID, co fyne.CanvasObject) {
				name := lists.filtered[id]
				label := co.(*widget.Label)
				program, err := repositories.ProgramRepo().Get(p.Name)
				if err != nil {
					return
				}
				sa, err := program.SearchAreaRepo(config.MainMonitorSizeString).Get(name)
				if err != nil {
					return
				}
				label.SetText(sa.Name)
			},
		)

		lists.searchareas.OnSelected = func(id widget.ListItemID) {
			program, err := repositories.ProgramRepo().Get(p.Name)
			if err != nil {
				return
			}
			saName := lists.filtered[id]
			sa, err := program.SearchAreaRepo(config.MainMonitorSizeString).Get(saName)
			if err != nil {
				return
			}
			// Update temporary search area (will be applied on save)
			tempSearchArea = actions.SearchArea{
				Name:    sa.Name,
				LeftX:   sa.LeftX,
				TopY:    sa.TopY,
				RightX:  sa.RightX,
				BottomY: sa.BottomY,
			}
		}

		lists.searchbar.PlaceHolder = "Search here"
		lists.searchbar.OnChanged = func(s string) {
			defaultList := p.SearchAreaRepo(config.MainMonitorSizeString).GetAllKeys()
			if s == "" {
				lists.filtered = defaultList
			} else {
				lists.filtered = []string{}
				for _, i := range defaultList {
					if fuzzy.MatchFold(s, i) {
						lists.filtered = append(lists.filtered, i)
					}
				}
			}
			lists.searchareas.UnselectAll()
			lists.searchareas.Refresh()
		}

		searchAreasAccordion.Append(widget.NewAccordionItem(
			p.Name,
			container.NewBorder(
				lists.searchbar,
				nil, nil, nil,
				lists.searchareas,
			),
		))
	}

	previewSize := fyne.NewSquareSize(30)
	previewList := widget.NewGridWrap(
		func() int { return len(tempTargets) },
		func() fyne.CanvasObject {
			// Template is a container so we can replace the icon per cell (GridWrap reuses templates).
			icon := canvas.NewImageFromResource(theme.BrokenImageIcon())
			icon.SetMinSize(previewSize)
			icon.FillMode = canvas.ImageFillContain
			return container.NewStack(icon)
		},
		func(id widget.GridWrapItemID, o fyne.CanvasObject) {
			if id >= len(tempTargets) {
				return
			}
			target := tempTargets[id]
			stack := o.(*fyne.Container)
			var newIcon *canvas.Image
			if path := getIconPathForTarget(target); path != "" {
				if res := assets.GetFyneResource(path); res != nil {
					newIcon = canvas.NewImageFromResource(res)
				} else {
					newIcon = canvas.NewImageFromResource(assets.AppIcon)
				}
			} else {
				newIcon = canvas.NewImageFromResource(assets.AppIcon)
			}
			newIcon.SetMinSize(previewSize)
			newIcon.FillMode = canvas.ImageFillContain
			stack.Objects[0] = newIcon
		},
	)
	previewLabel := widget.NewLabel("Selected items:")
	previewScroll := container.NewScroll(previewList)
	previewBox := container.NewBorder(
		previewLabel, nil, nil, nil,
		previewScroll,
	)
	previewBox.Hide() // show only when there are selected items

	refreshPreview := func() {
		if len(tempTargets) == 0 {
			previewBox.Hide()
		} else {
			previewBox.Show()
			previewList.Refresh()
		}
	}
	refreshPreview() // show initial selection

	// Create Items accordion
	itemsAccordion := widget.NewAccordion()
	for _, p := range repositories.ProgramRepo().GetAll() {
		accordionItem := createProgramAccordionItem(p, &tempTargets, refreshPreview)
		itemsAccordion.Append(accordionItem)
	}

	rightPanel := container.NewBorder(
		nil, nil,
		nil, nil,
		container.NewVSplit(
			container.NewVBox(
				widget.NewForm(
					widget.NewFormItem("Name:", nameEntry),
					widget.NewFormItem("Row Split:", rowSplitEntry),
					widget.NewFormItem("Col Split:", colSplitEntry),
					widget.NewFormItem("Output X Variable:", outputXVarEntry),
					widget.NewFormItem("Output Y Variable:", outputYVarEntry),
				),
			),
			previewBox,
		),
	)

	content :=
		container.NewHSplit(
			widget.NewAccordion(
				widget.NewAccordionItem("Search Areas",
					container.NewBorder(
						nil, nil, nil, nil,
						searchAreasAccordion,
					),
				),
				widget.NewAccordionItem("Items",
					container.NewBorder(
						nil, nil, nil, nil,
						itemsAccordion,
					),
				),
			),
			rightPanel,
		)

	saveFunc := func() {
		action.Name = nameEntry.Text
		if rs, err := strconv.Atoi(rowSplitEntry.Text); err == nil {
			action.RowSplit = rs
		}
		if cs, err := strconv.Atoi(colSplitEntry.Text); err == nil {
			action.ColSplit = cs
		}
		action.OutputXVariable = outputXVarEntry.Text
		action.OutputYVariable = outputYVarEntry.Text
		// Apply temporary changes
		action.SearchArea = tempSearchArea
		action.Targets = tempTargets
		slices.Sort(action.Targets)
	}

	return content, saveFunc
}

// Helper function to group items by base name (from binders/item.go)
// createProgramAccordionItem creates an accordion item for a specific program in the action dialog context.
// onSelectionChanged is called when the user toggles an item so the preview can refresh.
func createProgramAccordionItem(program *models.Program, tempTargets *[]string, onSelectionChanged func()) *widget.AccordionItem {
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

	lists := struct {
		searchbar *widget.Entry
		items     *widget.GridWrap
		filtered  []string
	}{
		filtered: baseNames,
	}

	lists.searchbar = widget.NewEntry()
	lists.items = widget.NewGridWrap(
		func() int {
			return len(lists.filtered)
		},
		func() fyne.CanvasObject {
			rect := canvas.NewRectangle(theme.BackgroundColor())
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
			// if GetUi().MainUi.Visible() {
			// if v, ok := GetUi().Mui.MTabs.SelectedTab().Macro.Root.GetAction(GetUi().Mui.MTabs.SelectedTab().SelectedNode).(*actions.ImageSearch); ok {
			// 	t = v.Targets
			// }
			t = *tempTargets
			// Check if this base item is selected (in targets)
			fullItemName := programName + config.ProgramDelimiter + baseItemName
			if slices.Contains(t, fullItemName) {
				rect.FillColor = color.RGBA{R: 0, G: 128, B: 0, A: 128}
			} else {
				rect.FillColor = color.RGBA{}
			}
			// }
			// Load icon from pre-computed cache
			cacheKey := programName + config.ProgramDelimiter + baseItemName
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

	lists.items.OnSelected = func(id widget.GridWrapItemID) {
		baseItemName := lists.filtered[id]
		name := programName + config.ProgramDelimiter + baseItemName
		if i := slices.Index(*tempTargets, name); i != -1 {
			// Item exists, remove it
			*tempTargets = slices.Delete(*tempTargets, i, i+1)
		} else {
			// Item doesn't exist, add it
			*tempTargets = append(*tempTargets, name)
		}
		slices.Sort(*tempTargets)
		lists.items.Refresh()
		lists.items.UnselectAll()
		if onSelectionChanged != nil {
			onSelectionChanged()
		}
	}

	lists.searchbar.PlaceHolder = "Search here"
	lists.searchbar.OnChanged = func(s string) {
		defaultList := groupItemsByBaseName(program.ItemRepo().GetAllKeys(), iconService)
		if s == "" {
			lists.filtered = defaultList
		} else {
			lists.filtered = []string{}
			for _, i := range defaultList {
				if fuzzy.MatchFold(s, i) {
					lists.filtered = append(lists.filtered, i)
				}
			}
		}
		lists.items.UnselectAll()
		lists.items.Refresh()
	}

	return widget.NewAccordionItem(
		programName,
		container.NewBorder(
			lists.searchbar,
			nil, nil, nil,
			lists.items,
		),
	)
}

// getIconPathForTarget returns the Fyne resource path for a target "ProgramName|baseName".
// Returns empty string if the target format is invalid or no variant is found.
func getIconPathForTarget(target string) string {
	programName, baseName, ok := strings.Cut(target, config.ProgramDelimiter)
	if !ok || programName == "" || baseName == "" {
		return ""
	}
	iconService := services.IconVariantServiceInstance()
	variants, err := iconService.GetVariants(programName, baseName)
	if err != nil || len(variants) == 0 {
		return ""
	}
	var selectedVariant string
	for _, v := range variants {
		if v == "Original" {
			selectedVariant = v
			break
		}
	}
	if selectedVariant == "" {
		selectedVariant = variants[0]
	}
	return programName + config.ProgramDelimiter + baseName + config.ProgramDelimiter + selectedVariant + config.PNG
}

func groupItemsByBaseName(itemNames []string, iconService *services.IconVariantService) []string {
	baseNameSet := make(map[string]bool)
	for _, itemName := range itemNames {
		baseName := iconService.GetBaseItemName(itemName)
		baseNameSet[baseName] = true
	}
	baseNames := make([]string, 0, len(baseNameSet))
	for baseName := range baseNameSet {
		baseNames = append(baseNames, baseName)
	}
	slices.Sort(baseNames)
	return baseNames
}

func createOcrDialogContent(action *actions.Ocr) (fyne.CanvasObject, func()) {
	nameEntry := widget.NewEntry()
	nameEntry.SetText(action.Name)
	targetEntry := widget.NewEntry()
	targetEntry.SetText(action.Target)
	outputVarEntry := widget.NewEntry()
	outputVarEntry.SetText(action.OutputVariable)

	content := widget.NewForm(
		widget.NewFormItem("Name:", nameEntry),
		widget.NewFormItem("Text Target:", targetEntry),
		widget.NewFormItem("Output Variable:", outputVarEntry),
	)

	saveFunc := func() {
		action.Name = nameEntry.Text
		action.Target = targetEntry.Text
		action.OutputVariable = outputVarEntry.Text
	}

	return content, saveFunc
}

func createSetVariableDialogContent(action *actions.SetVariable) (fyne.CanvasObject, func()) {
	nameEntry := widget.NewEntry()
	nameEntry.SetText(action.VariableName)
	valueEntry := widget.NewEntry()
	valueEntry.SetText(fmt.Sprintf("%v", action.Value))

	content := widget.NewForm(
		widget.NewFormItem("Variable Name:", nameEntry),
		widget.NewFormItem("Value:", valueEntry),
	)

	saveFunc := func() {
		action.VariableName = nameEntry.Text
		action.Value = valueEntry.Text // Could be enhanced to parse different types
	}

	return content, saveFunc
}

func createCalculateDialogContent(action *actions.Calculate) (fyne.CanvasObject, func()) {
	exprEntry := widget.NewEntry()
	exprEntry.SetText(action.Expression)
	varEntry := widget.NewEntry()
	varEntry.SetText(action.OutputVar)

	content := widget.NewForm(
		widget.NewFormItem("Expression:", exprEntry),
		widget.NewFormItem("Output Variable:", varEntry),
	)

	saveFunc := func() {
		action.Expression = exprEntry.Text
		action.OutputVar = varEntry.Text
	}

	return content, saveFunc
}

func createDataListDialogContent(action *actions.DataList) (fyne.CanvasObject, func()) {
	sourceEntry := widget.NewMultiLineEntry()
	sourceEntry.SetText(action.Source)
	varEntry := widget.NewEntry()
	varEntry.SetText(action.OutputVar)
	isFileCheck := widget.NewCheck("Source is file path", nil)
	isFileCheck.SetChecked(action.IsFile)

	content := widget.NewForm(
		widget.NewFormItem("Source (file path or text):", sourceEntry),
		widget.NewFormItem("Output Variable:", varEntry),
		widget.NewFormItem("", isFileCheck),
	)

	saveFunc := func() {
		action.Source = sourceEntry.Text
		action.OutputVar = varEntry.Text
		action.IsFile = isFileCheck.Checked
	}

	return content, saveFunc
}

func createSaveVariableDialogContent(action *actions.SaveVariable) (fyne.CanvasObject, func()) {
	varEntry := widget.NewEntry()
	varEntry.SetText(action.VariableName)
	destEntry := widget.NewEntry()
	destEntry.SetText(action.Destination)
	appendCheck := widget.NewCheck("Append to file", nil)
	appendCheck.SetChecked(action.Append)

	content := widget.NewForm(
		widget.NewFormItem("Variable Name:", varEntry),
		widget.NewFormItem("Destination (file path or 'clipboard'):", destEntry),
		widget.NewFormItem("", appendCheck),
	)

	saveFunc := func() {
		action.VariableName = varEntry.Text
		action.Destination = destEntry.Text
		action.Append = appendCheck.Checked
	}

	return content, saveFunc
}
