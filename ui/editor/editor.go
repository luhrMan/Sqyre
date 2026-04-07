package editor

import (
	"Sqyre/internal/config"
	"Sqyre/internal/desktop"
	"Sqyre/internal/models"
	"Sqyre/internal/screen"
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
	"fyne.io/fyne/v2/layout"
	"fyne.io/fyne/v2/theme"
	"fyne.io/fyne/v2/widget"

	"Sqyre/ui/completionentry"

	kxlayout "github.com/ErikKalkoken/fyne-kx/layout"
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
	previewImage         *canvas.Image
	UpdateButton         *widget.Button
	PreviewRefreshButton *widget.Button
	OriginalValues       map[string]string
}

func NewEditorTab(name string, left, right *fyne.Container) *container.TabItem {
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

func editorMonitorOutlines() []desktop.MonitorOutline {
	n := screen.NumDisplays()
	out := make([]desktop.MonitorOutline, 0, n)
	for i := 0; i < n; i++ {
		out = append(out, desktop.MonitorOutline{
			AbsBounds: screen.DisplayBoundsAbs(i),
			Enabled:   screen.IsMonitorEnabled(i),
		})
	}
	return out
}

// ConstructEditorTabs builds all editor tab widgets. Call before SetEditorUi.
func ConstructEditorTabs(eu *EditorUi, win fyne.Window) {
	eu.win = win
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

		et    = eu.EditorTabs
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
	et.ProgramsTab.UpdateButton.Icon = theme.ViewRefreshIcon()
	et.ProgramsTab.UpdateButton.Importance = widget.HighImportance
	et.ProgramsTab.UpdateButton.Disable()

	et.ProgramsTab.TabItem = NewEditorTab(
		"Programs",
		container.NewBorder(protw["searchbar"], nil, nil, nil, protw[plist]),
		container.NewBorder(nil, nil, nil, nil, protw[form]),
	)

	//===========================================================================================================ITEMS
	itw[acc] = custom_widgets.NewAccordionWithHeaderWidgets()
	itw["searchbar"] = widget.NewEntry()
	itw["searchbar"].(*widget.Entry).PlaceHolder = "Search here"
	itw[name] = new(widget.Entry)
	itw[cols] = new(widget.Entry)
	itw[rows] = new(widget.Entry)
	itw["tagEntry"] = completionentry.NewCompletionEntry([]string{})
	itw["tagEntry"].(*completionentry.CompletionEntry).PlaceHolder = "Enter tag name and press Enter"
	itw["tagSubmitButton"] = widget.NewButtonWithIcon("", theme.ContentAddIcon(), nil)
	itw["tagSubmitButton"].(*widget.Button).Importance = widget.MediumImportance
	itw["tagEntryContainer"] = container.NewBorder(nil, nil, nil, itw["tagSubmitButton"], itw["tagEntry"])
	itw[tags] = container.New(kxlayout.NewRowWrapLayout())
	itw[sm] = new(widget.Entry)
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
		win,
		nil, // onVariantChange set in setItemsWidgets (editor_wiring.go)
	)

	itw[form] = widget.NewForm(
		widget.NewFormItem(name, itw[name]),
		widget.NewFormItem(cols, itw[cols]),
		widget.NewFormItem(rows, itw[rows]),
		widget.NewFormItem(tags, itw["tagEntryContainer"]),
		widget.NewFormItem("", itw[tags]),
		widget.NewFormItem(sm, itw[sm]),
		widget.NewFormItem("Mask", itw["maskContainer"]),
	)
	et.ItemsTab.UpdateButton = widget.NewButton("Update", nil)
	et.ItemsTab.UpdateButton.Icon = theme.ViewRefreshIcon()
	et.ItemsTab.UpdateButton.Importance = widget.HighImportance
	et.ItemsTab.UpdateButton.Disable()

	iveBorder := canvas.NewRectangle(color.NRGBA{})
	iveBorder.StrokeColor = theme.ButtonColor()
	iveBorder.StrokeWidth = 2
	iveBorder.CornerRadius = 4

	et.ItemsTab.ProgramSelector = widget.NewSelect(nil, nil)
	et.ItemsTab.TabItem = NewEditorTab(
		"Items",
		container.NewBorder(itw["searchbar"], nil, nil, nil, itw[acc]),
		container.NewBorder(
			container.NewVBox(LabeledProgramSelector(et.ItemsTab.ProgramSelector), itw[form]),
			nil, nil, nil,
			container.NewStack(iveBorder, container.NewPadded(itw[ive])),
		),
	)

	//===========================================================================================================POINTS
	ptw[acc] = widget.NewAccordion()
	ptw["searchbar"] = widget.NewEntry()
	ptw["searchbar"].(*widget.Entry).PlaceHolder = "Search here"
	ptw[name] = new(widget.Entry)
	ptw[x] = custom_widgets.NewVarEntry(macroVarNames)
	ptw[y] = custom_widgets.NewVarEntry(macroVarNames)

	// Create record button for capturing point coordinates
	ptw["recordButton"] = widget.NewButtonWithIcon("", theme.MediaRecordIcon(), nil)
	ptw["recordButton"].(*widget.Button).Importance = widget.DangerImportance

	et.PointsTab.UpdateButton = widget.NewButton("Update", nil)
	et.PointsTab.UpdateButton.Icon = theme.ViewRefreshIcon()
	et.PointsTab.UpdateButton.Importance = widget.HighImportance
	et.PointsTab.UpdateButton.Disable()

	ptw[form] = widget.NewForm(
		widget.NewFormItem(name, ptw[name]),
		widget.NewFormItem(x, ptw[x]),
		widget.NewFormItem(y, ptw[y]),
		widget.NewFormItem("", container.NewHBox(layout.NewSpacer(), ptw["recordButton"])),
	)

	// Create preview image for Points tab
	pointPreviewImage := canvas.NewImageFromImage(nil)
	pointPreviewImage.FillMode = canvas.ImageFillContain
	pointPreviewImage.SetMinSize(fyne.NewSize(400, 300))

	// Store the image reference in the tab for later access
	et.PointsTab.previewImage = pointPreviewImage

	et.PointsTab.PreviewRefreshButton = widget.NewButtonWithIcon("Refresh preview", theme.ViewRefreshIcon(), nil)
	et.PointsTab.PreviewRefreshButton.Importance = widget.LowImportance

	et.PointsTab.ProgramSelector = widget.NewSelect(nil, nil)
	et.PointsTab.TabItem = NewEditorTab(
		"Points",
		container.NewBorder(ptw["searchbar"], nil, nil, nil, ptw[acc]),
		container.NewBorder(
			container.NewVBox(LabeledProgramSelector(et.PointsTab.ProgramSelector), ptw[form]),
			nil,
			nil,
			nil,
			container.NewVBox(
				wrapEditorPreviewImage(pointPreviewImage),
				container.NewHBox(layout.NewSpacer(), et.PointsTab.PreviewRefreshButton),
			),
		),
	)

	//===========================================================================================================SEARCHAREAS
	satw[acc] = widget.NewAccordion()
	satw["searchbar"] = widget.NewEntry()
	satw["searchbar"].(*widget.Entry).PlaceHolder = "Search here"
	satw[name] = new(widget.Entry)
	satw[x1] = custom_widgets.NewVarEntry(macroVarNames)
	satw[y1] = custom_widgets.NewVarEntry(macroVarNames)
	satw[x2] = custom_widgets.NewVarEntry(macroVarNames)
	satw[y2] = custom_widgets.NewVarEntry(macroVarNames)
	// Create record button for capturing search area rectangle (click and drag)
	satw["recordButton"] = widget.NewButtonWithIcon("", theme.MediaRecordIcon(), nil)
	satw["recordButton"].(*widget.Button).Importance = widget.DangerImportance
	et.SearchAreasTab.UpdateButton = widget.NewButton("Update", nil)
	et.SearchAreasTab.UpdateButton.Icon = theme.ViewRefreshIcon()
	et.SearchAreasTab.UpdateButton.Importance = widget.HighImportance
	et.SearchAreasTab.UpdateButton.Disable()

	satw[form] = widget.NewForm(
		widget.NewFormItem(name, satw[name]),
		widget.NewFormItem(x1, satw[x1]),
		widget.NewFormItem(y1, satw[y1]),
		widget.NewFormItem(x2, satw[x2]),
		widget.NewFormItem(y2, satw[y2]),
		widget.NewFormItem("", container.NewHBox(layout.NewSpacer(), satw["recordButton"])),
	)

	// Create preview image for Search Areas tab
	searchAreaPreviewImage := canvas.NewImageFromImage(nil)
	searchAreaPreviewImage.FillMode = canvas.ImageFillContain
	searchAreaPreviewImage.SetMinSize(fyne.NewSize(400, 300))

	// Store the image reference in the tab for later access
	et.SearchAreasTab.previewImage = searchAreaPreviewImage

	et.SearchAreasTab.PreviewRefreshButton = widget.NewButtonWithIcon("Refresh preview", theme.ViewRefreshIcon(), nil)
	et.SearchAreasTab.PreviewRefreshButton.Importance = widget.LowImportance

	et.SearchAreasTab.ProgramSelector = widget.NewSelect(nil, nil)
	et.SearchAreasTab.TabItem = NewEditorTab(
		"Search Areas",
		container.NewBorder(satw["searchbar"], nil, nil, nil, satw[acc]),
		container.NewBorder(
			container.NewVBox(LabeledProgramSelector(et.SearchAreasTab.ProgramSelector), satw[form]),
			nil,
			nil,
			nil,
			container.NewVBox(
				wrapEditorPreviewImage(searchAreaPreviewImage),
				container.NewHBox(layout.NewSpacer(), et.SearchAreasTab.PreviewRefreshButton),
			),
		),
	)

	//===========================================================================================================MASKS
	mtw := et.MasksTab.Widgets
	mtw["Accordion"] = widget.NewAccordion()
	mtw["searchbar"] = widget.NewEntry()
	mtw["searchbar"].(*widget.Entry).PlaceHolder = "Search here"
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
	mtw["CenterX"] = custom_widgets.NewVarEntry(macroVarNames)
	mtw["CenterX"].(*custom_widgets.VarEntry).PlaceHolder = "50"
	mtw["CenterY"] = custom_widgets.NewVarEntry(macroVarNames)
	mtw["CenterY"].(*custom_widgets.VarEntry).PlaceHolder = "50"
	mtw["centerContainer"] = container.NewGridWithColumns(2,
		container.NewBorder(nil, nil, widget.NewLabel("X %"), nil, mtw["CenterX"]),
		container.NewBorder(nil, nil, widget.NewLabel("Y %"), nil, mtw["CenterY"]),
	)

	// Rectangle entries:  base * height
	mtw["Base"] = custom_widgets.NewVarEntry(macroVarNames)
	mtw["Base"].(*custom_widgets.VarEntry).PlaceHolder = "base"
	mtw["Height"] = custom_widgets.NewVarEntry(macroVarNames)
	mtw["Height"].(*custom_widgets.VarEntry).PlaceHolder = "height"
	mtw["rectContainer"] =
		container.NewGridWithColumns(3,
			mtw["Base"],
			container.NewCenter(widget.NewLabel("*")),
			mtw["Height"],
		)

	// Circle entries:  π * radius²
	mtw["Radius"] = custom_widgets.NewVarEntry(macroVarNames)
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
	et.MasksTab.UpdateButton.Icon = theme.ViewRefreshIcon()
	et.MasksTab.UpdateButton.Importance = widget.HighImportance
	et.MasksTab.UpdateButton.Disable()

	mtw["Form"] = widget.NewForm(
		widget.NewFormItem("Name", mtw["Name"]),
		widget.NewFormItem("Shape", mtw["shapeSelect"]),
		widget.NewFormItem("Center", mtw["centerContainer"]),
		widget.NewFormItem("", mtw["shapeParamsContainer"]),
		widget.NewFormItem("", mtw["Inverse"]),
	)

	et.MasksTab.PreviewRefreshButton = widget.NewButtonWithIcon("Refresh preview", theme.ViewRefreshIcon(), nil)
	et.MasksTab.PreviewRefreshButton.Importance = widget.LowImportance

	et.MasksTab.ProgramSelector = widget.NewSelect(nil, nil)
	et.MasksTab.TabItem = NewEditorTab(
		"Masks",
		container.NewBorder(mtw["searchbar"], nil, nil, nil, mtw["Accordion"]),
		container.NewBorder(
			container.NewVBox(
				LabeledProgramSelector(et.MasksTab.ProgramSelector),
				mtw["Form"],
				container.NewHBox(mtw["uploadButton"], mtw["removeImageButton"]),
				mtw["imageStatus"],
			),
			nil, nil, nil,
			container.NewVBox(
				wrapEditorPreviewImage(maskPreviewImage),
				container.NewHBox(layout.NewSpacer(), et.MasksTab.PreviewRefreshButton),
			),
		),
	)

	//===========================================================================================================AUTOPIC
	atw := et.AutoPicTab.Widgets
	atw["Accordion"] = widget.NewAccordion()
	atw["searchbar"] = widget.NewEntry()
	atw["searchbar"].(*widget.Entry).PlaceHolder = "Search here"
	atw["saveButton"] = widget.NewButton("Save", eu.onAutoPicSave)

	// Create preview image and container
	previewImage := canvas.NewImageFromImage(nil)
	previewImage.FillMode = canvas.ImageFillContain
	previewImage.SetMinSize(fyne.NewSize(400, 300))

	// Store the image reference in the tab for later access
	et.AutoPicTab.previewImage = previewImage

	// Initially disable save button
	atw["saveButton"].(*widget.Button).Disable()

	et.AutoPicTab.PreviewRefreshButton = widget.NewButtonWithIcon("Refresh preview", theme.ViewRefreshIcon(), nil)
	et.AutoPicTab.PreviewRefreshButton.Importance = widget.LowImportance

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
				wrapEditorPreviewImage(previewImage),
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
		activeWire.ShowErrorWithEscape(errors.New("AutoPic: Cannot save - no search area selected"), eu.win)
		return
	}

	searchArea, ok := selectedItem.(*models.SearchArea)
	if !ok {
		activeWire.ShowErrorWithEscape(errors.New("AutoPic: Cannot save - selected item is not a search area"), eu.win)
		return
	}

	// Validate search area
	if searchArea == nil {
		activeWire.ShowErrorWithEscape(errors.New("AutoPic: Cannot save - search area is nil"), eu.win)
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
		activeWire.ShowErrorWithEscape(fmt.Errorf("AutoPic: Cannot save - invalid search area dimensions - width: %d, height: %d (area: %s)", w, h, searchArea.Name), eu.win)
		return
	}

	vb := screen.VirtualBounds()
	if lx < vb.Min.X || ty < vb.Min.Y || rx > vb.Max.X || by > vb.Max.Y {
		activeWire.ShowErrorWithEscape(fmt.Errorf("AutoPic: Cannot save - search area outside virtual desktop - desktop: (%d,%d)..(%d,%d), area: (%d,%d) to (%d,%d) (area: %s)",
			vb.Min.X, vb.Min.Y, vb.Max.X, vb.Max.Y, lx, ty, rx, by, searchArea.Name), eu.win)
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

		captureImg, err = desktop.Default.CaptureImg(lx, ty, w, h)
		if err != nil {
			activeWire.ShowErrorWithEscape(fmt.Errorf("AutoPic: Error capturing image - %v (area: %s)", err, searchArea.Name), eu.win)
			captureImg = nil
		}
	}()

	// Validate the captured image
	if captureImg == nil {
		activeWire.ShowErrorWithEscape(fmt.Errorf("AutoPic: Cannot save - screen capture returned nil image (area: %s)", searchArea.Name), eu.win)
		return
	}

	// Create filename with timestamp
	timestamp := time.Now().Format("20060102_150405")
	filename := fmt.Sprintf("%s_%s.png", timestamp, searchArea.Name)

	// Ensure AutoPic directory exists
	autoPicPath := config.GetAutoPicPath()
	if err := os.MkdirAll(autoPicPath, 0755); err != nil {
		activeWire.ShowErrorWithEscape(fmt.Errorf("AutoPic: Error creating AutoPic directory: %v", err), eu.win)
		return
	}

	// Validate the path
	fullPath := filepath.Join(autoPicPath, filename)
	if fullPath == "" {
		activeWire.ShowErrorWithEscape(errors.New("AutoPic: Error creating file path"), eu.win)
		return
	}

	// Save the image with error handling
	func() {
		defer func() {
			if r := recover(); r != nil {
				services.LogPanicToFile(r, "AutoPic: Image save (path: "+fullPath+")")
			}
		}()

		if err := desktop.Default.SavePng(captureImg, fullPath); err != nil {
			activeWire.ShowErrorWithEscape(fmt.Errorf("AutoPic: Error saving image to %s: %v", fullPath, err), eu.win)
			return
		}

		activeWire.ShowErrorWithEscape(fmt.Errorf("AutoPic: Image saved successfully to: %s", fullPath), eu.win)
	}()
}

