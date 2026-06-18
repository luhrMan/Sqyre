package editor

import (
	"Sqyre/internal/services"
	"Sqyre/ui/completionentry"
	"Sqyre/ui/custom_widgets"
	"image/color"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/canvas"
	"fyne.io/fyne/v2/container"
	"fyne.io/fyne/v2/layout"
	"fyne.io/fyne/v2/theme"
	"fyne.io/fyne/v2/widget"

	kxlayout "github.com/ErikKalkoken/fyne-kx/layout"
)

func newEditorUpdateButton() *widget.Button {
	btn := widget.NewButton("Update", nil)
	btn.Icon = theme.ViewRefreshIcon()
	btn.Importance = widget.HighImportance
	btn.Disable()
	return btn
}

func newEditorPreviewImage() *canvas.Image {
	return newEditorPreviewPanel().image
}

func newEditorPreviewRefreshButton() *widget.Button {
	btn := widget.NewButtonWithIcon("Refresh preview", theme.ViewRefreshIcon(), nil)
	btn.Importance = widget.LowImportance
	return btn
}

func buildPreviewColumn(panel *editorPreviewPanel, refreshBtn *widget.Button) fyne.CanvasObject {
	return container.NewVBox(
		panel.container,
		container.NewHBox(layout.NewSpacer(), refreshBtn),
	)
}

// populateProgramsFormWidgets creates the Programs tab right-side form widgets.
func populateProgramsFormWidgets(w map[string]fyne.CanvasObject) {
	w["Name"] = new(widget.Entry)
	w["Form"] = widget.NewForm(
		widget.NewFormItem("Name", w["Name"]),
	)
}

func buildProgramsRightPanel(w map[string]fyne.CanvasObject) fyne.CanvasObject {
	return w["Form"].(fyne.CanvasObject)
}

// populateItemsFormWidgets creates Items tab right-side widgets (form fields, tags, mask, icon editor).
func populateItemsFormWidgets(w map[string]fyne.CanvasObject, win fyne.Window) {
	w["Name"] = new(widget.Entry)
	w["Cols"] = new(widget.Entry)
	w["Rows"] = new(widget.Entry)
	w["tagEntry"] = completionentry.NewCompletionEntry([]string{})
	w["tagEntry"].(*completionentry.CompletionEntry).PlaceHolder = "Enter tag name and press Enter"
	w["tagSubmitButton"] = widget.NewButtonWithIcon("", theme.ContentAddIcon(), nil)
	w["tagSubmitButton"].(*widget.Button).Importance = widget.MediumImportance
	w["tagEntryContainer"] = container.NewBorder(nil, nil, nil, w["tagSubmitButton"], w["tagEntry"])
	w["Tags"] = container.New(kxlayout.NewRowWrapLayout())
	w["StackMax"] = new(widget.Entry)
	w["maskLabel"] = widget.NewLabel("None")
	w["maskSelectButton"] = widget.NewButtonWithIcon("Select", theme.SearchIcon(), nil)
	w["maskSelectButton"].(*widget.Button).Importance = widget.MediumImportance
	w["maskClearButton"] = widget.NewButtonWithIcon("", theme.ContentClearIcon(), nil)
	w["maskClearButton"].(*widget.Button).Importance = widget.LowImportance
	w["maskDetailsLabel"] = widget.NewLabel("")
	w["maskDetailsLabel"].(*widget.Label).TextStyle = fyne.TextStyle{Italic: true}
	w["maskContainer"] = container.NewVBox(
		container.NewBorder(nil, nil, nil,
			container.NewHBox(w["maskSelectButton"], w["maskClearButton"]),
			w["maskLabel"],
		),
		w["maskDetailsLabel"],
	)
	iconService := services.IconVariantServiceInstance()
	w["iconVariantEditor"] = custom_widgets.NewIconVariantEditor("", "", iconService, win, nil)
	w["Form"] = widget.NewForm(
		widget.NewFormItem("Name", w["Name"]),
		widget.NewFormItem("Cols", w["Cols"]),
		widget.NewFormItem("Rows", w["Rows"]),
		widget.NewFormItem("Tags", w["tagEntryContainer"]),
		widget.NewFormItem("", w["Tags"]),
		widget.NewFormItem("StackMax", w["StackMax"]),
		widget.NewFormItem("Mask", w["maskContainer"]),
	)
}

