package ui

import (
	"Sqyre/internal/assets"
	"Sqyre/internal/config"
	"Sqyre/internal/models"
	"Sqyre/internal/models/actions"
	"Sqyre/internal/models/repositories"
	"Sqyre/internal/services"
	"Sqyre/ui/custom_widgets"
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
	fynetooltip "github.com/dweymouth/fyne-tooltip"
	ttwidget "github.com/dweymouth/fyne-tooltip/widget"
	"github.com/go-vgo/robotgo"
	"github.com/lithammer/fuzzysearch/fuzzy"
	hook "github.com/luhrMan/gohook"
	"gocv.io/x/gocv"
)

// currentMacroVariables returns all variable names defined in the currently
// selected macro. Safe to call at any time; returns nil when no macro is open.
func currentMacroVariables() []string {
	st := GetUi().Mui.MTabs.SelectedTab()
	if st == nil || st.Macro == nil {
		return nil
	}
	return st.Macro.CollectDefinedVariables()
}

// newVarEntry creates a VarEntry wired to the current macro's variables.
func newVarEntry() *custom_widgets.VarEntry {
	return custom_widgets.NewVarEntry(currentMacroVariables)
}

// newMultiLineVarEntry creates a multi-line VarEntry wired to the current macro's variables.
func newMultiLineVarEntry() *custom_widgets.VarEntry {
	return custom_widgets.NewMultiLineVarEntry(currentMacroVariables)
}

// programListAccordionConfig configures the generic program list accordion builder.
// Callbacks receive the program and item key; implementors look up the model and invoke dialog-specific logic.
type programListAccordionConfig struct {
	GetKeys        func(*models.Program) []string
	GetDisplayName func(*models.Program, string) string
	OnSelect       func(*models.Program, string)
}

// buildProgramListAccordionWithSearchbar builds an accordion of programs, each with a list of items (e.g. points or search areas).
// One searchbar above filters by program name or item key (fuzzy). Config provides key source, label text, and selection callback.
func buildProgramListAccordionWithSearchbar(cfg programListAccordionConfig) (*widget.Entry, *widget.Accordion) {
	searchbar := widget.NewEntry()
	searchbar.SetPlaceHolder("Search here")
	acc := widget.NewAccordion()
	rebuild := func() {
		filterText := searchbar.Text
		acc.Items = nil
		for _, p := range repositories.ProgramRepo().GetAll() {
			defaultList := cfg.GetKeys(p)
			filtered := defaultList
			if filterText != "" {
				filtered = nil
				for _, key := range defaultList {
					if fuzzy.MatchFold(filterText, key) {
						filtered = append(filtered, key)
					}
				}
			}
			if filterText != "" && !fuzzy.MatchFold(filterText, p.Name) && len(filtered) == 0 {
				continue
			}
			list := widget.NewList(
				func() int { return len(filtered) },
				func() fyne.CanvasObject { return widget.NewLabel("template") },
				func(id widget.ListItemID, co fyne.CanvasObject) {
					key := filtered[id]
					co.(*widget.Label).SetText(cfg.GetDisplayName(p, key))
				},
			)
			prog := p
			list.OnSelected = func(id widget.ListItemID) {
				if id >= 0 && id < len(filtered) {
					cfg.OnSelect(prog, filtered[id])
				}
				list.Unselect(id)
			}
			acc.Append(widget.NewAccordionItem(fmt.Sprintf("%s (%d)", prog.Name, len(filtered)), list))
		}
		acc.Refresh()
	}
	searchbar.OnChanged = func(string) { rebuild() }
	rebuild()
	return searchbar, acc
}

// buildPointsAccordionWithSearchbar builds a Points accordion with a single searchbar above it.
// Filter matches program name or point name (fuzzy). onPointSelected is called when user selects a point.
func buildPointsAccordionWithSearchbar(onPointSelected func(actions.Point)) (*widget.Entry, *widget.Accordion) {
	return buildProgramListAccordionWithSearchbar(programListAccordionConfig{
		GetKeys: func(p *models.Program) []string {
			return p.PointRepo(config.MainMonitorSizeString).GetAllKeys()
		},
		GetDisplayName: func(p *models.Program, key string) string {
			pt, _ := p.PointRepo(config.MainMonitorSizeString).Get(key)
			if pt != nil {
				return pt.Name
			}
			return key
		},
		OnSelect: func(p *models.Program, key string) {
			pt, _ := p.PointRepo(config.MainMonitorSizeString).Get(key)
			if pt != nil {
				onPointSelected(actions.Point{Name: pt.Name, X: pt.X, Y: pt.Y})
			}
		},
	})
}

// buildSearchAreasAccordionWithSearchbar builds a Search Areas accordion with a single searchbar above it.
// Filter matches program name or search area name (fuzzy). onSelected is called when user selects a search area.
func buildSearchAreasAccordionWithSearchbar(onSelected func(actions.SearchArea)) (*widget.Entry, *widget.Accordion) {
	return buildProgramListAccordionWithSearchbar(programListAccordionConfig{
		GetKeys: func(p *models.Program) []string {
			return p.SearchAreaRepo(config.MainMonitorSizeString).GetAllKeys()
		},
		GetDisplayName: func(p *models.Program, key string) string {
			sa, _ := p.SearchAreaRepo(config.MainMonitorSizeString).Get(key)
			if sa != nil {
				return sa.Name
			}
			return key
		},
		OnSelect: func(p *models.Program, key string) {
			sa, _ := p.SearchAreaRepo(config.MainMonitorSizeString).Get(key)
			if sa != nil {
				onSelected(actions.SearchArea{
					Name:    sa.Name,
					LeftX:   sa.LeftX,
					TopY:    sa.TopY,
					RightX:  sa.RightX,
					BottomY: sa.BottomY,
				})
			}
		},
	})
}