func (eu *EditorUi) UpdateAutoPicPreview(searchArea *models.SearchArea) {
	// Validate search area
	if searchArea == nil {
		activeWire.ShowErrorWithEscape(errors.New("AutoPic: Cannot update preview - search area is nil"), eu.win)
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
		activeWire.ShowErrorWithEscape(fmt.Errorf("AutoPic: Invalid search area dimensions - width: %d, height: %d (area: %s)", w, h, searchArea.Name), eu.win)
		eu.clearPreviewImage()
		return
	}

	vb := screen.VirtualBounds()
	if lx < vb.Min.X || ty < vb.Min.Y || rx > vb.Max.X || by > vb.Max.Y {
		activeWire.ShowErrorWithEscape(fmt.Errorf("AutoPic: Search area outside virtual desktop - desktop: (%d,%d)..(%d,%d), area: (%d,%d) to (%d,%d) (area: %s)",
			vb.Min.X, vb.Min.Y, vb.Max.X, vb.Max.Y, lx, ty, rx, by, searchArea.Name), eu.win)
		eu.clearPreviewImage()
		return
	}

	// Attempt to capture the screen area with error recovery
	defer func() {
		if r := recover(); r != nil {
			services.LogPanicToFile(r, "AutoPic: Screen capture (area: "+searchArea.Name+")")
			eu.clearPreviewImage()
		}
	}()

	captureImg, err := desktop.Default.CaptureImg(lx, ty, w, h)
	if err != nil {
		activeWire.ShowErrorWithEscape(fmt.Errorf("AutoPic: Error capturing image - %v (area: %s)", err, searchArea.Name), eu.win)
		captureImg = nil
	}

	// Validate the captured image
	if captureImg == nil {
		activeWire.ShowErrorWithEscape(fmt.Errorf("AutoPic: Screen capture returned nil image (area: %s)", searchArea.Name), eu.win)
		eu.clearPreviewImage()
		return
	}

	// Update preview image
	if previewImage := eu.EditorTabs.AutoPicTab.previewImage; previewImage != nil {
		previewImage.Image = captureImg
		previewImage.Refresh()
	} else {
		activeWire.ShowErrorWithEscape(errors.New("AutoPic: Preview image widget is nil"), eu.win)
	}
}