func buildItemsRightPanel(programSelector *widget.Select, w map[string]fyne.CanvasObject) fyne.CanvasObject {
	iveBorder := canvas.NewRectangle(color.NRGBA{})
	iveBorder.StrokeColor = theme.ButtonColor()
	iveBorder.StrokeWidth = 2
	iveBorder.CornerRadius = 4
	return container.NewBorder(
		container.NewVBox(LabeledProgramSelector(programSelector), w["Form"]),
		nil, nil, nil,
		container.NewStack(iveBorder, container.NewPadded(w["iconVariantEditor"])),
	)
}

// populatePointsFormWidgets creates Points tab right-side form widgets.
func populatePointsFormWidgets(w map[string]fyne.CanvasObject) {
	w["Name"] = new(widget.Entry)
	w["X"] = custom_widgets.NewVarEntry(macroVarNames)
	w["Y"] = custom_widgets.NewVarEntry(macroVarNames)
	w["recordButton"] = widget.NewButtonWithIcon("", theme.MediaRecordIcon(), nil)
	w["recordButton"].(*widget.Button).Importance = widget.DangerImportance
	w["Form"] = widget.NewForm(
		widget.NewFormItem("Name", w["Name"]),
		widget.NewFormItem("X", w["X"]),
		widget.NewFormItem("Y", w["Y"]),
		widget.NewFormItem("", container.NewHBox(layout.NewSpacer(), w["recordButton"])),
	)
}

func buildPointsRightPanel(programSelector *widget.Select, w map[string]fyne.CanvasObject, previewPanel *editorPreviewPanel, refreshBtn *widget.Button) fyne.CanvasObject {
	return container.NewBorder(
		container.NewVBox(LabeledProgramSelector(programSelector), w["Form"]),
		nil, nil, nil,
		buildPreviewColumn(previewPanel, refreshBtn),
	)
}

// populateSearchAreasFormWidgets creates Search Areas tab right-side form widgets.
func populateSearchAreasFormWidgets(w map[string]fyne.CanvasObject) {
	w["Name"] = new(widget.Entry)
	w["LeftX"] = custom_widgets.NewVarEntry(macroVarNames)
	w["TopY"] = custom_widgets.NewVarEntry(macroVarNames)
	w["RightX"] = custom_widgets.NewVarEntry(macroVarNames)
	w["BottomY"] = custom_widgets.NewVarEntry(macroVarNames)
	w["recordButton"] = widget.NewButtonWithIcon("", theme.MediaRecordIcon(), nil)
	w["recordButton"].(*widget.Button).Importance = widget.DangerImportance
	w["Form"] = widget.NewForm(
		widget.NewFormItem("Name", w["Name"]),
		widget.NewFormItem("LeftX", w["LeftX"]),
		widget.NewFormItem("TopY", w["TopY"]),
		widget.NewFormItem("RightX", w["RightX"]),
		widget.NewFormItem("BottomY", w["BottomY"]),
		widget.NewFormItem("", container.NewHBox(layout.NewSpacer(), w["recordButton"])),
	)
}

func buildSearchAreasRightPanel(programSelector *widget.Select, w map[string]fyne.CanvasObject, previewPanel *editorPreviewPanel, refreshBtn *widget.Button) fyne.CanvasObject {
	return container.NewBorder(
		container.NewVBox(LabeledProgramSelector(programSelector), w["Form"]),
		nil, nil, nil,
		buildPreviewColumn(previewPanel, refreshBtn),
	)
}