// buildItemsAccordionWithSearchbar builds an Items section with a searchbar above and an accordion
// (extending Fyne's) where each program has a tri-state (empty/half/full) on the right of the header row.
// Returns refreshAccordion so the dialog can refresh the accordion when selection changes (e.g. after tri-state or item click).
func buildItemsAccordionWithSearchbar(
	getTargets func() []string,
	onItemSelected func(programName, baseItemName string),
	onSelectionChanged func(newTargets []string),
	refreshPreview func(),
) (*widget.Entry, fyne.CanvasObject, func()) {
	searchbar := widget.NewEntry()
	searchbar.SetPlaceHolder("Search here")
	acc := custom_widgets.NewAccordionWithHeaderWidgets()
	var itemGrids []*widget.GridWrap
	refreshAccordion := func() {
		acc.Refresh()
		for _, g := range itemGrids {
			g.Refresh()
		}
	}
	rebuild := func() {
		filterText := searchbar.Text
		acc.RemoveAll()
		itemGrids = itemGrids[:0]
		for _, p := range repositories.ProgramRepo().GetAll() {
			if filterText != "" && !fuzzy.MatchFold(filterText, p.Name) && !programHasMatchingItemsDialog(p, filterText) {
				continue
			}
			prog := p
			opts := ItemsAccordionOptions{
				Program:            prog,
				FilterText:         filterText,
				GetSelectedTargets: getTargets,
				OnItemSelected: func(baseItemName string) {
					onItemSelected(prog.Name, baseItemName)
				},
				OnSelectionChanged:      onSelectionChanged,
				AllButtonInHeader:       true,
				OnSelectionMaybeChanged: refreshAccordion,
				RegisterRefreshTarget:   func(grid *widget.GridWrap) { itemGrids = append(itemGrids, grid) },
			}
			item, allBtn := CreateProgramAccordionItem(opts)
			acc.AppendWithHeader(item, allBtn)
		}
	}
	searchbar.OnChanged = func(string) { rebuild() }
	rebuild()
	return searchbar, container.NewScroll(acc), refreshAccordion
}

// programHasMatchingItemsDialog returns true if the program has any item whose base name or tags match filterText (fuzzy).
func programHasMatchingItemsDialog(program *models.Program, filterText string) bool {
	if filterText == "" {
		return true
	}
	// Use same logic as binders: baseNames + tag match
	iconService := services.IconVariantServiceInstance()
	baseNames := iconService.GroupItemsByBaseName(program.ItemRepo().GetAllKeys())
	baseNameToItemName := make(map[string]string)
	for _, itemName := range program.ItemRepo().GetAllKeys() {
		baseName := iconService.GetBaseItemName(itemName)
		if _, exists := baseNameToItemName[baseName]; !exists {
			baseNameToItemName[baseName] = itemName
		}
	}
	for _, baseName := range baseNames {
		if fuzzy.MatchFold(filterText, baseName) {
			return true
		}
		itemName := baseNameToItemName[baseName]
		if itemName == "" {
			itemName = baseName
		}
		item, err := program.ItemRepo().Get(itemName)
		if err == nil {
			for _, tag := range item.Tags {
				if fuzzy.MatchFold(filterText, tag) {
					return true
				}
			}
		}
	}
	return false
}

// ShowActionDialog displays a dialog for editing an action.
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
	case *actions.Type:
		content, saveFunc = createTypeDialogContent(node)
		content.Resize(fyne.NewSize(400, 120))
	case *actions.Loop:
		content, saveFunc = createLoopDialogContent(node)
		content.Resize(fyne.NewSize(600, 100))
	case *actions.ImageSearch:
		content, saveFunc = createImageSearchDialogContent(node)
		content.Resize(fyne.NewSize(1000, 1000))
	case *actions.Ocr:
		content, saveFunc = createOcrDialogContent(node)
		content.Resize(fyne.NewSize(600, 500))
	case *actions.SetVariable:
		content, saveFunc = createSetVariableDialogContent(node)
		content.Resize(fyne.NewSize(600, 100))
	case *actions.Calculate:
		content, saveFunc = createCalculateDialogContent(node)
		content.Resize(fyne.NewSize(600, 100))
	case *actions.DataList:
		content, saveFunc = createDataListDialogContent(node)
		content.Resize(fyne.NewSize(600, 100))
	case *actions.SaveVariable:
		content, saveFunc = createSaveVariableDialogContent(node)
		content.Resize(fyne.NewSize(600, 100))
	// case *actions.Calibration:
	// 	content, saveFunc = createCalibrationDialogContent(node)
	// 	content.Resize(fyne.NewSize(600, 500))
	case *actions.FindPixel:
		content, saveFunc = createFindPixelDialogContent(node)
		content.Resize(fyne.NewSize(800, 500))
	case *actions.FocusWindow:
		content, saveFunc = createFocusWindowDialogContent(node)
		content.Resize(fyne.NewSize(500, 400))
	case *actions.RunMacro:
		content, saveFunc = createRunMacroDialogContent(node)
		content.Resize(fyne.NewSize(400, 120))
	default:
		content = widget.NewLabel("Unknown action type")
		saveFunc = func() {}
	}
	// Show custom dialog with save/cancel buttons
	showCustomActionDialog(u, action, content, saveFunc, onSave)
}