func (eu *EditorUi) clearPreviewImage() {
	if previewImage := eu.EditorTabs.AutoPicTab.previewImage; previewImage != nil {
		previewImage.Image = nil
		previewImage.Refresh()
	}
}

func (eu *EditorUi) ErrorPopUp(s string) {
	label := widget.NewLabel(s)
	label.Importance = widget.DangerImportance

	pu := widget.NewPopUp(
		container.NewBorder(
			nil, nil,
			widget.NewIcon(theme.CancelIcon()),
			nil,
			label,
		),
		eu.win.Canvas(),
	)
	pu.Show()
}

func (eu *EditorUi) UpdateSearchAreaPreview(searchArea *models.SearchArea) {
	eu.EditorTabs.SearchAreasTab.previewImage.Resource = nil
	// Validate search area
	if searchArea == nil {
		activeWire.ShowErrorWithEscape(errors.New("SearchArea: Cannot update preview - search area is nil"), eu.win)
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
		eu.clearSearchAreaPreviewImage()
		eu.EditorTabs.SearchAreasTab.previewImage.Resource = theme.BrokenImageIcon()
		eu.ErrorPopUp(fmt.Sprintf("SearchArea: Invalid search area dimensions - width: %d, height: %d (area: %s)", w, h, searchArea.Name))
		// label := widget.NewLabel(fmt.Sprintf("SearchArea: Invalid search area dimensions - width: %d, height: %d (area: %s)", w, h, searchArea.Name))
		// label.Importance = widget.DangerImportance

		// pu := widget.NewPopUp(
		// 	container.NewBorder(
		// 		nil, nil,
		// 		widget.NewIcon(theme.CancelIcon()),
		// 		nil,
		// 		label,
		// 	),
		// 	eu.win.Canvas(),
		// )
		// pu.Show()
		// activeWire.ShowErrorWithEscape(fmt.Errorf("SearchArea: Invalid search area dimensions - width: %d, height: %d (area: %s)", w, h, searchArea.Name), eu.win)
		return
	}

	vb := screen.VirtualBounds()
	if lx < vb.Min.X || ty < vb.Min.Y || rx > vb.Max.X || by > vb.Max.Y {
		activeWire.ShowErrorWithEscape(fmt.Errorf("SearchArea: Search area outside virtual desktop - desktop: (%d,%d)..(%d,%d), area: (%d,%d) to (%d,%d) (area: %s)",
			vb.Min.X, vb.Min.Y, vb.Max.X, vb.Max.Y, lx, ty, rx, by, searchArea.Name), eu.win)
		eu.clearSearchAreaPreviewImage()
		return
	}

	// Attempt to capture the full virtual desktop (all enabled monitors)
	defer func() {
		if r := recover(); r != nil {
			services.LogPanicToFile(r, "SearchArea: Screen capture (area: "+searchArea.Name+")")
			eu.clearSearchAreaPreviewImage()
		}
	}()

	previewImg, err := desktop.SearchAreaPreviewImage(vb, lx, ty, rx, by, editorMonitorOutlines())
	if err != nil {
		activeWire.ShowErrorWithEscape(fmt.Errorf("SearchArea: Error building preview - %v (area: %s)", err, searchArea.Name), eu.win)
		eu.clearSearchAreaPreviewImage()
		return
	}
	if previewImg == nil {
		activeWire.ShowErrorWithEscape(fmt.Errorf("SearchArea: Screen capture returned nil image (area: %s)", searchArea.Name), eu.win)
		eu.clearSearchAreaPreviewImage()
		return
	}

	// Update preview image
	if previewImage := eu.EditorTabs.SearchAreasTab.previewImage; previewImage != nil {
		previewImage.Image = previewImg
		previewImage.Refresh()
	} else {
		activeWire.ShowErrorWithEscape(errors.New("SearchArea: Preview image widget is nil"), eu.win)
	}
}

