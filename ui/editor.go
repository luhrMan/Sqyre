package ui

import (
	"Sqyre/internal/config"
	"Sqyre/internal/models"
	"Sqyre/internal/services"
	"Sqyre/ui/custom_widgets"
	"errors"
	"fmt"
	"image"
	"image/color"
	"os"
	"path/filepath"
	"time"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/canvas"
	"fyne.io/fyne/v2/container"
	"fyne.io/fyne/v2/dialog"
	"fyne.io/fyne/v2/layout"
	"fyne.io/fyne/v2/theme"
	"fyne.io/fyne/v2/widget"

	"Sqyre/ui/completionentry"

	"github.com/go-vgo/robotgo"
	"gocv.io/x/gocv"
)

type EditorUi struct {
	fyne.CanvasObject
	AddButton       *widget.Button
	RemoveButton    *widget.Button
	ProgramSelector *widget.SelectEntry
	EditorTabs      struct {
		*container.AppTabs
		ProgramsTab    *EditorTab
		ItemsTab       *EditorTab
		PointsTab      *EditorTab
		SearchAreasTab *EditorTab
		MasksTab       *EditorTab
		AutoPicTab     *EditorTab
	}
}
type EditorTab struct {
	*container.TabItem
	Split *container.Split
	Left  *fyne.Container
	Right *fyne.Container

	Widgets        map[string]fyne.CanvasObject
	SelectedItem   any
	previewImage   *canvas.Image
	UpdateButton   *widget.Button
	OriginalValues map[string]string
}

func NewEditorTab(name string, left, right *fyne.Container) *container.TabItem {
	split := container.NewHSplit(left, right)
	return container.NewTabItem(name, split)
}

