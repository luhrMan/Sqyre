package ui

import (
	"Squire/internal/config"
	"Squire/internal/models"
	"Squire/internal/services"
	"Squire/ui/custom_widgets"
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
	"fyne.io/fyne/v2/theme"
	"fyne.io/fyne/v2/widget"
	fynetooltip "github.com/dweymouth/fyne-tooltip"
	"github.com/go-vgo/robotgo"
	"gocv.io/x/gocv"
)

type EditorUi struct {
	fyne.CanvasObject
	NavButton       *widget.Button
	AddButton       *widget.Button
	RemoveButton    *widget.Button
	ProgramSelector *widget.SelectEntry
	EditorTabs      struct {
		*container.AppTabs
		ProgramsTab    *EditorTab
		ItemsTab       *EditorTab
		PointsTab      *EditorTab
		SearchAreasTab *EditorTab
		AutoPicTab     *EditorTab
	}
}
type EditorTab struct {
	*container.TabItem
	Split *container.Split
	Left  *fyne.Container
	Right *fyne.Container

	Widgets      map[string]fyne.CanvasObject
	SelectedItem any
	previewImage *canvas.Image // For AutoPic tab
}

func NewEditorTab(name string, left, right *fyne.Container) *container.TabItem {
	split := container.NewHSplit(left, right)
	return container.NewTabItem(name, split)
}

func (u *Ui) constructEditorTabs() {
	var (
		name = "Name"
		x    = "X"
		y    = "Y"
		x1   = "LeftX"
		y1   = "TopY"
		x2   = "RightX"
		y2   = "BottomY"
		cols = "Cols"
		rows = "Rows"
		tags = "Tags"
		sm   = "StackMax"
		// m    = "Merchant"
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
	protw[form].(*widget.Form).SubmitText = "Update"

	et.ProgramsTab.TabItem = NewEditorTab(
		"Programs",
		container.NewBorder(protw["searchbar"], nil, nil, nil, protw[plist]),
		container.NewBorder(nil, nil, nil, nil, protw[form]),
	)

	//===========================================================================================================ITEMS
	itw[acc] = widget.NewAccordion()
	itw[name] = new(widget.Entry)
	itw[cols] = new(widget.Entry)
	itw[rows] = new(widget.Entry)
	itw["tagEntry"] = widget.NewEntry()
	itw["tagEntry"].(*widget.Entry).PlaceHolder = "Enter tag name and press Enter"
	itw[tags] = container.NewGridWithColumns(3) // Grid container for displaying tags
	itw[sm] = new(widget.Entry)

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
		widget.NewFormItem(tags, itw["tagEntry"]),
		widget.NewFormItem("", itw[tags]),
		widget.NewFormItem(sm, itw[sm]),
		// widget.NewFormItem(m, ui.EditorTabs.ItemsTab.Widgets[m]),
		// widget.NewFormItem("icons", container.NewGridWithRows(2, widget.NewIcon(theme.MediaFastForwardIcon()))),
	)
	itw[form].(*widget.Form).SubmitText = "Update"

	et.ItemsTab.TabItem = NewEditorTab(
		"Items",
		container.NewBorder(nil, nil, nil, nil, itw[acc]),
		container.NewBorder(itw[form], nil, nil, nil, itw[ive]),
	)

	//===========================================================================================================POINTS
	ptw[acc] = widget.NewAccordion()
	ptw[name] = new(widget.Entry)
	ptw[x] = new(widget.Entry)
	ptw[y] = new(widget.Entry)
	ptw[form] = widget.NewForm(
		widget.NewFormItem(name, ptw[name]),
		widget.NewFormItem(x, ptw[x]),
		widget.NewFormItem(y, ptw[y]),
	)

	ptw[form].(*widget.Form).SubmitText = "Update"

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
	satw[x1] = new(widget.Entry)
	satw[y1] = new(widget.Entry)
	satw[x2] = new(widget.Entry)
	satw[y2] = new(widget.Entry)
	satw[form] = widget.NewForm(
		widget.NewFormItem(name, satw[name]),
		widget.NewFormItem(x1, satw[x1]),
		widget.NewFormItem(y1, satw[y1]),
		widget.NewFormItem(x2, satw[x2]),
		widget.NewFormItem(y2, satw[y2]),
	)
	satw[form].(*widget.Form).SubmitText = "Update"

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
	et.Append(et.AutoPicTab.TabItem)
}