// actionModalDialog implements dialog.Dialog using widget.NewModalPopUp so content is not
// inset by fyne dialog.Layout (padWidth/2); the bordered panel can align with the popup edge.
type actionModalDialog struct {
	pop *widget.PopUp
}

func (d *actionModalDialog) Show()                 { d.pop.Show() }
func (d *actionModalDialog) Hide()                 { d.pop.Hide() }
func (d *actionModalDialog) Dismiss()              { d.Hide() }
func (d *actionModalDialog) SetDismissText(string) {}
func (d *actionModalDialog) SetOnClosed(func())    {}
func (d *actionModalDialog) Refresh()              { d.pop.Refresh() }
func (d *actionModalDialog) Resize(s fyne.Size)    { d.pop.Resize(s) }
func (d *actionModalDialog) MinSize() fyne.Size    { return d.pop.MinSize() }

var _ dialog.Dialog = (*actionModalDialog)(nil)

func showCustomActionDialog(u *Ui, action actions.ActionInterface, content fyne.CanvasObject, saveFunc func(), onSave func(actions.ActionInterface)) {
	var d *actionModalDialog
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

	titleLabel := widget.NewLabel("Edit Action - " + action.GetType())
	titleLabel.TextStyle = fyne.TextStyle{Bold: true}

	dialogContent := container.NewBorder(
		container.NewPadded(titleLabel),
		buttons,
		nil,
		nil,
		content,
	)

	th := fyne.CurrentApp().Settings().Theme()
	v := fyne.CurrentApp().Settings().ThemeVariant()
	panelBg := canvas.NewRectangle(th.Color(theme.ColorNameOverlayBackground, v))
	panelBg.CornerRadius = theme.InputRadiusSize()

	border := canvas.NewRectangle(color.Transparent)
	border.StrokeColor = sqyrePrimary
	border.StrokeWidth = 2
	border.CornerRadius = theme.InputRadiusSize()
	innerPadded := container.NewPadded(container.NewPadded(container.NewPadded(container.NewPadded(dialogContent))))
	borderedDialogContent := container.NewStack(panelBg, border, innerPadded)

	pop := widget.NewModalPopUp(borderedDialogContent, u.Window.Canvas())
	fynetooltip.AddPopUpToolTipLayer(pop)
	d = &actionModalDialog{pop: pop}
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
	timeEntry := newVarEntry()
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
func pointCoordToInt(v any) int {
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
				services.LogPanicToFile(r, "Action dialog: point preview capture")
				pointPreviewImage.Image = nil
				pointPreviewImage.Refresh()
			}
		}()

		captureImg, err := robotgo.CaptureImg(0, 0, screenWidth, screenHeight)
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

	// Points accordion with searchbar above (fuzzy match program name + point name)
	pointsSearchbar, pointsAccordion := buildPointsAccordionWithSearchbar(func(pt actions.Point) {
		tempPoint = pt
		updateCoordsLabel(&tempPoint)
		updatePreview(&tempPoint)
	})

	// Update label and preview for initial point
	updateCoordsLabel(&tempPoint)
	updatePreview(&tempPoint)

	smoothCheck := widget.NewCheck("Smooth", nil)
	smoothCheck.SetChecked(action.Smooth)

	content := container.NewVBox(
		container.NewHBox(coordsLabel, layout.NewSpacer(), smoothCheck),
		container.NewHSplit(
			container.NewBorder(pointsSearchbar, nil, nil, nil, pointsAccordion),
			pointPreviewImage,
		),
	)

	saveFunc := func() {
		action.Point = tempPoint
		action.Smooth = smoothCheck.Checked
	}

	return content, saveFunc
}

