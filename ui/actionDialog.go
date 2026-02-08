package ui

import (
	"Squire/internal/assets"
	"Squire/internal/config"
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
	hook "github.com/luhrMan/gohook"
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
	case *actions.Calibration:
		content, saveFunc = createCalibrationDialogContent(node)
		content.Resize(fyne.NewSize(600, 500))
	case *actions.WaitForPixel:
		content, saveFunc = createWaitForPixelDialogContent(node)
		content.Resize(fyne.NewSize(450, 280))
	case *actions.FocusWindow:
		content, saveFunc = createFocusWindowDialogContent(node)
		content.Resize(fyne.NewSize(500, 400))
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
	holdCheck := custom_widgets.NewToggle(func(b bool) {})
	holdCheck.SetToggled(action.Hold)

	content := container.NewVBox(
		container.NewHBox(
			layout.NewSpacer(),
			widget.NewLabel("left"),
			buttonCheck,
			widget.NewLabel("right"),
			layout.NewSpacer(),
		),
		container.NewHBox(
			widget.NewLabel("Hold"),
			holdCheck,
		),
	)

	saveFunc := func() {
		action.Button = buttonCheck.Toggled
		action.Hold = holdCheck.Toggled
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
	toleranceEntry := widget.NewEntry()
	toleranceEntry.SetText(fmt.Sprintf("%g", action.Tolerance))
	blurEntry := widget.NewEntry()
	blurEntry.SetText(fmt.Sprintf("%d", action.Blur))
	outputXVarEntry := widget.NewEntry()
	outputXVarEntry.SetText(action.OutputXVariable)
	outputYVarEntry := widget.NewEntry()
	outputYVarEntry.SetText(action.OutputYVariable)
	waitTilFoundCheck := widget.NewCheck("Wait until found", nil)
	waitTilFoundCheck.SetChecked(action.WaitTilFound)
	waitTilFoundSecondsEntry := widget.NewEntry()
	if action.WaitTilFoundSeconds <= 0 {
		waitTilFoundSecondsEntry.SetText("10")
	} else {
		waitTilFoundSecondsEntry.SetText(fmt.Sprintf("%d", action.WaitTilFoundSeconds))
	}
	waitTilFoundSecondsEntry.SetPlaceHolder("Seconds to keep trying if not found")

	// Temporary storage for changes (only applied on save)
	tempSearchArea := action.SearchArea
	tempTargets := slices.Clone(action.Targets)
	tempTargetsRef := &tempTargets

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

	// Create Items accordion (shared with editor Items tab)
	itemsAccordion := widget.NewAccordion()
	for _, p := range repositories.ProgramRepo().GetAll() {
		prog := p
		accordionItem := CreateProgramAccordionItem(ItemsAccordionOptions{
			Program:            prog,
			GetSelectedTargets: func() []string { return *tempTargetsRef },
			OnItemSelected: func(baseItemName string) {
				name := prog.Name + config.ProgramDelimiter + baseItemName
				t := *tempTargetsRef
				if i := slices.Index(t, name); i != -1 {
					t = slices.Delete(t, i, i+1)
				} else {
					t = append(t, name)
				}
				slices.Sort(t)
				*tempTargetsRef = t
				refreshPreview()
			},
		})
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
					widget.NewFormItem("Tolerance:", toleranceEntry),
					widget.NewFormItem("Blur:", blurEntry),
					widget.NewFormItem("Output X Variable:", outputXVarEntry),
					widget.NewFormItem("Output Y Variable:", outputYVarEntry),
					widget.NewFormItem("", waitTilFoundCheck),
					widget.NewFormItem("Timeout (seconds):", waitTilFoundSecondsEntry),
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
		if tol, err := strconv.ParseFloat(toleranceEntry.Text, 32); err == nil {
			action.Tolerance = float32(tol)
		}
		if b, err := strconv.Atoi(blurEntry.Text); err == nil {
			action.Blur = b
		}
		action.OutputXVariable = outputXVarEntry.Text
		action.OutputYVariable = outputYVarEntry.Text
		action.WaitTilFound = waitTilFoundCheck.Checked
		if s, err := strconv.Atoi(waitTilFoundSecondsEntry.Text); err == nil && s >= 0 {
			action.WaitTilFoundSeconds = s
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
	targetEntry := widget.NewEntry()
	targetEntry.SetText(action.Target)
	outputVarEntry := widget.NewEntry()
	outputVarEntry.SetText(action.OutputVariable)
	waitTilFoundCheck := widget.NewCheck("Wait until found", nil)
	waitTilFoundCheck.SetChecked(action.WaitTilFound)
	waitTilFoundSecondsEntry := widget.NewEntry()
	if action.WaitTilFoundSeconds <= 0 {
		waitTilFoundSecondsEntry.SetText("10")
	} else {
		waitTilFoundSecondsEntry.SetText(fmt.Sprintf("%d", action.WaitTilFoundSeconds))
	}
	waitTilFoundSecondsEntry.SetPlaceHolder("Seconds to keep trying if not found")

	// Temporary storage for changes (only applied on save)
	tempSearchArea := action.SearchArea

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

	form := widget.NewForm(
		widget.NewFormItem("Name:", nameEntry),
		widget.NewFormItem("Text Target:", targetEntry),
		widget.NewFormItem("Output Variable:", outputVarEntry),
		widget.NewFormItem("", waitTilFoundCheck),
		widget.NewFormItem("Timeout (seconds):", waitTilFoundSecondsEntry),
	)

	content := container.NewHSplit(
		widget.NewAccordion(
			widget.NewAccordionItem("Search Areas",
				container.NewBorder(
					nil, nil, nil, nil,
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
		action.WaitTilFound = waitTilFoundCheck.Checked
		if s, err := strconv.Atoi(waitTilFoundSecondsEntry.Text); err == nil && s >= 0 {
			action.WaitTilFoundSeconds = s
		}
		action.SearchArea = tempSearchArea
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
	sourceEntry.SetPlaceHolder("File: path relative to ~/Sqyre/variables/ (e.g. mylist.txt)\nOr paste text directly")
	varEntry := widget.NewEntry()
	varEntry.SetText(action.OutputVar)
	lengthVarEntry := widget.NewEntry()
	lengthVarEntry.SetText(action.LengthVar)
	lengthVarEntry.SetPlaceHolder("e.g. lineCount (optional, for Loop)")
	isFileCheck := widget.NewCheck("Source is file path (relative to ~/Sqyre/variables/)", nil)
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
	varEntry := widget.NewEntry()
	varEntry.SetText(action.VariableName)
	destEntry := widget.NewEntry()
	destEntry.SetText(action.Destination)
	destEntry.SetPlaceHolder("~/Sqyre/variables/... or 'clipboard'")
	appendCheck := widget.NewCheck("Append to file", nil)
	appendCheck.SetChecked(action.Append)
	appendNewlineCheck := widget.NewCheck("New line with every append", nil)
	appendNewlineCheck.SetChecked(action.AppendNewline)

	content := widget.NewForm(
		widget.NewFormItem("Variable Name:", varEntry),
		widget.NewFormItem("Destination (~/Sqyre/variables/... or 'clipboard'):", destEntry),
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

func createWaitForPixelDialogContent(action *actions.WaitForPixel) (fyne.CanvasObject, func()) {
	nameEntry := widget.NewEntry()
	nameEntry.SetText(action.Name)
	nameEntry.SetPlaceHolder("Optional name for this action")

	xEntry := widget.NewEntry()
	yEntry := widget.NewEntry()
	switch v := action.Point.X.(type) {
	case int:
		xEntry.SetText(fmt.Sprintf("%d", v))
	case string:
		xEntry.SetText(v)
	default:
		xEntry.SetText(fmt.Sprintf("%v", v))
	}
	switch v := action.Point.Y.(type) {
	case int:
		yEntry.SetText(fmt.Sprintf("%d", v))
	case string:
		yEntry.SetText(v)
	default:
		yEntry.SetText(fmt.Sprintf("%v", v))
	}
	xEntry.SetPlaceHolder("X or ${var}")
	yEntry.SetPlaceHolder("Y or ${var}")

	colorEntry := widget.NewEntry()
	colorEntry.SetText(action.TargetColor)
	colorEntry.SetPlaceHolder("Hex e.g. ffffff or #ffffff")

	// Color swatch: shows current color and stays in sync with colorEntry
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

	// Dropper: first click activates; second click (anywhere on screen) records X, Y and color at that pixel
	dropperBtn := ttwidget.NewButtonWithIcon("", theme.DocumentCreateIcon(), func() {
		go func() {
			hook.Register(hook.MouseDown, []string{}, func(e hook.Event) {
				switch e.Button {
				case hook.MouseMap["left"]:
					x, y := robotgo.Location()
					logicalX := x - config.XOffset
					logicalY := y - config.YOffset
					hex := robotgo.GetPixelColor(x, y)
					hex = strings.TrimPrefix(strings.ToLower(hex), "#")
					if len(hex) == 8 {
						hex = hex[2:]
					}
					fyne.Do(func() {
						xEntry.SetText(fmt.Sprintf("%d", logicalX))
						yEntry.SetText(fmt.Sprintf("%d", logicalY))
						colorEntry.SetText(hex)
						updateSwatch()
					})
				default:
					// right or other: just cancel
				}
				hook.Unregister(hook.MouseDown, []string{})
			})
		}()
	})
	dropperBtn.SetToolTip("Click Dropper, then click on screen to record X, Y and color at that pixel")

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

	timeoutEntry := widget.NewEntry()
	if action.TimeoutSeconds > 0 {
		timeoutEntry.SetText(fmt.Sprintf("%d", action.TimeoutSeconds))
	} else {
		timeoutEntry.SetPlaceHolder("0 = wait indefinitely")
	}

	content := widget.NewForm(
		widget.NewFormItem("Name:", nameEntry),
		widget.NewFormItem("X:", xEntry),
		widget.NewFormItem("Y:", yEntry),
		widget.NewFormItem("Target color:", colorRow),
		widget.NewFormItem("Color tolerance:", toleranceRow),
		widget.NewFormItem("Timeout:", timeoutEntry),
	)

	saveFunc := func() {
		action.Name = strings.TrimSpace(nameEntry.Text)
		if x, err := strconv.Atoi(strings.TrimSpace(xEntry.Text)); err == nil {
			action.Point.X = x
		} else {
			action.Point.X = strings.TrimSpace(xEntry.Text)
		}
		if y, err := strconv.Atoi(strings.TrimSpace(yEntry.Text)); err == nil {
			action.Point.Y = y
		} else {
			action.Point.Y = strings.TrimSpace(yEntry.Text)
		}
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
		if s, err := strconv.Atoi(strings.TrimSpace(timeoutEntry.Text)); err == nil && s >= 0 {
			action.TimeoutSeconds = s
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
	go func() {
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
	}()

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

func createCalibrationDialogContent(action *actions.Calibration) (fyne.CanvasObject, func()) {
	nameEntry := widget.NewEntry()
	nameEntry.SetText(action.Name)
	programEntry := widget.NewEntry()
	programEntry.SetText(action.ProgramName)
	programEntry.SetPlaceHolder("Program name (e.g. from Programs tab)")
	resolutionEntry := widget.NewEntry()
	resolutionEntry.SetText(action.ResolutionKey)
	resolutionEntry.SetPlaceHolder("Leave empty for current monitor")
	rowSplitEntry := widget.NewEntry()
	rowSplitEntry.SetText(fmt.Sprintf("%d", action.RowSplit))
	colSplitEntry := widget.NewEntry()
	colSplitEntry.SetText(fmt.Sprintf("%d", action.ColSplit))
	toleranceEntry := widget.NewEntry()
	toleranceEntry.SetText(fmt.Sprintf("%g", action.Tolerance))
	blurEntry := widget.NewEntry()
	blurEntry.SetText(fmt.Sprintf("%d", action.Blur))

	tempSearchArea := action.SearchArea
	tempTargets := slices.Clone(action.Targets)

	// Search area selector: pick program then area name
	searchAreaProgramSelect := widget.NewSelect(repositories.ProgramRepo().GetAllKeys(), nil)
	if action.ProgramName != "" {
		searchAreaProgramSelect.SetSelected(action.ProgramName)
	} else if len(repositories.ProgramRepo().GetAllKeys()) > 0 {
		searchAreaProgramSelect.SetSelected(repositories.ProgramRepo().GetAllKeys()[0])
	}
	var searchAreaNameSelect *widget.Select
	refreshSearchAreaNames := func() {
		pname := searchAreaProgramSelect.Selected
		if pname == "" {
			return
		}
		p, _ := repositories.ProgramRepo().Get(pname)
		if p == nil {
			return
		}
		keys := p.SearchAreaRepo(config.MainMonitorSizeString).GetAllKeys()
		if searchAreaNameSelect == nil {
			searchAreaNameSelect = widget.NewSelect(keys, func(s string) {
				sa, _ := p.SearchAreaRepo(config.MainMonitorSizeString).Get(s)
				if sa != nil {
					tempSearchArea = actions.SearchArea{Name: sa.Name, LeftX: sa.LeftX, TopY: sa.TopY, RightX: sa.RightX, BottomY: sa.BottomY}
				}
			})
		} else {
			searchAreaNameSelect.Options = keys
			searchAreaNameSelect.Refresh()
		}
		if action.SearchArea.Name != "" {
			for _, k := range keys {
				if k == action.SearchArea.Name {
					searchAreaNameSelect.SetSelected(k)
					break
				}
			}
		}
	}
	searchAreaProgramSelect.OnChanged = func(string) { refreshSearchAreaNames() }
	refreshSearchAreaNames()

	// Targets list
	targetsList := widget.NewList(
		func() int { return len(tempTargets) },
		func() fyne.CanvasObject {
			return container.NewHBox(
				widget.NewEntry(),
				widget.NewSelect([]string{"point", "searcharea"}, nil),
				widget.NewEntry(),
			)
		},
		func(id widget.ListItemID, co fyne.CanvasObject) {
			row := co.(*fyne.Container).Objects
			if id < len(tempTargets) {
				t := tempTargets[id]
				row[0].(*widget.Entry).SetText(t.OutputName)
				row[1].(*widget.Select).SetSelected(t.OutputType)
				if row[1].(*widget.Select).Selected == "" {
					row[1].(*widget.Select).SetSelected("point")
				}
				row[2].(*widget.Entry).SetText(t.Target)
			}
			i := id
			row[0].(*widget.Entry).OnChanged = func(s string) {
				if i < len(tempTargets) {
					tempTargets[i].OutputName = s
				}
			}
			row[1].(*widget.Select).OnChanged = func(s string) {
				if i < len(tempTargets) {
					tempTargets[i].OutputType = s
				}
			}
			row[2].(*widget.Entry).OnChanged = func(s string) {
				if i < len(tempTargets) {
					tempTargets[i].Target = s
				}
			}
		},
	)
	addTargetBtn := ttwidget.NewButton("Add target", func() {
		tempTargets = append(tempTargets, actions.CalibrationTarget{OutputType: "point"})
		targetsList.Refresh()
	})
	content := container.NewVBox(
		widget.NewForm(
			widget.NewFormItem("Name:", nameEntry),
			widget.NewFormItem("Program name:", programEntry),
			widget.NewFormItem("Resolution (optional):", resolutionEntry),
			widget.NewFormItem("Search area (optional) — program:", searchAreaProgramSelect),
		),
	)
	if searchAreaNameSelect != nil {
		content.Add(widget.NewForm(widget.NewFormItem("Search area name:", searchAreaNameSelect)))
	}
	content.Add(widget.NewForm(
		widget.NewFormItem("Row split:", rowSplitEntry),
		widget.NewFormItem("Col split:", colSplitEntry),
		widget.NewFormItem("Tolerance:", toleranceEntry),
		widget.NewFormItem("Blur:", blurEntry),
	))
	content.Add(widget.NewLabel("Calibration targets (output name, type, image target e.g. program|item):"))
	content.Add(container.NewBorder(nil, nil, nil, addTargetBtn, targetsList))
	content.Add(layout.NewSpacer())

	saveFunc := func() {
		action.Name = nameEntry.Text
		action.ProgramName = programEntry.Text
		action.ResolutionKey = strings.TrimSpace(resolutionEntry.Text)
		action.SearchArea = tempSearchArea
		action.Targets = tempTargets
		if n, err := strconv.Atoi(rowSplitEntry.Text); err == nil {
			action.RowSplit = n
		}
		if n, err := strconv.Atoi(colSplitEntry.Text); err == nil {
			action.ColSplit = n
		}
		if f, err := strconv.ParseFloat(toleranceEntry.Text, 32); err == nil {
			action.Tolerance = float32(f)
		}
		if n, err := strconv.Atoi(blurEntry.Text); err == nil {
			action.Blur = n
		}
	}

	return content, saveFunc
}