func (u *Ui) constructEditorTabs() {
	var (
		name  = "Name"
		x     = "X"
		y     = "Y"
		x1    = "LeftX"
		y1    = "TopY"
		x2    = "RightX"
		y2    = "BottomY"
		cols  = "Cols"
		rows  = "Rows"
		tags  = "Tags"
		sm    = "StackMax"
		form  = "Form"
		acc   = "Accordion"
		plist = "list"
		ive   = "iconVariantEditor"

		et    = ui.EditorTabs
		protw = et.ProgramsTab.Widgets
		itw   = et.ItemsTab.Widgets
		ptw   = et.PointsTab.Widgets
		satw  = et.SearchAreasTab.Widgets
	)

	protw[name] = new(widget.Entry)
	protw[plist] = new(widget.List)
	protw["searchbar"] = new(widget.Entry)
	protw[form] = widget.NewForm(
		widget.NewFormItem(name, protw[name]),
	)

	et.ProgramsTab.UpdateButton = widget.NewButton("Update", nil)
	et.ProgramsTab.UpdateButton.Importance = widget.HighImportance
	et.ProgramsTab.UpdateButton.Disable()

	et.ProgramsTab.TabItem = NewEditorTab(
		"Programs",
		container.NewBorder(protw["searchbar"], nil, nil, nil, protw[plist]),
		container.NewBorder(nil, nil, nil, nil, container.NewVBox(protw[form], container.NewHBox(layout.NewSpacer(), et.ProgramsTab.UpdateButton))),
	)

	//===========================================================================================================ITEMS
	itw[acc] = widget.NewAccordion()
	itw[name] = new(widget.Entry)
	itw[cols] = new(widget.Entry)
	itw[rows] = new(widget.Entry)
	itw["tagEntry"] = completionentry.NewCompletionEntry([]string{})
	itw["tagEntry"].(*completionentry.CompletionEntry).PlaceHolder = "Enter tag name and press Enter"
	// Create a "+" button for submitting tags
	itw["tagSubmitButton"] = widget.NewButtonWithIcon("", theme.ContentAddIcon(), nil)
	itw["tagSubmitButton"].(*widget.Button).Importance = widget.MediumImportance
	// Create container with tag entry and submit button
	itw["tagEntryContainer"] = container.NewBorder(nil, nil, nil, itw["tagSubmitButton"], itw["tagEntry"])
	itw[tags] = container.NewGridWithColumns(2) // Grid container for displaying tags
	itw[sm] = new(widget.Entry)

	// Mask selector: label showing current mask + button to open selection popup
	itw["maskLabel"] = widget.NewLabel("None")
	itw["maskSelectButton"] = widget.NewButtonWithIcon("Select", theme.SearchIcon(), nil)
	itw["maskSelectButton"].(*widget.Button).Importance = widget.MediumImportance
	itw["maskClearButton"] = widget.NewButtonWithIcon("", theme.ContentClearIcon(), nil)
	itw["maskClearButton"].(*widget.Button).Importance = widget.LowImportance
	itw["maskDetailsLabel"] = widget.NewLabel("")
	itw["maskDetailsLabel"].(*widget.Label).TextStyle = fyne.TextStyle{Italic: true}
	itw["maskContainer"] = container.NewVBox(
		container.NewBorder(nil, nil, nil,
			container.NewHBox(itw["maskSelectButton"], itw["maskClearButton"]),
			itw["maskLabel"],
		),
		itw["maskDetailsLabel"],
	)

	// Create IconVariantEditor widget
	iconService := services.IconVariantServiceInstance()
	itw[ive] = custom_widgets.NewIconVariantEditor(
		"", // programName will be set when item is selected
		"", // itemName will be set when item is selected
		iconService,
		ui.Window,
		nil, // onVariantChange callback will be set in binders
	)

	itw[form] = widget.NewForm(
		widget.NewFormItem(name, itw[name]),
		widget.NewFormItem(cols, itw[cols]),
		widget.NewFormItem(rows, itw[rows]),
		widget.NewFormItem(tags, itw["tagEntryContainer"]),
		widget.NewFormItem("", container.NewHScroll(itw[tags])),
		widget.NewFormItem(sm, itw[sm]),
		widget.NewFormItem("Mask", itw["maskContainer"]),
	)
	et.ItemsTab.UpdateButton = widget.NewButton("Update", nil)
	et.ItemsTab.UpdateButton.Importance = widget.HighImportance
	et.ItemsTab.UpdateButton.Disable()

	iveBorder := canvas.NewRectangle(color.NRGBA{})
	iveBorder.StrokeColor = theme.ButtonColor()
	iveBorder.StrokeWidth = 2
	iveBorder.CornerRadius = 4

	et.ItemsTab.TabItem = NewEditorTab(
		"Items",
		container.NewBorder(nil, nil, nil, nil, itw[acc]),
		container.NewBorder(
			container.NewVBox(itw[form], container.NewHBox(layout.NewSpacer(), et.ItemsTab.UpdateButton)),
			nil, nil, nil,
			container.NewStack(iveBorder, container.NewPadded(itw[ive])),
		),
	)

	//===========================================================================================================POINTS
	ptw[acc] = widget.NewAccordion()
	ptw[name] = new(widget.Entry)
	ptw[x] = custom_widgets.NewVarEntry(currentMacroVariables)
	ptw[y] = custom_widgets.NewVarEntry(currentMacroVariables)

	// Create record button for capturing point coordinates
	ptw["recordButton"] = widget.NewButtonWithIcon("", theme.MediaRecordIcon(), nil)
	ptw["recordButton"].(*widget.Button).Importance = widget.DangerImportance

	et.PointsTab.UpdateButton = widget.NewButton("Update", nil)
	et.PointsTab.UpdateButton.Importance = widget.HighImportance
	et.PointsTab.UpdateButton.Disable()

	ptw[form] = widget.NewForm(
		widget.NewFormItem(name, ptw[name]),
		widget.NewFormItem(x, ptw[x]),
		widget.NewFormItem(y, ptw[y]),
		widget.NewFormItem("", container.NewHBox(layout.NewSpacer(), ptw["recordButton"], et.PointsTab.UpdateButton)),
	)

	// Create preview image for Points tab
	pointPreviewImage := canvas.NewImageFromImage(nil)
	pointPreviewImage.FillMode = canvas.ImageFillContain
	pointPreviewImage.SetMinSize(fyne.NewSize(400, 300))

	// Store the image reference in the tab for later access
	et.PointsTab.previewImage = pointPreviewImage

	et.PointsTab.TabItem = NewEditorTab(
		"Points",
		container.NewBorder(nil, nil, nil, nil, ptw[acc]),
		container.NewBorder(
			ptw[form],
			nil,
			nil,
			nil,
			pointPreviewImage,
		),
	)

	//===========================================================================================================SEARCHAREAS
	satw[acc] = widget.NewAccordion()
	satw[name] = new(widget.Entry)
	satw[x1] = custom_widgets.NewVarEntry(currentMacroVariables)
	satw[y1] = custom_widgets.NewVarEntry(currentMacroVariables)
	satw[x2] = custom_widgets.NewVarEntry(currentMacroVariables)
	satw[y2] = custom_widgets.NewVarEntry(currentMacroVariables)
	// Create record button for capturing search area rectangle (click and drag)
	satw["recordButton"] = widget.NewButtonWithIcon("", theme.MediaRecordIcon(), nil)
	satw["recordButton"].(*widget.Button).Importance = widget.DangerImportance
	et.SearchAreasTab.UpdateButton = widget.NewButton("Update", nil)
	et.SearchAreasTab.UpdateButton.Importance = widget.HighImportance
	et.SearchAreasTab.UpdateButton.Disable()

	satw[form] = widget.NewForm(
		widget.NewFormItem(name, satw[name]),
		widget.NewFormItem(x1, satw[x1]),
		widget.NewFormItem(y1, satw[y1]),
		widget.NewFormItem(x2, satw[x2]),
		widget.NewFormItem(y2, satw[y2]),
		widget.NewFormItem("", container.NewHBox(layout.NewSpacer(), satw["recordButton"], et.SearchAreasTab.UpdateButton)),
	)

	// Create preview image for Search Areas tab
	searchAreaPreviewImage := canvas.NewImageFromImage(nil)
	searchAreaPreviewImage.FillMode = canvas.ImageFillContain
	searchAreaPreviewImage.SetMinSize(fyne.NewSize(400, 300))

	// Store the image reference in the tab for later access
	et.SearchAreasTab.previewImage = searchAreaPreviewImage

	et.SearchAreasTab.TabItem = NewEditorTab(
		"Search Areas",
		container.NewBorder(nil, nil, nil, nil, satw[acc]),
		container.NewBorder(
			satw[form],
			nil,
			nil,
			nil,
			searchAreaPreviewImage,
		),
	)

	//===========================================================================================================MASKS
	mtw := et.MasksTab.Widgets
	mtw["Accordion"] = widget.NewAccordion()
	mtw["Name"] = new(widget.Entry)
	mtw["uploadButton"] = widget.NewButtonWithIcon("Upload Image", theme.FolderOpenIcon(), nil)
	mtw["removeImageButton"] = widget.NewButtonWithIcon("Remove Image", theme.ContentRemoveIcon(), nil)
	mtw["removeImageButton"].(*widget.Button).Importance = widget.DangerImportance
	mtw["removeImageButton"].(*widget.Button).Hide()

	maskPreviewImage := canvas.NewImageFromImage(nil)
	maskPreviewImage.FillMode = canvas.ImageFillContain
	maskPreviewImage.SetMinSize(fyne.NewSize(400, 300))
	et.MasksTab.previewImage = maskPreviewImage

	mtw["imageStatus"] = widget.NewLabel("")
	mtw["imageStatus"].(*widget.Label).Hide()

	// Shape selector: Rectangle or Circle
	mtw["shapeSelect"] = widget.NewRadioGroup([]string{"Rectangle", "Circle"}, nil)
	mtw["shapeSelect"].(*widget.RadioGroup).Horizontal = true
	mtw["shapeSelect"].(*widget.RadioGroup).Required = true
	mtw["shapeSelect"].(*widget.RadioGroup).SetSelected("Rectangle")

	// Center location entries (percentage of template dimensions)
	mtw["CenterX"] = custom_widgets.NewVarEntry(currentMacroVariables)
	mtw["CenterX"].(*custom_widgets.VarEntry).PlaceHolder = "50"
	mtw["CenterY"] = custom_widgets.NewVarEntry(currentMacroVariables)
	mtw["CenterY"].(*custom_widgets.VarEntry).PlaceHolder = "50"
	mtw["centerContainer"] = container.NewGridWithColumns(2,
		container.NewBorder(nil, nil, widget.NewLabel("X %"), nil, mtw["CenterX"]),
		container.NewBorder(nil, nil, widget.NewLabel("Y %"), nil, mtw["CenterY"]),
	)

	// Rectangle entries:  base * height
	mtw["Base"] = custom_widgets.NewVarEntry(currentMacroVariables)
	mtw["Base"].(*custom_widgets.VarEntry).PlaceHolder = "base"
	mtw["Height"] = custom_widgets.NewVarEntry(currentMacroVariables)
	mtw["Height"].(*custom_widgets.VarEntry).PlaceHolder = "height"
	mtw["rectContainer"] =
		container.NewGridWithColumns(3,
			mtw["Base"],
			container.NewCenter(widget.NewLabel("*")),
			mtw["Height"],
		)

	// Circle entries:  π * radius²
	mtw["Radius"] = custom_widgets.NewVarEntry(currentMacroVariables)
	mtw["Radius"].(*custom_widgets.VarEntry).PlaceHolder = "radius"
	mtw["circleContainer"] = container.NewBorder(
		nil, nil,
		widget.NewLabel("π *"), widget.NewLabel("²"),
		mtw["Radius"],
	)

	// Initially show rectangle, hide circle
	mtw["circleContainer"].(*fyne.Container).Hide()

	// Shape container holds whichever is active
	mtw["shapeParamsContainer"] = container.NewVBox(
		mtw["rectContainer"],
		mtw["circleContainer"],
	)

	// Toggle visibility when shape changes
	mtw["shapeSelect"].(*widget.RadioGroup).OnChanged = func(selected string) {
		switch selected {
		case "Rectangle":
			mtw["rectContainer"].(*fyne.Container).Show()
			mtw["circleContainer"].(*fyne.Container).Hide()
		case "Circle":
			mtw["rectContainer"].(*fyne.Container).Hide()
			mtw["circleContainer"].(*fyne.Container).Show()
		}
	}

	mtw["Inverse"] = widget.NewCheck("Inverse (shape included, rest excluded)", nil)

	et.MasksTab.UpdateButton = widget.NewButton("Update", nil)
	et.MasksTab.UpdateButton.Importance = widget.HighImportance
	et.MasksTab.UpdateButton.Disable()

	mtw["Form"] = widget.NewForm(
		widget.NewFormItem("Name", mtw["Name"]),
		widget.NewFormItem("Shape", mtw["shapeSelect"]),
		widget.NewFormItem("Center", mtw["centerContainer"]),
		widget.NewFormItem("", mtw["shapeParamsContainer"]),
		widget.NewFormItem("", mtw["Inverse"]),
	)

	et.MasksTab.TabItem = NewEditorTab(
		"Masks",
		container.NewBorder(nil, nil, nil, nil, mtw["Accordion"]),
		container.NewBorder(
			container.NewVBox(
				mtw["Form"],
				container.NewHBox(layout.NewSpacer(), et.MasksTab.UpdateButton),
				container.NewHBox(mtw["uploadButton"], mtw["removeImageButton"]),
				mtw["imageStatus"],
			),
			nil, nil, nil,
			maskPreviewImage,
		),
	)

	//===========================================================================================================AUTOPIC
	atw := et.AutoPicTab.Widgets
	atw["Accordion"] = widget.NewAccordion()
	atw["saveButton"] = widget.NewButton("Save", u.onAutoPicSave)

	// Create preview image and container
	previewImage := canvas.NewImageFromImage(nil)
	previewImage.FillMode = canvas.ImageFillContain
	previewImage.SetMinSize(fyne.NewSize(400, 300))

	// Store the image reference in the tab for later access
	et.AutoPicTab.previewImage = previewImage

	// Initially disable save button
	atw["saveButton"].(*widget.Button).Disable()

	et.AutoPicTab.TabItem = NewEditorTab(
		"AutoPic",
		container.NewBorder(
			nil,
			nil,
			nil,
			nil,
			atw["Accordion"],
		),
		container.NewBorder(
			nil,
			atw["saveButton"],
			nil,
			nil,
			previewImage,
		),
	)

	et.Append(et.ProgramsTab.TabItem)
	et.Append(et.ItemsTab.TabItem)
	et.Append(et.PointsTab.TabItem)
	et.Append(et.SearchAreasTab.TabItem)
	et.Append(et.MasksTab.TabItem)
	et.Append(et.AutoPicTab.TabItem)
}