func createClickDialogContent(action *actions.Click) (fyne.CanvasObject, func()) {
	buttonCheck := custom_widgets.NewToggle(func(b bool) {})
	buttonCheck.SetToggled(action.Button)
	stateToggle := custom_widgets.NewToggle(func(b bool) {})
	stateToggle.SetToggled(action.State)

	content := container.NewVBox(
		container.NewHBox(
			layout.NewSpacer(),
			widget.NewLabel("left"),
			buttonCheck,
			widget.NewLabel("right"),
			layout.NewSpacer(),
		),
		container.NewHBox(
			layout.NewSpacer(),
			widget.NewLabel("up"),
			stateToggle,
			widget.NewLabel("down"),
			layout.NewSpacer(),
		),
	)

	saveFunc := func() {
		action.Button = buttonCheck.Toggled
		action.State = stateToggle.Toggled
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

func createTypeDialogContent(action *actions.Type) (fyne.CanvasObject, func()) {
	textEntry := newVarEntry()
	textEntry.SetText(action.Text)
	textEntry.SetPlaceHolder("Text to type (supports ${variable})")

	delayEntry := widget.NewEntry()
	delayEntry.SetText(fmt.Sprintf("%d", action.DelayMs))
	delayEntry.SetPlaceHolder("Delay between key presses (ms)")

	content := widget.NewForm(
		widget.NewFormItem("Text to type:", textEntry),
		widget.NewFormItem("Delay (ms):", delayEntry),
	)

	saveFunc := func() {
		action.Text = textEntry.Text
		if val, err := strconv.Atoi(strings.TrimSpace(delayEntry.Text)); err == nil && val >= 0 {
			action.DelayMs = val
		}
	}

	return content, saveFunc
}

func createLoopDialogContent(action *actions.Loop) (fyne.CanvasObject, func()) {
	nameEntry := widget.NewEntry()
	nameEntry.SetText(action.Name)
	countEntry := newVarEntry()
	countEntry.SetPlaceHolder("e.g. 5 or ${countVar}")
	switch c := action.Count.(type) {
	case int:
		countEntry.SetText(fmt.Sprintf("%d", c))
	case string:
		countEntry.SetText(c)
	default:
		countEntry.SetText(fmt.Sprintf("%v", c))
	}

	content := widget.NewForm(
		widget.NewFormItem("Name:", nameEntry),
		widget.NewFormItem("Loops (number or variable):", countEntry),
	)

	saveFunc := func() {
		action.Name = nameEntry.Text
		s := strings.TrimSpace(countEntry.Text)
		if s == "" {
			action.Count = 1
			return
		}
		if count, err := strconv.Atoi(s); err == nil {
			action.Count = count
		} else {
			action.Count = s
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
	toleranceMin, toleranceMax := 0.0, 1.0
	toleranceIncrementer := custom_widgets.NewFloatIncrementer(float64(action.Tolerance), 0.01, &toleranceMin, &toleranceMax, 2)
	toleranceIncrementer.SetValue(float64(action.Tolerance))
	blurMin, blurMax := 1, 21
	blurIncrementer := custom_widgets.NewIncrementer(action.Blur, 2, &blurMin, &blurMax)
	blurIncrementer.SetValue(action.Blur)
	outputXVarEntry := newVarEntry()
	outputXVarEntry.SetText(action.OutputXVariable)
	outputXVarEntry.SetPlaceHolder("e.g. foundX (sub-actions also get ${StackMax}, ${Cols}, ${Rows}, ${ItemName}, ${ImagePixelWidth}, ${ImagePixelHeight})")
	outputYVarEntry := newVarEntry()
	outputYVarEntry.SetText(action.OutputYVariable)
	outputYVarEntry.SetPlaceHolder("e.g. foundY")
	waitTilFoundCheck := widget.NewCheck("Wait until found", nil)
	waitTilFoundCheck.SetChecked(action.WaitTilFound)
	waitTilFoundSecondsMin := 0
	waitTilFoundSecondsIncrementer := custom_widgets.NewIncrementer(action.WaitTilFoundSeconds, 1, &waitTilFoundSecondsMin, nil)
	if action.WaitTilFoundSeconds <= 0 {
		waitTilFoundSecondsIncrementer.SetValue(10)
	} else {
		waitTilFoundSecondsIncrementer.SetValue(action.WaitTilFoundSeconds)
	}
	waitTilFoundIntervalMin := 100
	waitTilFoundIntervalIncrementer := custom_widgets.NewIncrementer(action.WaitTilFoundIntervalMs, 100, &waitTilFoundIntervalMin, nil)
	if action.WaitTilFoundIntervalMs < 100 {
		waitTilFoundIntervalIncrementer.SetValue(100)
	} else {
		waitTilFoundIntervalIncrementer.SetValue(action.WaitTilFoundIntervalMs)
	}
	setWaitTilFoundEntriesEnabled := func(enabled bool) {
		if enabled {
			waitTilFoundSecondsIncrementer.Enable()
			waitTilFoundIntervalIncrementer.Enable()
			return
		}
		waitTilFoundSecondsIncrementer.Disable()
		waitTilFoundIntervalIncrementer.Disable()
	}
	waitTilFoundCheck.OnChanged = setWaitTilFoundEntriesEnabled
	setWaitTilFoundEntriesEnabled(waitTilFoundCheck.Checked)

	// Temporary storage for changes (only applied on save)
	tempSearchArea := action.SearchArea
	tempTargets := slices.Clone(action.Targets)
	tempTargetsRef := &tempTargets

	// Search Areas accordion with searchbar above (fuzzy match program name + search area name)
	searchAreasSearchbar, searchAreasAccordion := buildSearchAreasAccordionWithSearchbar(func(sa actions.SearchArea) {
		tempSearchArea = sa
	})

	previewSize := fyne.NewSquareSize(30)
	var refreshItemsAccordion func()
	var removeTarget func(target string)

	previewList := widget.NewGridWrap(
		func() int { return len(tempTargets) },
		func() fyne.CanvasObject {
			icon := canvas.NewImageFromResource(theme.BrokenImageIcon())
			icon.SetMinSize(previewSize)
			icon.FillMode = canvas.ImageFillContain
			removeBtn := ttwidget.NewButtonWithIcon("", theme.CancelIcon(), nil)
			removeBtn.Importance = widget.LowImportance
			return container.NewStack(icon, removeBtn)
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

			removeBtn := stack.Objects[1].(*ttwidget.Button)
			removeBtn.OnTapped = func() {
				if removeTarget != nil {
					removeTarget(target)
				}
			}
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

	removeTarget = func(target string) {
		t := *tempTargetsRef
		if i := slices.Index(t, target); i != -1 {
			t = slices.Delete(t, i, i+1)
			*tempTargetsRef = t
		}
		refreshPreview()
		if refreshItemsAccordion != nil {
			refreshItemsAccordion()
		}
	}

	// Items accordion with searchbar above (fuzzy match program name + item name/tags)
	var itemsSearchbar *widget.Entry
	var itemsAccordion fyne.CanvasObject
	itemsSearchbar, itemsAccordion, refreshItemsAccordion = buildItemsAccordionWithSearchbar(
		func() []string { return *tempTargetsRef },
		func(programName, baseItemName string) {
			name := programName + config.ProgramDelimiter + baseItemName
			t := *tempTargetsRef
			if i := slices.Index(t, name); i != -1 {
				t = slices.Delete(t, i, i+1)
			} else {
				t = append(t, name)
			}
			slices.Sort(t)
			*tempTargetsRef = t
			refreshPreview()
			refreshItemsAccordion()
		},
		func(newTargets []string) {
			*tempTargetsRef = newTargets
			refreshPreview()
			refreshItemsAccordion()
		},
		refreshPreview,
	)

	rightPanel := container.NewBorder(
		nil, nil,
		nil, nil,
		container.NewVSplit(
			container.NewVBox(
				widget.NewForm(
					widget.NewFormItem("Name:", nameEntry),
					widget.NewFormItem("Row Split:", rowSplitEntry),
					widget.NewFormItem("Col Split:", colSplitEntry),
					widget.NewFormItem("Tolerance:", toleranceIncrementer),
					widget.NewFormItem("Blur:", container.NewVBox(blurIncrementer, layout.NewSpacer())),
					widget.NewFormItem("Output X Variable:", outputXVarEntry),
					widget.NewFormItem("Output Y Variable:", outputYVarEntry),
					widget.NewFormItem("", waitTilFoundCheck),
					widget.NewFormItem("Timeout (seconds):", waitTilFoundSecondsIncrementer),
					widget.NewFormItem("Search interval (ms):", waitTilFoundIntervalIncrementer),
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
						searchAreasSearchbar, nil, nil, nil,
						searchAreasAccordion,
					),
				),
				widget.NewAccordionItem("Items",
					container.NewBorder(
						itemsSearchbar, nil, nil, nil,
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
		action.Tolerance = float32(toleranceIncrementer.Value)
		action.Blur = blurIncrementer.Value
		action.OutputXVariable = outputXVarEntry.Text
		action.OutputYVariable = outputYVarEntry.Text
		action.WaitTilFound = waitTilFoundCheck.Checked
		if waitTilFoundSecondsIncrementer.Value >= 0 {
			action.WaitTilFoundSeconds = waitTilFoundSecondsIncrementer.Value
		}
		if waitTilFoundIntervalIncrementer.Value >= 0 {
			action.WaitTilFoundIntervalMs = waitTilFoundIntervalIncrementer.Value
		}
		// Apply temporary changes
		action.SearchArea = tempSearchArea
		action.Targets = tempTargets
		slices.Sort(action.Targets)
	}

	return content, saveFunc
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

func createOcrDialogContent(action *actions.Ocr) (fyne.CanvasObject, func()) {
	nameEntry := widget.NewEntry()
	nameEntry.SetText(action.Name)
	targetEntry := newVarEntry()
	targetEntry.SetText(action.Target)
	outputVarEntry := newVarEntry()
	outputVarEntry.SetText(action.OutputVariable)
	outputXVarEntry := newVarEntry()
	outputXVarEntry.SetText(action.OutputXVariable)
	outputXVarEntry.SetPlaceHolder("e.g. foundX")
	outputYVarEntry := newVarEntry()
	outputYVarEntry.SetText(action.OutputYVariable)
	outputYVarEntry.SetPlaceHolder("e.g. foundY")
	waitTilFoundCheck := widget.NewCheck("Wait until found", nil)
	waitTilFoundCheck.SetChecked(action.WaitTilFound)
	waitTilFoundSecondsMin := 0
	waitTilFoundSecondsIncrementer := custom_widgets.NewIncrementer(action.WaitTilFoundSeconds, 1, &waitTilFoundSecondsMin, nil)
	if action.WaitTilFoundSeconds <= 0 {
		waitTilFoundSecondsIncrementer.SetValue(10)
	} else {
		waitTilFoundSecondsIncrementer.SetValue(action.WaitTilFoundSeconds)
	}
	waitTilFoundIntervalMin := 0
	waitTilFoundIntervalIncrementer := custom_widgets.NewIncrementer(action.WaitTilFoundIntervalMs, 100, &waitTilFoundIntervalMin, nil)
	if action.WaitTilFoundIntervalMs < 100 {
		waitTilFoundIntervalIncrementer.SetValue(100)
	} else {
		waitTilFoundIntervalIncrementer.SetValue(action.WaitTilFoundIntervalMs)
	}
	setWaitTilFoundEntriesEnabled := func(enabled bool) {
		if enabled {
			waitTilFoundSecondsIncrementer.Enable()
			waitTilFoundIntervalIncrementer.Enable()
			return
		}
		waitTilFoundSecondsIncrementer.Disable()
		waitTilFoundIntervalIncrementer.Disable()
	}
	waitTilFoundCheck.OnChanged = setWaitTilFoundEntriesEnabled
	setWaitTilFoundEntriesEnabled(waitTilFoundCheck.Checked)

	// Temporary storage for changes (only applied on save)
	tempSearchArea := action.SearchArea

	// Search Areas accordion with searchbar above (fuzzy match program name + search area name)
	searchAreasSearchbar, searchAreasAccordion := buildSearchAreasAccordionWithSearchbar(func(sa actions.SearchArea) {
		tempSearchArea = sa
	})

	form := widget.NewForm(
		widget.NewFormItem("Name:", nameEntry),
		widget.NewFormItem("Text Target:", targetEntry),
		widget.NewFormItem("Output Variable:", outputVarEntry),
		widget.NewFormItem("Output X Variable:", outputXVarEntry),
		widget.NewFormItem("Output Y Variable:", outputYVarEntry),
		widget.NewFormItem("", waitTilFoundCheck),
		widget.NewFormItem("Timeout (seconds):", waitTilFoundSecondsIncrementer),
		widget.NewFormItem("Search interval (ms):", waitTilFoundIntervalIncrementer),
	)

	content := container.NewHSplit(
		widget.NewAccordion(
			widget.NewAccordionItem("Search Areas",
				container.NewBorder(
					searchAreasSearchbar, nil, nil, nil,
					searchAreasAccordion,
				),
			),
		),
		form,
	)

	saveFunc := func() {
		action.Name = nameEntry.Text
		action.Target = targetEntry.Text
		action.OutputVariable = outputVarEntry.Text
		action.OutputXVariable = outputXVarEntry.Text
		action.OutputYVariable = outputYVarEntry.Text
		action.WaitTilFound = waitTilFoundCheck.Checked
		if waitTilFoundSecondsIncrementer.Value >= 0 {
			action.WaitTilFoundSeconds = waitTilFoundSecondsIncrementer.Value
		}
		if waitTilFoundIntervalIncrementer.Value >= 0 {
			action.WaitTilFoundIntervalMs = waitTilFoundIntervalIncrementer.Value
		}
		action.SearchArea = tempSearchArea
	}

	return content, saveFunc
}

func createSetVariableDialogContent(action *actions.SetVariable) (fyne.CanvasObject, func()) {
	nameEntry := newVarEntry()
	nameEntry.SetText(action.VariableName)
	valueEntry := newVarEntry()
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
	exprEntry := newVarEntry()
	exprEntry.SetText(action.Expression)
	varEntry := newVarEntry()
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
	sourceEntry := newMultiLineVarEntry()
	sourceEntry.SetText(action.Source)
	sourceEntry.SetPlaceHolder("File: path relative to ~/.sqyre/variables/ (e.g. mylist.txt)\nOr paste text directly")
	varEntry := newVarEntry()
	varEntry.SetText(action.OutputVar)
	lengthVarEntry := newVarEntry()
	lengthVarEntry.SetText(action.LengthVar)
	lengthVarEntry.SetPlaceHolder("e.g. lineCount (optional, for Loop)")
	isFileCheck := widget.NewCheck("Source is file path (relative to ~/.sqyre/variables/)", nil)
	isFileCheck.SetChecked(action.IsFile)
	skipBlankCheck := widget.NewCheck("Skip blank lines (exclude from count and iteration)", nil)
	skipBlankCheck.SetChecked(action.SkipBlankLines)

	content := widget.NewForm(
		widget.NewFormItem("Source (file path or text):", sourceEntry),
		widget.NewFormItem("", isFileCheck),
		widget.NewFormItem("", skipBlankCheck),
		widget.NewFormItem("Output Variable:", varEntry),
		widget.NewFormItem("Length Variable (optional):", lengthVarEntry),
	)

	saveFunc := func() {
		action.Source = sourceEntry.Text
		action.OutputVar = varEntry.Text
		action.LengthVar = strings.TrimSpace(lengthVarEntry.Text)
		action.IsFile = isFileCheck.Checked
		action.SkipBlankLines = skipBlankCheck.Checked
	}

	return content, saveFunc
}

func createSaveVariableDialogContent(action *actions.SaveVariable) (fyne.CanvasObject, func()) {
	varEntry := newVarEntry()
	varEntry.SetText(action.VariableName)
	destEntry := newVarEntry()
	destEntry.SetText(action.Destination)
	destEntry.SetPlaceHolder("~/.sqyre/variables/... or 'clipboard'")
	appendCheck := widget.NewCheck("Append to file", nil)
	appendCheck.SetChecked(action.Append)
	appendNewlineCheck := widget.NewCheck("New line with every append", nil)
	appendNewlineCheck.SetChecked(action.AppendNewline)

	content := widget.NewForm(
		widget.NewFormItem("Variable Name:", varEntry),
		widget.NewFormItem("Destination (~/.sqyre/variables/... or 'clipboard'):", destEntry),
		widget.NewFormItem("", appendCheck),
		widget.NewFormItem("", appendNewlineCheck),
	)

	saveFunc := func() {
		action.VariableName = varEntry.Text
		action.Destination = destEntry.Text
		action.Append = appendCheck.Checked
		action.AppendNewline = appendNewlineCheck.Checked
	}

	return content, saveFunc
}

// hexToColor parses "#rrggbb", "rrggbb", or "aarrggbb" into a color. Alpha is ignored for display.
func hexToColor(hex string) (color.Color, bool) {
	hex = strings.TrimPrefix(strings.ToLower(strings.TrimSpace(hex)), "#")
	if len(hex) == 8 {
		hex = hex[2:]
	}
	if len(hex) != 6 {
		return color.RGBA{}, false
	}
	var r, g, b uint8
	if _, err := fmt.Sscanf(hex, "%02x%02x%02x", &r, &g, &b); err != nil {
		return color.RGBA{}, false
	}
	return color.RGBA{R: r, G: g, B: b, A: 255}, true
}

func createFindPixelDialogContent(action *actions.FindPixel) (fyne.CanvasObject, func()) {
	nameEntry := widget.NewEntry()
	nameEntry.SetText(action.Name)
	nameEntry.SetPlaceHolder("Optional name for this action")

	tempSearchArea := action.SearchArea

	// Search Areas accordion with searchbar above (fuzzy match program name + search area name)
	searchAreasSearchbar, searchAreasAccordion := buildSearchAreasAccordionWithSearchbar(func(sa actions.SearchArea) {
		tempSearchArea = sa
	})

	colorEntry := widget.NewEntry()
	colorEntry.SetText(action.TargetColor)
	colorEntry.SetPlaceHolder("Hex e.g. ffffff or #ffffff")

	swatch := canvas.NewRectangle(color.RGBA{128, 128, 128, 255})
	swatch.SetMinSize(fyne.NewSize(32, 32))
	swatch.StrokeWidth = 1
	swatch.StrokeColor = color.RGBA{R: 80, G: 80, B: 80, A: 255}
	updateSwatch := func() {
		if c, ok := hexToColor(colorEntry.Text); ok {
			swatch.FillColor = c
		}
		swatch.Refresh()
	}
	updateSwatch()
	colorEntry.OnChanged = func(string) { updateSwatch() }

	dropperBtn := ttwidget.NewButtonWithIcon("", theme.MediaRecordIcon(), func() {
		dismissOverlay := ShowRecordingOverlay(
			"Pick a Color",
			"Left click anywhere to sample the pixel color",
			"Right click to cancel",
		)

		services.GoSafe(func() {
			hook.Register(hook.MouseDown, []string{}, func(e hook.Event) {
				switch e.Button {
				case hook.MouseMap["left"]:
					x, y := robotgo.Location()
					hex := robotgo.GetPixelColor(x, y)
					hex = strings.TrimPrefix(strings.ToLower(hex), "#")
					if len(hex) == 8 {
						hex = hex[2:]
					}
					fyne.DoAndWait(func() {
						colorEntry.SetText(hex)
						updateSwatch()
						dismissOverlay()
					})
				default:
					fyne.DoAndWait(func() {
						dismissOverlay()
					})
				}
				go hook.Unregister(hook.MouseDown, []string{})
			})
		})
	})
	dropperBtn.SetToolTip("Click Dropper, then click on screen to pick a color")

	colorRow := container.NewBorder(
		nil, nil,
		swatch, dropperBtn,
		colorEntry,
	)

	toleranceEntry := widget.NewEntry()
	toleranceEntry.SetText(fmt.Sprintf("%d", action.ColorTolerance))
	toleranceEntry.SetPlaceHolder("0 = exact match")
	toleranceSlider := widget.NewSlider(0, 100)
	toleranceSlider.SetValue(float64(action.ColorTolerance))
	toleranceSlider.OnChanged = func(f float64) {
		toleranceEntry.SetText(fmt.Sprintf("%.0f", f))
	}
	toleranceEntry.OnChanged = func(s string) {
		if val, err := strconv.ParseFloat(strings.TrimSpace(s), 64); err == nil {
			if val < 0 {
				val = 0
			}
			if val > 100 {
				val = 100
			}
			toleranceSlider.SetValue(val)
		}
	}
	toleranceRow := container.NewHBox(toleranceEntry, widget.NewLabel("%"), toleranceSlider)

	outputXVarEntry := newVarEntry()
	outputXVarEntry.SetText(action.OutputXVariable)
	outputXVarEntry.SetPlaceHolder("e.g. foundX")
	outputYVarEntry := newVarEntry()
	outputYVarEntry.SetText(action.OutputYVariable)
	outputYVarEntry.SetPlaceHolder("e.g. foundY")

	waitTilFoundCheck := widget.NewCheck("Wait until found", nil)
	waitTilFoundCheck.SetChecked(action.WaitTilFound)
	waitTilFoundSecondsEntry := widget.NewEntry()
	if action.WaitTilFoundSeconds <= 0 {
		waitTilFoundSecondsEntry.SetText("10")
	} else {
		waitTilFoundSecondsEntry.SetText(fmt.Sprintf("%d", action.WaitTilFoundSeconds))
	}
	waitTilFoundSecondsEntry.SetPlaceHolder("Seconds to keep trying if not found")
	waitTilFoundIntervalEntry := widget.NewEntry()
	if action.WaitTilFoundIntervalMs < 100 {
		waitTilFoundIntervalEntry.SetText("100")
	} else {
		waitTilFoundIntervalEntry.SetText(fmt.Sprintf("%d", action.WaitTilFoundIntervalMs))
	}
	waitTilFoundIntervalEntry.SetPlaceHolder("Milliseconds between retries (default 100)")

	form := widget.NewForm(
		widget.NewFormItem("Name:", nameEntry),
		widget.NewFormItem("Target color:", colorRow),
		widget.NewFormItem("Color tolerance:", toleranceRow),
		widget.NewFormItem("Output X Variable:", outputXVarEntry),
		widget.NewFormItem("Output Y Variable:", outputYVarEntry),
		widget.NewFormItem("", waitTilFoundCheck),
		widget.NewFormItem("Timeout (seconds):", waitTilFoundSecondsEntry),
		widget.NewFormItem("Search interval (ms):", waitTilFoundIntervalEntry),
	)

	content := container.NewHSplit(
		widget.NewAccordion(
			widget.NewAccordionItem("Search Areas",
				container.NewBorder(
					searchAreasSearchbar, nil, nil, nil,
					searchAreasAccordion,
				),
			),
		),
		form,
	)

	saveFunc := func() {
		action.Name = strings.TrimSpace(nameEntry.Text)
		action.SearchArea = tempSearchArea
		action.TargetColor = strings.ToLower(strings.TrimPrefix(strings.TrimSpace(colorEntry.Text), "#"))
		if t, err := strconv.Atoi(strings.TrimSpace(toleranceEntry.Text)); err == nil {
			if t < 0 {
				t = 0
			}
			if t > 100 {
				t = 100
			}
			action.ColorTolerance = t
		}
		action.OutputXVariable = outputXVarEntry.Text
		action.OutputYVariable = outputYVarEntry.Text
		action.WaitTilFound = waitTilFoundCheck.Checked
		if s, err := strconv.Atoi(waitTilFoundSecondsEntry.Text); err == nil && s >= 0 {
			action.WaitTilFoundSeconds = s
		}
		if ms, err := strconv.Atoi(waitTilFoundIntervalEntry.Text); err == nil && ms >= 0 {
			action.WaitTilFoundIntervalMs = ms
		}
	}

	return content, saveFunc
}

func createFocusWindowDialogContent(action *actions.FocusWindow) (fyne.CanvasObject, func()) {
	windowEntry := widget.NewEntry()
	windowEntry.SetText(action.WindowTarget)
	windowEntry.SetPlaceHolder("Type to search or pick from list (e.g. chrome, code)")

	// Full list from API; filtered list is what the list widget shows
	allWindowNames := []string{}
	filteredNames := []string{}

	applyFilter := func() {
		q := strings.TrimSpace(strings.ToLower(windowEntry.Text))
		if q == "" {
			filteredNames = make([]string, len(allWindowNames))
			copy(filteredNames, allWindowNames)
		} else {
			filteredNames = filteredNames[:0]
			for _, name := range allWindowNames {
				if fuzzy.Match(q, strings.ToLower(name)) {
					filteredNames = append(filteredNames, name)
				}
			}
		}
	}

	windowList := widget.NewList(
		func() int { return len(filteredNames) },
		func() fyne.CanvasObject { return widget.NewLabel("") },
		func(id widget.ListItemID, co fyne.CanvasObject) {
			if id < len(filteredNames) {
				co.(*widget.Label).SetText(filteredNames[id])
			}
		},
	)
	windowList.OnSelected = func(id widget.ListItemID) {
		if id >= 0 && id < len(filteredNames) {
			windowEntry.SetText(filteredNames[id])
		}
	}

	refreshList := func() {
		applyFilter()
		windowList.Refresh()
	}

	windowEntry.OnChanged = func(string) { refreshList() }

	refreshBtn := widget.NewButton("Refresh list", func() {
		names, err := services.ActiveWindowNames()
		if err != nil {
			allWindowNames = []string{fmt.Sprintf("(error: %v)", err)}
		} else {
			allWindowNames = names
		}
		refreshList()
	})
	// Load list on open
	services.GoSafe(func() {
		names, err := services.ActiveWindowNames()
		if err != nil {
			fyne.Do(func() {
				allWindowNames = []string{fmt.Sprintf("(error: %v)", err)}
				refreshList()
			})
			return
		}
		fyne.Do(func() {
			allWindowNames = names
			refreshList()
		})
	})

	listCard := container.NewBorder(
		widget.NewLabel("Active windows (list filters as you type):"),
		refreshBtn,
		nil, nil,
		windowList,
	)
	listCard.Resize(fyne.NewSize(400, 200))

	content := container.NewBorder(
		widget.NewForm(
			widget.NewFormItem("Window to focus / search:", windowEntry),
		),
		nil, nil, nil,
		listCard,
	)

	saveFunc := func() {
		action.WindowTarget = strings.TrimSpace(windowEntry.Text)
	}

	return content, saveFunc
}

func createRunMacroDialogContent(action *actions.RunMacro) (fyne.CanvasObject, func()) {
	macroNames := repositories.MacroRepo().GetAllKeys()
	// Exclude the currently open macro to prevent infinite recursion
	if st := GetUi().Mui.MTabs.SelectedTab(); st != nil && st.Macro != nil && st.Macro.Name != "" {
		macroNames = slices.DeleteFunc(macroNames, func(name string) bool { return name == st.Macro.Name })
	}
	if len(macroNames) == 0 {
		macroNames = []string{""}
	}
	macroSelect := widget.NewSelect(macroNames, nil)
	if action.MacroName != "" && !slices.Contains(macroNames, action.MacroName) {
		// Macro was deleted or renamed; add current value so it's visible (unless it's the current macro - then clear)
		st := GetUi().Mui.MTabs.SelectedTab()
		if st != nil && st.Macro != nil && action.MacroName == st.Macro.Name {
			macroNames = append([]string{""}, macroNames...)
			macroSelect.Options = macroNames
			macroSelect.SetSelected("")
		} else {
			macroSelect.Options = append([]string{action.MacroName}, macroNames...)
			macroSelect.SetSelected(action.MacroName)
		}
	} else {
		macroSelect.SetSelected(action.MacroName)
	}

	content := widget.NewForm(
		widget.NewFormItem("Macro to run:", macroSelect),
	)

	saveFunc := func() {
		action.MacroName = macroSelect.Selected
	}

	return content, saveFunc
}
