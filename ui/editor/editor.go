package editor

import (
	"Sqyre/internal/config"
	"Sqyre/internal/models"
	"Sqyre/internal/screen"
	"Sqyre/ui/custom_widgets"
	"errors"
	"fmt"
	"image"
	"image/color"
	"log"
	"os"
	"path/filepath"
	"time"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/canvas"
	"fyne.io/fyne/v2/container"
	"fyne.io/fyne/v2/layout"
	"fyne.io/fyne/v2/theme"
	"fyne.io/fyne/v2/widget"

	"github.com/go-vgo/robotgo"
	"gocv.io/x/gocv"
)

type EditorUi struct {
	fyne.CanvasObject
	win          fyne.Window
	AddButton    *widget.Button
	RemoveButton *widget.Button
	ActionBar    *fyne.Container
	EditorTabs   struct {
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

	ProgramSelector      *widget.Select
	Widgets              map[string]fyne.CanvasObject
	SelectedItem         any
	previewPanel         *editorPreviewPanel
	UpdateButton         *widget.Button
	PreviewRefreshButton *widget.Button
	OriginalValues       map[string]string
}

func NewEditorTab(name string, left, right fyne.CanvasObject) *container.TabItem {
	split := container.NewHSplit(left, right)
	return container.NewTabItem(name, split)
}

// LabeledProgramSelector wraps a ProgramSelector with a "Program" label to its left.
func LabeledProgramSelector(sel *widget.Select) *fyne.Container {
	lbl := widget.NewLabel("Program")
	lbl.TextStyle = fyne.TextStyle{Bold: true}
	return container.NewBorder(nil, nil, lbl, nil, sel)
}

// ActiveProgramName returns the ProgramSelector Selected value from the currently selected editor tab.
func (eu *EditorUi) ActiveProgramName() string {
	tab := eu.ActiveTab()
	if tab == nil || tab.ProgramSelector == nil {
		return ""
	}
	return tab.ProgramSelector.Selected
}

// ActiveTab returns the EditorTab corresponding to the currently selected AppTabs tab.
func (eu *EditorUi) ActiveTab() *EditorTab {
	sel := eu.EditorTabs.Selected()
	if sel == nil {
		return nil
	}
	switch sel.Text {
	case "Programs":
		return eu.EditorTabs.ProgramsTab
	case "Items":
		return eu.EditorTabs.ItemsTab
	case "Points":
		return eu.EditorTabs.PointsTab
	case "Search Areas":
		return eu.EditorTabs.SearchAreasTab
	case "Masks":
		return eu.EditorTabs.MasksTab
	case "AutoPic":
		return eu.EditorTabs.AutoPicTab
	}
	return nil
}

// wrapEditorPreviewImage adds a themed border around editor preview imagery (same treatment as the Items tab icon editor).
func wrapEditorPreviewImage(inner fyne.CanvasObject) fyne.CanvasObject {
	border := canvas.NewRectangle(color.NRGBA{})
	border.StrokeColor = theme.ButtonColor()
	border.StrokeWidth = 2
	border.CornerRadius = 4
	return container.NewStack(border, inner)
}

const (
	editorPreviewMonitorDash = 10
	editorPreviewMonitorGap  = 6
)

// editorPreviewMonitorOutline is the dotted monitor bezel color (matches search-area / point accent).
var editorPreviewMonitorOutline = color.RGBA{R: 255, A: 255}

func drawPreviewDottedHLine(mat *gocv.Mat, y, x0, x1 int, c color.RGBA, thick int) {
	if x0 > x1 {
		return
	}
	step := editorPreviewMonitorDash + editorPreviewMonitorGap
	for x := x0; x <= x1; x += step {
		xe := x + editorPreviewMonitorDash - 1
		if xe > x1 {
			xe = x1
		}
		gocv.Line(mat, image.Pt(x, y), image.Pt(xe, y), c, thick)
	}
}

func drawPreviewDottedVLine(mat *gocv.Mat, x, y0, y1 int, c color.RGBA, thick int) {
	if y0 > y1 {
		return
	}
	step := editorPreviewMonitorDash + editorPreviewMonitorGap
	for y := y0; y <= y1; y += step {
		ye := y + editorPreviewMonitorDash - 1
		if ye > y1 {
			ye = y1
		}
		gocv.Line(mat, image.Pt(x, y), image.Pt(x, ye), c, thick)
	}
}

func drawPreviewDottedRectOutline(mat *gocv.Mat, r image.Rectangle, c color.RGBA, thick int) {
	if r.Empty() || r.Dx() <= 0 || r.Dy() <= 0 {
		return
	}
	x0, y0 := r.Min.X, r.Min.Y
	x1, y1 := r.Max.X-1, r.Max.Y-1
	if x1 < x0 || y1 < y0 {
		return
	}
	drawPreviewDottedHLine(mat, y0, x0, x1, c, thick)
	drawPreviewDottedHLine(mat, y1, x0, x1, c, thick)
	drawPreviewDottedVLine(mat, x0, y0, y1, c, thick)
	drawPreviewDottedVLine(mat, x1, y0, y1, c, thick)
}

// drawEditorPreviewMonitorOutlines draws a dotted rectangle for each enabled monitor (clip to capture).
func drawEditorPreviewMonitorOutlines(mat *gocv.Mat, vb image.Rectangle) {
	const thick = 1
	n := screen.NumDisplays()
	for i := 0; i < n; i++ {
		if !screen.IsMonitorEnabled(i) {
			continue
		}
		b := screen.DisplayBoundsAbs(i)
		inter := b.Intersect(vb)
		if inter.Empty() {
			continue
		}
		rel := image.Rect(inter.Min.X-vb.Min.X, inter.Min.Y-vb.Min.Y, inter.Max.X-vb.Min.X, inter.Max.Y-vb.Min.Y)
		drawPreviewDottedRectOutline(mat, rel, editorPreviewMonitorOutline, thick)
	}
}

// ConstructEditorTabs builds all editor tab widgets. Call before SetEditorUi.
func ConstructEditorTabs(eu *EditorUi, win fyne.Window) {
	eu.win = win
	var (
		acc   = "Accordion"
		plist = "list"

		et    = eu.EditorTabs
		protw = et.ProgramsTab.Widgets
		itw   = et.ItemsTab.Widgets
		ptw   = et.PointsTab.Widgets
		satw  = et.SearchAreasTab.Widgets
	)

	protw[plist] = new(widget.List)
	protw["searchbar"] = new(widget.Entry)
	populateProgramsFormWidgets(protw)
	et.ProgramsTab.UpdateButton = newEditorUpdateButton()
	et.ProgramsTab.TabItem = NewEditorTab(
		"Programs",
		container.NewBorder(protw["searchbar"], nil, nil, nil, protw[plist]),
		container.NewBorder(nil, nil, nil, nil, buildProgramsRightPanel(protw)),
	)

	//===========================================================================================================ITEMS
	itw[acc] = custom_widgets.NewAccordionWithHeaderWidgets()
	itw["searchbar"] = widget.NewEntry()
	itw["searchbar"].(*widget.Entry).PlaceHolder = "Search here"
	populateItemsFormWidgets(itw, win)
	et.ItemsTab.UpdateButton = newEditorUpdateButton()
	et.ItemsTab.ProgramSelector = widget.NewSelect(nil, nil)
	et.ItemsTab.TabItem = NewEditorTab(
		"Items",
		container.NewBorder(itw["searchbar"], nil, nil, nil, itw[acc]),
		buildItemsRightPanel(et.ItemsTab.ProgramSelector, itw),
	)

	//===========================================================================================================POINTS
	ptw[acc] = widget.NewAccordion()
	ptw["searchbar"] = widget.NewEntry()
	ptw["searchbar"].(*widget.Entry).PlaceHolder = "Search here"
	populatePointsFormWidgets(ptw)
	et.PointsTab.UpdateButton = newEditorUpdateButton()
	pointPreviewPanel := newEditorPreviewPanel()
	et.PointsTab.previewPanel = pointPreviewPanel
	et.PointsTab.PreviewRefreshButton = newEditorPreviewRefreshButton()
	et.PointsTab.ProgramSelector = widget.NewSelect(nil, nil)
	et.PointsTab.TabItem = NewEditorTab(
		"Points",
		container.NewBorder(ptw["searchbar"], nil, nil, nil, ptw[acc]),
		buildPointsRightPanel(et.PointsTab.ProgramSelector, ptw, pointPreviewPanel, et.PointsTab.PreviewRefreshButton),
	)

	//===========================================================================================================SEARCHAREAS
	satw[acc] = widget.NewAccordion()
	satw["searchbar"] = widget.NewEntry()
	satw["searchbar"].(*widget.Entry).PlaceHolder = "Search here"
	populateSearchAreasFormWidgets(satw)
	et.SearchAreasTab.UpdateButton = newEditorUpdateButton()
	searchAreaPreviewPanel := newEditorPreviewPanel()
	et.SearchAreasTab.previewPanel = searchAreaPreviewPanel
	et.SearchAreasTab.PreviewRefreshButton = newEditorPreviewRefreshButton()
	et.SearchAreasTab.ProgramSelector = widget.NewSelect(nil, nil)
	et.SearchAreasTab.TabItem = NewEditorTab(
		"Search Areas",
		container.NewBorder(satw["searchbar"], nil, nil, nil, satw[acc]),
		buildSearchAreasRightPanel(et.SearchAreasTab.ProgramSelector, satw, searchAreaPreviewPanel, et.SearchAreasTab.PreviewRefreshButton),
	)

	//===========================================================================================================MASKS
	mtw := et.MasksTab.Widgets
	mtw["Accordion"] = widget.NewAccordion()
	mtw["searchbar"] = widget.NewEntry()
	mtw["searchbar"].(*widget.Entry).PlaceHolder = "Search here"
	populateMasksFormWidgets(mtw)
	maskPreviewPanel := newEditorPreviewPanel()
	et.MasksTab.previewPanel = maskPreviewPanel
	et.MasksTab.UpdateButton = newEditorUpdateButton()
	et.MasksTab.PreviewRefreshButton = newEditorPreviewRefreshButton()
	et.MasksTab.ProgramSelector = widget.NewSelect(nil, nil)
	et.MasksTab.TabItem = NewEditorTab(
		"Masks",
		container.NewBorder(mtw["searchbar"], nil, nil, nil, mtw["Accordion"]),
		buildMasksRightPanel(et.MasksTab.ProgramSelector, mtw, maskPreviewPanel, et.MasksTab.PreviewRefreshButton),
	)

	//===========================================================================================================AUTOPIC
	atw := et.AutoPicTab.Widgets
	atw["Accordion"] = widget.NewAccordion()
	atw["searchbar"] = widget.NewEntry()
	atw["searchbar"].(*widget.Entry).PlaceHolder = "Search here"
	atw["saveButton"] = widget.NewButton("Save", eu.onAutoPicSave)

	autoPicPreviewPanel := newEditorPreviewPanel()
	et.AutoPicTab.previewPanel = autoPicPreviewPanel

	// Initially disable save button
	atw["saveButton"].(*widget.Button).Disable()

	et.AutoPicTab.PreviewRefreshButton = newEditorPreviewRefreshButton()

	et.AutoPicTab.TabItem = NewEditorTab(
		"AutoPic",
		container.NewBorder(
			atw["searchbar"],
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
			container.NewVBox(
				autoPicPreviewPanel.container,
				container.NewHBox(layout.NewSpacer(), et.AutoPicTab.PreviewRefreshButton),
			),
		),
	)

	et.Append(et.ProgramsTab.TabItem)
	et.Append(et.ItemsTab.TabItem)
	et.Append(et.PointsTab.TabItem)
	et.Append(et.SearchAreasTab.TabItem)
	et.Append(et.MasksTab.TabItem)
	et.Append(et.AutoPicTab.TabItem)
}

func (eu *EditorUi) activeUpdateButton() *widget.Button {
	tab := eu.ActiveTab()
	if tab == nil {
		return nil
	}
	return tab.UpdateButton
}

func (eu *EditorUi) refreshEditorActionBar() {
	if eu.ActionBar == nil {
		return
	}
	objects := []fyne.CanvasObject{layout.NewSpacer()}
	if sel := eu.EditorTabs.Selected(); sel == nil || sel.Text != "AutoPic" {
		objects = append(objects, eu.AddButton)
	}
	if update := eu.activeUpdateButton(); update != nil {
		objects = append(objects, update)
	}
	if sel := eu.EditorTabs.Selected(); sel == nil || sel.Text != "AutoPic" {
		if eu.canDeleteActiveEditorSelection() {
			eu.RemoveButton.Enable()
		} else {
			eu.RemoveButton.Disable()
		}
		objects = append(objects, eu.RemoveButton)
	} else {
		eu.RemoveButton.Disable()
	}
	eu.ActionBar.Objects = objects
	eu.ActionBar.Refresh()
}

func (eu *EditorUi) canDeleteActiveEditorSelection() bool {
	et := eu.EditorTabs
	sel := eu.EditorTabs.Selected()
	if sel == nil {
		return false
	}
	switch sel.Text {
	case "Programs":
		if v, ok := et.ProgramsTab.SelectedItem.(*models.Program); ok {
			return v != nil && v.Name != ""
		}
	case "Items":
		if v, ok := et.ItemsTab.SelectedItem.(*models.Item); ok {
			return v != nil && v.Name != ""
		}
	case "Points":
		if v, ok := et.PointsTab.SelectedItem.(*models.Point); ok {
			return v != nil && v.Name != ""
		}
	case "Search Areas":
		if v, ok := et.SearchAreasTab.SelectedItem.(*models.SearchArea); ok {
			return v != nil && v.Name != ""
		}
	case "Masks":
		if v, ok := et.MasksTab.SelectedItem.(*models.Mask); ok {
			return v != nil && v.Name != ""
		}
	}
	return false
}

func (eu *EditorUi) RefreshEditorActionBar() {
	eu.refreshEditorActionBar()
}

func (eu *EditorUi) constructAddButton() {
	eu.AddButton.Text = "New"
	eu.AddButton.Icon = theme.ContentAddIcon()
	eu.AddButton.Importance = widget.SuccessImportance

}

func (eu *EditorUi) constructRemoveButton() {
	eu.RemoveButton.Text = "Delete"
	eu.RemoveButton.Icon = theme.ContentRemoveIcon()
	eu.RemoveButton.Importance = widget.DangerImportance
	eu.RemoveButton.Disable()
}

// PrepareToolbarButtons sets New/Delete button labels and icons after ConstructEditorTabs.
func PrepareToolbarButtons(eu *EditorUi) {
	eu.constructAddButton()
	eu.constructRemoveButton()
}

// AutoPic tab handlers

func (eu *EditorUi) onAutoPicSave() {
	selectedItem := eu.EditorTabs.AutoPicTab.SelectedItem
	if selectedItem == nil {
		editorErr(errors.New("AutoPic: Cannot save - no search area selected"))
		return
	}

	searchArea, ok := selectedItem.(*models.SearchArea)
	if !ok || searchArea == nil {
		editorErr(errors.New("AutoPic: Cannot save - selected item is not a search area"))
		return
	}

	b := searchAreaBoundsFrom(searchArea)
	b, err := resolveSearchAreaBounds("AutoPic: Cannot save", searchArea, b)
	if err != nil {
		editorErr(err)
		return
	}

	captureImg, err := captureCroppedArea(b.lx, b.ty, b.w, b.h)
	if err != nil {
		editorErr(fmt.Errorf("AutoPic: %w (area: %s)", err, searchArea.Name))
		return
	}

	timestamp := time.Now().Format("20060102_150405")
	filename := fmt.Sprintf("%s_%s.png", timestamp, searchArea.Name)
	autoPicPath := config.GetAutoPicPath()
	if err := os.MkdirAll(autoPicPath, 0755); err != nil {
		editorErr(fmt.Errorf("AutoPic: Error creating AutoPic directory: %w", err))
		return
	}

	fullPath := filepath.Join(autoPicPath, filename)
	if err := robotgo.SavePng(captureImg, fullPath); err != nil {
		editorErr(fmt.Errorf("AutoPic: Error saving image to %s: %w", fullPath, err))
		return
	}
	log.Printf("AutoPic: Image saved to %s", fullPath)
}

func (eu *EditorUi) UpdateAutoPicPreview(searchArea *models.SearchArea) {
	panel := eu.EditorTabs.AutoPicTab.previewPanel
	if searchArea == nil {
		panel.setError(errors.New("AutoPic: Cannot update preview - search area is nil"))
		return
	}

	b := searchAreaBoundsFrom(searchArea)
	b, err := resolveSearchAreaBounds("AutoPic", searchArea, b)
	if err != nil {
		panel.setError(err)
		return
	}

	captureImg, err := captureCroppedArea(b.lx, b.ty, b.w, b.h)
	if err != nil {
		panel.setError(fmt.Errorf("AutoPic: %w (area: %s)", err, searchArea.Name))
		return
	}
	panel.setImage(captureImg)
}

func (eu *EditorUi) clearPreviewImage() {
	if panel := eu.EditorTabs.AutoPicTab.previewPanel; panel != nil {
		panel.clear()
	}
}

func (eu *EditorUi) UpdateSearchAreaPreview(searchArea *models.SearchArea) {
	panel := eu.EditorTabs.SearchAreasTab.previewPanel
	if searchArea == nil {
		panel.setError(errors.New("SearchArea: Cannot update preview - search area is nil"))
		return
	}

	b := searchAreaBoundsFrom(searchArea)
	b, err := resolveSearchAreaBounds("SearchArea", searchArea, b)
	if err != nil {
		panel.setError(err)
		return
	}

	previewImg, err := captureVirtualDesktop(func(mat *gocv.Mat, bounds image.Rectangle) {
		rect := image.Rect(b.lx-bounds.Min.X, b.ty-bounds.Min.Y, b.rx-bounds.Min.X, b.by-bounds.Min.Y)
		gocv.Rectangle(mat, rect, color.RGBA{R: 255, A: 255}, 2)
	})
	if err != nil {
		panel.setError(fmt.Errorf("SearchArea: %w (area: %s)", err, searchArea.Name))
		return
	}
	panel.setImage(previewImg)
}

func (eu *EditorUi) clearSearchAreaPreviewImage() {
	if panel := eu.EditorTabs.SearchAreasTab.previewPanel; panel != nil {
		panel.clear()
	}
}

func (eu *EditorUi) UpdatePointPreview(point *models.Point) {
	panel := eu.EditorTabs.PointsTab.previewPanel
	if point == nil {
		panel.setError(errors.New("Point: Cannot update preview - point is nil"))
		return
	}

	px := coordToIntForPreview(point.X)
	py := coordToIntForPreview(point.Y)
	vb := screen.VirtualBounds()
	if px < vb.Min.X || py < vb.Min.Y || px > vb.Max.X || py > vb.Max.Y {
		panel.setError(fmt.Errorf("Point: Point outside virtual desktop - desktop: (%d,%d)..(%d,%d), point: (%d,%d) (point: %s)",
			vb.Min.X, vb.Min.Y, vb.Max.X, vb.Max.Y, px, py, point.Name))
		return
	}

	previewImg, err := captureVirtualDesktop(func(mat *gocv.Mat, bounds image.Rectangle) {
		center := image.Point{X: px - bounds.Min.X, Y: py - bounds.Min.Y}
		red := color.RGBA{R: 255, A: 255}
		gocv.Circle(mat, center, 8, red, 2)
		gocv.Line(mat, image.Point{X: center.X - 15, Y: center.Y}, image.Point{X: center.X + 15, Y: center.Y}, red, 2)
		gocv.Line(mat, image.Point{X: center.X, Y: center.Y - 15}, image.Point{X: center.X, Y: center.Y + 15}, red, 2)
	})
	if err != nil {
		panel.setError(fmt.Errorf("Point: %w (point: %s)", err, point.Name))
		return
	}

	fyne.DoAndWait(func() { panel.setImage(previewImg) })
}

func (eu *EditorUi) clearPointPreviewImage() {
	if panel := eu.EditorTabs.PointsTab.previewPanel; panel != nil {
		panel.clear()
	}
}

// UpdateMaskPreview loads and displays the mask image for the given program and mask name.
func (eu *EditorUi) UpdateMaskPreview(programName, maskName string) {
	masksPath := config.GetMasksPath()
	imgPath := filepath.Join(masksPath, programName, maskName+config.PNG)

	if _, err := os.Stat(imgPath); err != nil {
		eu.ClearMaskPreviewImage()
		return
	}

	mat := gocv.IMRead(imgPath, gocv.IMReadColor)
	if mat.Empty() {
		eu.ClearMaskPreviewImage()
		return
	}
	defer mat.Close()

	img, err := mat.ToImage()
	if err != nil {
		eu.ClearMaskPreviewImage()
		return
	}

	if panel := eu.EditorTabs.MasksTab.previewPanel; panel != nil {
		panel.setImage(img)
	}
}

func (eu *EditorUi) ClearMaskPreviewImage() {
	if panel := eu.EditorTabs.MasksTab.previewPanel; panel != nil {
		panel.clear()
	}
}

// SetMaskImageMode switches the right-side UI between variable entry and uploaded image display.
// When hasImage is true, the value/shape entries are hidden and the image status + remove button are shown.
func (eu *EditorUi) SetMaskImageMode(hasImage bool) {
	setMaskImageModeOnWidgets(eu.EditorTabs.MasksTab.Widgets, hasImage)
}

// HasMaskImage checks if an image file exists for the given program and mask.
func HasMaskImage(programName, maskName string) bool {
	masksPath := config.GetMasksPath()
	imgPath := filepath.Join(masksPath, programName, maskName+config.PNG)
	_, err := os.Stat(imgPath)
	return err == nil
}

func (eu *EditorUi) RefreshAutoPicSearchAreas() {
	// Reset the selected item and disable save button
	eu.EditorTabs.AutoPicTab.SelectedItem = nil
	if saveButton, ok := eu.EditorTabs.AutoPicTab.Widgets["saveButton"].(*widget.Button); ok {
		saveButton.Disable()
	}
	// Clear preview image
	if panel := eu.EditorTabs.AutoPicTab.previewPanel; panel != nil {
		panel.clear()
	}
}