func (u *Ui) constructAddButton() {
	u.EditorUi.AddButton.Text = "New"
	u.EditorUi.AddButton.Icon = theme.ContentAddIcon()
	u.EditorUi.AddButton.Importance = widget.SuccessImportance

}

func (u *Ui) constructRemoveButton() {
	u.EditorUi.RemoveButton.Text = ""
	u.EditorUi.RemoveButton.Icon = theme.ContentRemoveIcon()
	u.EditorUi.RemoveButton.Importance = widget.DangerImportance
}

// AutoPic tab handlers

func (u *Ui) onAutoPicSave() {
	selectedItem := u.EditorTabs.AutoPicTab.SelectedItem
	if selectedItem == nil {
		dialog.ShowError(errors.New("AutoPic: Cannot save - no search area selected"), u.Window)
		return
	}

	searchArea, ok := selectedItem.(*models.SearchArea)
	if !ok {
		dialog.ShowError(errors.New("AutoPic: Cannot save - selected item is not a search area"), u.Window)
		return
	}

	// Validate search area
	if searchArea == nil {
		dialog.ShowError(errors.New("AutoPic: Cannot save - search area is nil"), u.Window)
		return
	}

	// Validate search area dimensions (variable refs yield 0 for preview)
	lx := searchAreaCoordToInt(searchArea.LeftX)
	ty := searchAreaCoordToInt(searchArea.TopY)
	rx := searchAreaCoordToInt(searchArea.RightX)
	by := searchAreaCoordToInt(searchArea.BottomY)
	w := rx - lx
	h := by - ty

	if w <= 0 || h <= 0 {
		dialog.ShowError(fmt.Errorf("AutoPic: Cannot save - invalid search area dimensions - width: %d, height: %d (area: %s)", w, h, searchArea.Name), u.Window)
		return
	}

	// Validate coordinates are within reasonable bounds
	screenWidth := config.MonitorWidth
	screenHeight := config.MonitorHeight

	if lx < 0 || ty < 0 ||
		rx > screenWidth || by > screenHeight {
		dialog.ShowError(fmt.Errorf("AutoPic: Cannot save - search area coordinates out of screen bounds - screen: %dx%d, area: (%d,%d) to (%d,%d) (area: %s)",
			screenWidth, screenHeight, lx, ty, rx, by, searchArea.Name), u.Window)
		return
	}

	// Attempt to capture the screen area with error recovery
	var captureImg image.Image
	var err error
	func() {
		defer func() {
			if r := recover(); r != nil {
				services.LogPanicToFile(r, "AutoPic: Screen capture during save (area: "+searchArea.Name+")")
				captureImg = nil
			}
		}()

		captureImg, err = robotgo.CaptureImg(lx, ty, w, h)
		if err != nil {
			dialog.ShowError(fmt.Errorf("AutoPic: Error capturing image - %v (area: %s)", err, searchArea.Name), u.Window)
			captureImg = nil
		}
	}()

	// Validate the captured image
	if captureImg == nil {
		dialog.ShowError(fmt.Errorf("AutoPic: Cannot save - screen capture returned nil image (area: %s)", searchArea.Name), u.Window)
		return
	}

	// Create filename with timestamp
	timestamp := time.Now().Format("20060102_150405")
	filename := fmt.Sprintf("%s_%s.png", timestamp, searchArea.Name)

	// Ensure AutoPic directory exists
	autoPicPath := config.GetAutoPicPath()
	if err := os.MkdirAll(autoPicPath, 0755); err != nil {
		dialog.ShowError(fmt.Errorf("AutoPic: Error creating AutoPic directory: %v", err), u.Window)
		return
	}

	// Validate the path
	fullPath := filepath.Join(autoPicPath, filename)
	if fullPath == "" {
		dialog.ShowError(errors.New("AutoPic: Error creating file path"), u.Window)
		return
	}

	// Save the image with error handling
	func() {
		defer func() {
			if r := recover(); r != nil {
				services.LogPanicToFile(r, "AutoPic: Image save (path: "+fullPath+")")
			}
		}()

		if err := robotgo.SavePng(captureImg, fullPath); err != nil {
			dialog.ShowError(fmt.Errorf("AutoPic: Error saving image to %s: %v", fullPath, err), u.Window)
			return
		}

		dialog.ShowError(fmt.Errorf("AutoPic: Image saved successfully to: %s", fullPath), u.Window)
	}()
}