// populateMasksFormWidgets creates Masks tab right-side form widgets.
func populateMasksFormWidgets(w map[string]fyne.CanvasObject) {
	w["Name"] = new(widget.Entry)
	w["uploadButton"] = widget.NewButtonWithIcon("Upload Image", theme.FolderOpenIcon(), nil)
	w["removeImageButton"] = widget.NewButtonWithIcon("Remove Image", theme.ContentRemoveIcon(), nil)
	w["removeImageButton"].(*widget.Button).Importance = widget.DangerImportance
	w["removeImageButton"].(*widget.Button).Hide()
	w["imageStatus"] = widget.NewLabel("")
	w["imageStatus"].(*widget.Label).Hide()

	w["shapeSelect"] = widget.NewRadioGroup([]string{"Rectangle", "Circle"}, nil)
	w["shapeSelect"].(*widget.RadioGroup).Horizontal = true
	w["shapeSelect"].(*widget.RadioGroup).Required = true
	w["shapeSelect"].(*widget.RadioGroup).SetSelected("Rectangle")

	w["CenterX"] = custom_widgets.NewVarEntry(macroVarNames)
	w["CenterX"].(*custom_widgets.VarEntry).PlaceHolder = "50"
	w["CenterY"] = custom_widgets.NewVarEntry(macroVarNames)
	w["CenterY"].(*custom_widgets.VarEntry).PlaceHolder = "50"
	w["centerContainer"] = container.NewGridWithColumns(2,
		container.NewBorder(nil, nil, widget.NewLabel("X %"), nil, w["CenterX"]),
		container.NewBorder(nil, nil, widget.NewLabel("Y %"), nil, w["CenterY"]),
	)

	w["Base"] = custom_widgets.NewVarEntry(macroVarNames)
	w["Base"].(*custom_widgets.VarEntry).PlaceHolder = "base"
	w["Height"] = custom_widgets.NewVarEntry(macroVarNames)
	w["Height"].(*custom_widgets.VarEntry).PlaceHolder = "height"
	w["rectContainer"] = container.NewGridWithColumns(3,
		w["Base"],
		container.NewCenter(widget.NewLabel("*")),
		w["Height"],
	)

	w["Radius"] = custom_widgets.NewVarEntry(macroVarNames)
	w["Radius"].(*custom_widgets.VarEntry).PlaceHolder = "radius"
	w["circleContainer"] = container.NewBorder(
		nil, nil,
		widget.NewLabel("π *"), widget.NewLabel("²"),
		w["Radius"],
	)
	w["circleContainer"].(*fyne.Container).Hide()

	w["shapeParamsContainer"] = container.NewVBox(
		w["rectContainer"],
		w["circleContainer"],
	)

	w["shapeSelect"].(*widget.RadioGroup).OnChanged = func(selected string) {
		switch selected {
		case "Rectangle":
			w["rectContainer"].(*fyne.Container).Show()
			w["circleContainer"].(*fyne.Container).Hide()
		case "Circle":
			w["rectContainer"].(*fyne.Container).Hide()
			w["circleContainer"].(*fyne.Container).Show()
		}
	}

	w["Inverse"] = widget.NewCheck("Inverse (shape included, rest excluded)", nil)

	w["Form"] = widget.NewForm(
		widget.NewFormItem("Name", w["Name"]),
		widget.NewFormItem("Shape", w["shapeSelect"]),
		widget.NewFormItem("Center", w["centerContainer"]),
		widget.NewFormItem("", w["shapeParamsContainer"]),
		widget.NewFormItem("", w["Inverse"]),
	)
}

func buildMasksRightPanel(programSelector *widget.Select, w map[string]fyne.CanvasObject, previewPanel *editorPreviewPanel, refreshBtn *widget.Button) fyne.CanvasObject {
	return container.NewBorder(
		container.NewVBox(
			LabeledProgramSelector(programSelector),
			w["Form"],
			container.NewHBox(w["uploadButton"], w["removeImageButton"]),
			w["imageStatus"],
		),
		nil, nil, nil,
		buildPreviewColumn(previewPanel, refreshBtn),
	)
}

// setMaskImageModeOnWidgets toggles mask shape vs uploaded-image UI on the given widget map.
func setMaskImageModeOnWidgets(w map[string]fyne.CanvasObject, hasImage bool) {
	if hasImage {
		w["shapeSelect"].(*widget.RadioGroup).Hide()
		w["centerContainer"].(*fyne.Container).Hide()
		w["shapeParamsContainer"].(*fyne.Container).Hide()
		w["imageStatus"].(*widget.Label).SetText("Image mask uploaded")
		w["imageStatus"].(*widget.Label).Show()
		w["removeImageButton"].(*widget.Button).Show()
	} else {
		w["shapeSelect"].(*widget.RadioGroup).Show()
		w["centerContainer"].(*fyne.Container).Show()
		w["shapeParamsContainer"].(*fyne.Container).Show()
		selected := w["shapeSelect"].(*widget.RadioGroup).Selected
		switch selected {
		case "Circle":
			w["rectContainer"].(*fyne.Container).Hide()
			w["circleContainer"].(*fyne.Container).Show()
		default:
			w["rectContainer"].(*fyne.Container).Show()
			w["circleContainer"].(*fyne.Container).Hide()
		}
		w["imageStatus"].(*widget.Label).Hide()
		w["removeImageButton"].(*widget.Button).Hide()
	}
}