func (eu *EditorUi) clearSearchAreaPreviewImage() {
	if previewImage := eu.EditorTabs.SearchAreasTab.previewImage; previewImage != nil {
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

func (eu *EditorUi) UpdatePointPreview(point *models.Point) {
	// Validate point
	if point == nil {
		activeWire.ShowErrorWithEscape(errors.New("Point: Cannot update preview - point is nil"), eu.win)
		return
	}

	px := pointCoordToIntForPreview(point.X)
	py := pointCoordToIntForPreview(point.Y)

	vb := screen.VirtualBounds()
	if px < vb.Min.X || py < vb.Min.Y || px > vb.Max.X || py > vb.Max.Y {
		activeWire.ShowErrorWithEscape(fmt.Errorf("Point: Point outside virtual desktop - desktop: (%d,%d)..(%d,%d), point: (%d,%d) (point: %s)",
			vb.Min.X, vb.Min.Y, vb.Max.X, vb.Max.Y, px, py, point.Name), eu.win)
		eu.clearPointPreviewImage()
		return
	}

	// Attempt to capture the full virtual desktop (all enabled monitors)
	defer func() {
		if r := recover(); r != nil {
			services.LogPanicToFile(r, "Point: Screen capture (point: "+point.Name+")")
			eu.clearPointPreviewImage()
		}
	}()

	previewImg, err := desktop.PointPreviewImage(vb, px, py, editorMonitorOutlines())
	if err != nil {
		activeWire.ShowErrorWithEscape(fmt.Errorf("Point: Error building preview - %v (point: %s)", err, point.Name), eu.win)
		eu.clearPointPreviewImage()
		return
	}
	if previewImg == nil {
		activeWire.ShowErrorWithEscape(fmt.Errorf("Point: Screen capture returned nil image (point: %s)", point.Name), eu.win)
		eu.clearPointPreviewImage()
		return
	}

	// Update preview image
	if previewImage := eu.EditorTabs.PointsTab.previewImage; previewImage != nil {
		previewImage.Image = previewImg
		fyne.DoAndWait(func() {
			previewImage.Refresh()
		})
	} else {
		activeWire.ShowErrorWithEscape(errors.New("Point: Preview image widget is nil"), eu.win)
	}
}

func (eu *EditorUi) clearPointPreviewImage() {
	if previewImage := eu.EditorTabs.PointsTab.previewImage; previewImage != nil {
		previewImage.Image = nil
		previewImage.Refresh()
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

	img, err := desktop.MaskImageFromFile(imgPath)
	if err != nil || img == nil {
		eu.ClearMaskPreviewImage()
		return
	}

	if previewImage := eu.EditorTabs.MasksTab.previewImage; previewImage != nil {
		previewImage.Image = img
		previewImage.Refresh()
	}
}

func (eu *EditorUi) ClearMaskPreviewImage() {
	if previewImage := eu.EditorTabs.MasksTab.previewImage; previewImage != nil {
		previewImage.Image = nil
		previewImage.Refresh()
	}
}

// SetMaskImageMode switches the right-side UI between variable entry and uploaded image display.
// When hasImage is true, the value/shape entries are hidden and the image status + remove button are shown.
func (eu *EditorUi) SetMaskImageMode(hasImage bool) {
	mtw := eu.EditorTabs.MasksTab.Widgets
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

func (eu *EditorUi) RefreshAutoPicSearchAreas() {
	// Reset the selected item and disable save button
	eu.EditorTabs.AutoPicTab.SelectedItem = nil
	if saveButton, ok := eu.EditorTabs.AutoPicTab.Widgets["saveButton"].(*widget.Button); ok {
		saveButton.Disable()
	}
	// Clear preview image
	if previewImage := eu.EditorTabs.AutoPicTab.previewImage; previewImage != nil {
		previewImage.Image = nil
		previewImage.Refresh()
	}
}