func (u *Ui) UpdateAutoPicPreview(searchArea *models.SearchArea) {
	// Validate search area
	if searchArea == nil {
		dialog.ShowError(errors.New("AutoPic: Cannot update preview - search area is nil"), u.Window)
		return
	}

	// Validate search area dimensions (variable refs yield 0 for preview)
	lx := searchAreaCoordToInt(searchArea.LeftX)
	ty := searchAreaCoordToInt(searchArea.TopY)
	rx := searchAreaCoordToInt(searchArea.RightX)
	by := searchAreaCoordToInt(searchArea.BottomY)
	w := rx - lx
	h := by - ty

	if w <= 0 || h <= 0 {
		dialog.ShowError(fmt.Errorf("AutoPic: Invalid search area dimensions - width: %d, height: %d (area: %s)", w, h, searchArea.Name), u.Window)
		u.clearPreviewImage()
		return
	}

	// Validate coordinates are within reasonable bounds
	screenWidth := config.MonitorWidth
	screenHeight := config.MonitorHeight

	if lx < 0 || ty < 0 ||
		rx > screenWidth || by > screenHeight {
		dialog.ShowError(fmt.Errorf("AutoPic: Search area coordinates out of screen bounds - screen: %dx%d, area: (%d,%d) to (%d,%d) (area: %s)",
			screenWidth, screenHeight, lx, ty, rx, by, searchArea.Name), u.Window)
		u.clearPreviewImage()
		return
	}

	// Attempt to capture the screen area with error recovery
	defer func() {
		if r := recover(); r != nil {
			services.LogPanicToFile(r, "AutoPic: Screen capture (area: "+searchArea.Name+")")
			u.clearPreviewImage()
		}
	}()

	captureImg, err := robotgo.CaptureImg(lx, ty, w, h)
	if err != nil {
		dialog.ShowError(fmt.Errorf("AutoPic: Error capturing image - %v (area: %s)", err, searchArea.Name), u.Window)
		captureImg = nil
	}

	// Validate the captured image
	if captureImg == nil {
		dialog.ShowError(fmt.Errorf("AutoPic: Screen capture returned nil image (area: %s)", searchArea.Name), u.Window)
		u.clearPreviewImage()
		return
	}

	// Update preview image
	if previewImage := u.EditorTabs.AutoPicTab.previewImage; previewImage != nil {
		previewImage.Image = captureImg
		previewImage.Refresh()
	} else {
		dialog.ShowError(errors.New("AutoPic: Preview image widget is nil"), u.Window)
	}
}