func (u *Ui) constructNavButton() {
	u.EditorUi.NavButton.Text = "Back"
	u.EditorUi.NavButton.Icon = theme.NavigateBackIcon()
	u.EditorUi.NavButton.OnTapped = func() {
		u.Window.SetContent(fynetooltip.AddWindowToolTipLayer(u.MainUi.CanvasObject, u.Window.Canvas()))
	}
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

	// Validate search area dimensions
	w := searchArea.RightX - searchArea.LeftX
	h := searchArea.BottomY - searchArea.TopY

	if w <= 0 || h <= 0 {
		dialog.ShowError(fmt.Errorf("AutoPic: Cannot save - invalid search area dimensions - width: %d, height: %d (area: %s)", w, h, searchArea.Name), u.Window)
		return
	}

	// Validate coordinates are within reasonable bounds
	screenWidth := config.MonitorWidth
	screenHeight := config.MonitorHeight

	if searchArea.LeftX < 0 || searchArea.TopY < 0 ||
		searchArea.RightX > screenWidth || searchArea.BottomY > screenHeight {
		dialog.ShowError(fmt.Errorf("AutoPic: Cannot save - search area coordinates out of screen bounds - screen: %dx%d, area: (%d,%d) to (%d,%d) (area: %s)",
			screenWidth, screenHeight, searchArea.LeftX, searchArea.TopY, searchArea.RightX, searchArea.BottomY, searchArea.Name), u.Window)
		return
	}

	// Calculate capture coordinates with offsets
	captureX := searchArea.LeftX + config.XOffset
	captureY := searchArea.TopY + config.YOffset

	// Additional validation for capture coordinates
	if captureX < 0 || captureY < 0 {
		dialog.ShowError(fmt.Errorf("AutoPic: Cannot save - invalid capture coordinates after offset - x: %d, y: %d (area: %s)", captureX, captureY, searchArea.Name), u.Window)
		return
	}

	// Attempt to capture the screen area with error recovery
	var captureImg image.Image
	func() {
		defer func() {
			if r := recover(); r != nil {
				dialog.ShowError(fmt.Errorf("AutoPic: Screen capture panic recovered during save - %v (area: %s)", r, searchArea.Name), u.Window)
				captureImg = nil
			}
		}()

		captureImg = robotgo.CaptureImg(captureX, captureY, w, h)
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
				dialog.ShowError(fmt.Errorf("AutoPic: Image save panic recovered - %v (path: %s)", r, fullPath), u.Window)
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

	// Validate search area dimensions
	w := searchArea.RightX - searchArea.LeftX
	h := searchArea.BottomY - searchArea.TopY

	if w <= 0 || h <= 0 {
		dialog.ShowError(fmt.Errorf("AutoPic: Invalid search area dimensions - width: %d, height: %d (area: %s)", w, h, searchArea.Name), u.Window)
		u.clearPreviewImage()
		return
	}

	// Validate coordinates are within reasonable bounds
	screenWidth := config.MonitorWidth
	screenHeight := config.MonitorHeight

	if searchArea.LeftX < 0 || searchArea.TopY < 0 ||
		searchArea.RightX > screenWidth || searchArea.BottomY > screenHeight {
		dialog.ShowError(fmt.Errorf("AutoPic: Search area coordinates out of screen bounds - screen: %dx%d, area: (%d,%d) to (%d,%d) (area: %s)",
			screenWidth, screenHeight, searchArea.LeftX, searchArea.TopY, searchArea.RightX, searchArea.BottomY, searchArea.Name), u.Window)
		u.clearPreviewImage()
		return
	}

	// Calculate capture coordinates with offsets
	captureX := searchArea.LeftX + config.XOffset
	captureY := searchArea.TopY + config.YOffset

	// Additional validation for capture coordinates
	if captureX < 0 || captureY < 0 {
		dialog.ShowError(fmt.Errorf("AutoPic: Invalid capture coordinates after offset - x: %d, y: %d (area: %s)", captureX, captureY, searchArea.Name), u.Window)
		u.clearPreviewImage()
		return
	}

	// Attempt to capture the screen area with error recovery
	defer func() {
		if r := recover(); r != nil {
			dialog.ShowError(fmt.Errorf("AutoPic: Screen capture panic recovered - %v (area: %s)", r, searchArea.Name), u.Window)
			u.clearPreviewImage()
		}
	}()

	captureImg := robotgo.CaptureImg(captureX, captureY, w, h)

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

	// Validate search area dimensions
	w := searchArea.RightX - searchArea.LeftX
	h := searchArea.BottomY - searchArea.TopY

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

	if searchArea.LeftX < 0 || searchArea.TopY < 0 ||
		searchArea.RightX > screenWidth || searchArea.BottomY > screenHeight {
		dialog.ShowError(fmt.Errorf("SearchArea: Search area coordinates out of screen bounds - screen: %dx%d, area: (%d,%d) to (%d,%d) (area: %s)",
			screenWidth, screenHeight, searchArea.LeftX, searchArea.TopY, searchArea.RightX, searchArea.BottomY, searchArea.Name), u.Window)
		u.clearSearchAreaPreviewImage()
		return
	}

	// Calculate capture coordinates with offsets

	// Attempt to capture the full screen with error recovery
	defer func() {
		if r := recover(); r != nil {
			dialog.ShowError(fmt.Errorf("SearchArea: Screen capture panic recovered - %v (area: %s)", r, searchArea.Name), u.Window)
			u.clearSearchAreaPreviewImage()
		}
	}()

	captureImg := robotgo.CaptureImg(config.XOffset, config.YOffset, screenWidth, screenHeight)

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
	rect := image.Rect(searchArea.LeftX, searchArea.TopY, searchArea.RightX, searchArea.BottomY)
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

func (u *Ui) UpdatePointPreview(point *models.Point) {
	// Validate point
	if point == nil {
		dialog.ShowError(errors.New("Point: Cannot update preview - point is nil"), u.Window)
		return
	}

	// Validate coordinates are within reasonable bounds
	screenWidth := config.MonitorWidth
	screenHeight := config.MonitorHeight

	if point.X < 0 || point.Y < 0 ||
		point.X > screenWidth || point.Y > screenHeight {
		dialog.ShowError(fmt.Errorf("Point: Point coordinates out of screen bounds - screen: %dx%d, point: (%d,%d) (point: %s)",
			screenWidth, screenHeight, point.X, point.Y, point.Name), u.Window)
		u.clearPointPreviewImage()
		return
	}

	// Attempt to capture the full screen with error recovery
	defer func() {
		if r := recover(); r != nil {
			dialog.ShowError(fmt.Errorf("Point: Screen capture panic recovered - %v (point: %s)", r, point.Name), u.Window)
			u.clearPointPreviewImage()
		}
	}()

	captureImg := robotgo.CaptureImg(config.XOffset, config.YOffset, screenWidth, screenHeight)

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
	center := image.Point{X: point.X, Y: point.Y}
	redColor := color.RGBA{R: 255, A: 255}

	// Draw circle
	gocv.Circle(&mat, center, 8, redColor, 2)

	// Draw crosshair lines
	// Horizontal line
	gocv.Line(&mat, image.Point{X: point.X - 15, Y: point.Y}, image.Point{X: point.X + 15, Y: point.Y}, redColor, 2)
	// Vertical line
	gocv.Line(&mat, image.Point{X: point.X, Y: point.Y - 15}, image.Point{X: point.X, Y: point.Y + 15}, redColor, 2)

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
		previewImage.Refresh()
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