func (u *Ui) clearPreviewImage() {
	if previewImage := u.EditorTabs.AutoPicTab.previewImage; previewImage != nil {
		previewImage.Image = nil
		previewImage.Refresh()
	}
}

func (u *Ui) ErrorPopUp(s string) {
	label := widget.NewLabel(s)
	label.Importance = widget.DangerImportance

	pu := widget.NewPopUp(
		container.NewBorder(
			nil, nil,
			widget.NewIcon(theme.CancelIcon()),
			nil,
			label,
		),
		u.Window.Canvas(),
	)
	pu.Show()
}

func (u *Ui) UpdateSearchAreaPreview(searchArea *models.SearchArea) {
	u.EditorTabs.SearchAreasTab.previewImage.Resource = nil
	// Validate search area
	if searchArea == nil {
		dialog.ShowError(errors.New("SearchArea: Cannot update preview - search area is nil"), u.Window)
		return
	}

	// Validate search area dimensions (variable refs yield 0 for preview)
	lx := searchAreaCoordToInt(searchArea.LeftX)
	ty := searchAreaCoordToInt(searchArea.TopY)
	rx := searchAreaCoordToInt(searchArea.RightX)
	by := searchAreaCoordToInt(searchArea.BottomY)
	w := rx - lx
	h := by - ty

	if w <= 0 || h <= 0 {
		u.clearSearchAreaPreviewImage()
		u.EditorTabs.SearchAreasTab.previewImage.Resource = theme.BrokenImageIcon()
		u.ErrorPopUp(fmt.Sprintf("SearchArea: Invalid search area dimensions - width: %d, height: %d (area: %s)", w, h, searchArea.Name))
		// label := widget.NewLabel(fmt.Sprintf("SearchArea: Invalid search area dimensions - width: %d, height: %d (area: %s)", w, h, searchArea.Name))
		// label.Importance = widget.DangerImportance

		// pu := widget.NewPopUp(
		// 	container.NewBorder(
		// 		nil, nil,
		// 		widget.NewIcon(theme.CancelIcon()),
		// 		nil,
		// 		label,
		// 	),
		// 	u.Window.Canvas(),
		// )
		// pu.Show()
		// dialog.ShowError(fmt.Errorf("SearchArea: Invalid search area dimensions - width: %d, height: %d (area: %s)", w, h, searchArea.Name), u.Window)
		return
	}

	// Validate coordinates are within reasonable bounds
	screenWidth := config.MonitorWidth
	screenHeight := config.MonitorHeight

	if lx < 0 || ty < 0 ||
		rx > screenWidth || by > screenHeight {
		dialog.ShowError(fmt.Errorf("SearchArea: Search area coordinates out of screen bounds - screen: %dx%d, area: (%d,%d) to (%d,%d) (area: %s)",
			screenWidth, screenHeight, lx, ty, rx, by, searchArea.Name), u.Window)
		u.clearSearchAreaPreviewImage()
		return
	}

	// Attempt to capture the full screen with error recovery
	defer func() {
		if r := recover(); r != nil {
			services.LogPanicToFile(r, "SearchArea: Screen capture (area: "+searchArea.Name+")")
			u.clearSearchAreaPreviewImage()
		}
	}()

	captureImg, err := robotgo.CaptureImg(0, 0, screenWidth, screenHeight)
	if err != nil {
		dialog.ShowError(fmt.Errorf("SearchArea: Error capturing image - %v (area: %s)", err, searchArea.Name), u.Window)
		captureImg = nil
	}

	// Validate the captured image
	if captureImg == nil {
		dialog.ShowError(fmt.Errorf("SearchArea: Screen capture returned nil image (area: %s)", searchArea.Name), u.Window)
		u.clearSearchAreaPreviewImage()
		return
	}

	// Convert to gocv Mat for drawing
	mat, err := gocv.ImageToMatRGB(captureImg)
	if err != nil {
		dialog.ShowError(fmt.Errorf("SearchArea: Error converting image to Mat - %v (area: %s)", err, searchArea.Name), u.Window)
		u.clearSearchAreaPreviewImage()
		return
	}
	defer mat.Close()

	// Draw red rectangle showing search area boundaries
	rect := image.Rect(lx, ty, rx, by)
	redColor := color.RGBA{R: 255, A: 255}
	gocv.Rectangle(&mat, rect, redColor, 2)

	// Convert back to image.Image
	previewImg, err := mat.ToImage()
	if err != nil {
		dialog.ShowError(fmt.Errorf("SearchArea: Error converting Mat to image - %v (area: %s)", err, searchArea.Name), u.Window)
		u.clearSearchAreaPreviewImage()
		return
	}

	// Update preview image
	if previewImage := u.EditorTabs.SearchAreasTab.previewImage; previewImage != nil {
		previewImage.Image = previewImg
		previewImage.Refresh()
	} else {
		dialog.ShowError(errors.New("SearchArea: Preview image widget is nil"), u.Window)
	}
}

func (u *Ui) clearSearchAreaPreviewImage() {
	if previewImage := u.EditorTabs.SearchAreasTab.previewImage; previewImage != nil {
		previewImage.Image = nil
		previewImage.Refresh()
	}
}

// pointCoordToIntForPreview returns an int for preview drawing; literal ints are used, variable refs (string) yield 0.
func pointCoordToIntForPreview(v any) int {
	switch val := v.(type) {
	case int:
		return val
	case float64:
		return int(val)
	default:
		return 0
	}
}

// searchAreaCoordToInt returns an int for preview/validation; literal ints are used, variable refs (string) yield 0.
func searchAreaCoordToInt(v any) int {
	switch val := v.(type) {
	case int:
		return val
	case float64:
		return int(val)
	default:
		return 0
	}
}

func (u *Ui) UpdatePointPreview(point *models.Point) {
	// Validate point
	if point == nil {
		dialog.ShowError(errors.New("Point: Cannot update preview - point is nil"), u.Window)
		return
	}

	px := pointCoordToIntForPreview(point.X)
	py := pointCoordToIntForPreview(point.Y)

	// Validate coordinates are within reasonable bounds
	screenWidth := config.MonitorWidth
	screenHeight := config.MonitorHeight

	if px < 0 || py < 0 || px > screenWidth || py > screenHeight {
		dialog.ShowError(fmt.Errorf("Point: Point coordinates out of screen bounds - screen: %dx%d, point: (%d,%d) (point: %s)",
			screenWidth, screenHeight, px, py, point.Name), u.Window)
		u.clearPointPreviewImage()
		return
	}

	// Attempt to capture the full screen with error recovery
	defer func() {
		if r := recover(); r != nil {
			services.LogPanicToFile(r, "Point: Screen capture (point: "+point.Name+")")
			u.clearPointPreviewImage()
		}
	}()

	captureImg, err := robotgo.CaptureImg(0, 0, screenWidth, screenHeight)
	if err != nil {
		dialog.ShowError(fmt.Errorf("Point: Error capturing image - %v (point: %s)", err, point.Name), u.Window)
		captureImg = nil
	}
	// Validate the captured image
	if captureImg == nil {
		dialog.ShowError(fmt.Errorf("Point: Screen capture returned nil image (point: %s)", point.Name), u.Window)
		u.clearPointPreviewImage()
		return
	}

	// Convert to gocv Mat for drawing
	mat, err := gocv.ImageToMatRGB(captureImg)
	if err != nil {
		dialog.ShowError(fmt.Errorf("Point: Error converting image to Mat - %v (point: %s)", err, point.Name), u.Window)
		u.clearPointPreviewImage()
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
		dialog.ShowError(fmt.Errorf("Point: Error converting Mat to image - %v (point: %s)", err, point.Name), u.Window)
		u.clearPointPreviewImage()
		return
	}

	// Update preview image
	if previewImage := u.EditorTabs.PointsTab.previewImage; previewImage != nil {
		previewImage.Image = previewImg
		fyne.DoAndWait(func() {
			previewImage.Refresh()
		})
	} else {
		dialog.ShowError(errors.New("Point: Preview image widget is nil"), u.Window)
	}
}

func (u *Ui) clearPointPreviewImage() {
	if previewImage := u.EditorTabs.PointsTab.previewImage; previewImage != nil {
		previewImage.Image = nil
		previewImage.Refresh()
	}
}

// UpdateMaskPreview loads and displays the mask image for the given program and mask name.
func (u *Ui) UpdateMaskPreview(programName, maskName string) {
	masksPath := config.GetMasksPath()
	imgPath := filepath.Join(masksPath, programName, maskName+config.PNG)

	if _, err := os.Stat(imgPath); err != nil {
		u.ClearMaskPreviewImage()
		return
	}

	mat := gocv.IMRead(imgPath, gocv.IMReadColor)
	if mat.Empty() {
		u.ClearMaskPreviewImage()
		return
	}
	defer mat.Close()

	img, err := mat.ToImage()
	if err != nil {
		u.ClearMaskPreviewImage()
		return
	}

	if previewImage := u.EditorTabs.MasksTab.previewImage; previewImage != nil {
		previewImage.Image = img
		previewImage.Refresh()
	}
}

func (u *Ui) ClearMaskPreviewImage() {
	if previewImage := u.EditorTabs.MasksTab.previewImage; previewImage != nil {
		previewImage.Image = nil
		previewImage.Refresh()
	}
}

// SetMaskImageMode switches the right-side UI between variable entry and uploaded image display.
// When hasImage is true, the value/shape entries are hidden and the image status + remove button are shown.
func (u *Ui) SetMaskImageMode(hasImage bool) {
	mtw := u.EditorTabs.MasksTab.Widgets
	if hasImage {
		mtw["shapeSelect"].(*widget.RadioGroup).Hide()
		mtw["centerContainer"].(*fyne.Container).Hide()
		mtw["shapeParamsContainer"].(*fyne.Container).Hide()
		mtw["imageStatus"].(*widget.Label).SetText("Image mask uploaded")
		mtw["imageStatus"].(*widget.Label).Show()
		mtw["removeImageButton"].(*widget.Button).Show()
	} else {
		mtw["shapeSelect"].(*widget.RadioGroup).Show()
		mtw["centerContainer"].(*fyne.Container).Show()
		mtw["shapeParamsContainer"].(*fyne.Container).Show()
		selected := mtw["shapeSelect"].(*widget.RadioGroup).Selected
		switch selected {
		case "Circle":
			mtw["rectContainer"].(*fyne.Container).Hide()
			mtw["circleContainer"].(*fyne.Container).Show()
		default:
			mtw["rectContainer"].(*fyne.Container).Show()
			mtw["circleContainer"].(*fyne.Container).Hide()
		}
		mtw["imageStatus"].(*widget.Label).Hide()
		mtw["removeImageButton"].(*widget.Button).Hide()
	}
}

// HasMaskImage checks if an image file exists for the given program and mask.
func HasMaskImage(programName, maskName string) bool {
	masksPath := config.GetMasksPath()
	imgPath := filepath.Join(masksPath, programName, maskName+config.PNG)
	_, err := os.Stat(imgPath)
	return err == nil
}

func (u *Ui) RefreshAutoPicSearchAreas() {
	// Reset the selected item and disable save button
	u.EditorTabs.AutoPicTab.SelectedItem = nil
	if saveButton, ok := u.EditorTabs.AutoPicTab.Widgets["saveButton"].(*widget.Button); ok {
		saveButton.Disable()
	}
	// Clear preview image
	if previewImage := u.EditorTabs.AutoPicTab.previewImage; previewImage != nil {
		previewImage.Image = nil
		previewImage.Refresh()
	}
}
