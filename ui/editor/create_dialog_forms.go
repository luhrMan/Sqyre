package editor

import (
	"Sqyre/internal/models"
	"Sqyre/ui/custom_widgets"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/container"
	"fyne.io/fyne/v2/layout"
	"fyne.io/fyne/v2/theme"
	"fyne.io/fyne/v2/widget"
)

const createDialogCoordStep = 1

func newCreateCoordIncrementer(value int) *custom_widgets.Incrementer {
	return custom_widgets.NewIncrementerWithEntry(value, createDialogCoordStep, nil, nil)
}

func wireCreateCoordPreview(w map[string]fyne.CanvasObject, keys []string, refresh func()) {
	for _, key := range keys {
		inc, ok := w[key].(*custom_widgets.Incrementer)
		if !ok {
			continue
		}
		inc.OnChanged = func(int) { refresh() }
	}
}

func populatePointsCreateFormWidgets(w map[string]fyne.CanvasObject) {
	w["Name"] = new(widget.Entry)
	w["X"] = newCreateCoordIncrementer(0)
	w["Y"] = newCreateCoordIncrementer(0)
	w["recordButton"] = widget.NewButtonWithIcon("", theme.MediaRecordIcon(), nil)
	w["recordButton"].(*widget.Button).Importance = widget.DangerImportance
	w["Form"] = widget.NewForm(
		widget.NewFormItem("Name", w["Name"]),
		widget.NewFormItem("X", w["X"]),
		widget.NewFormItem("Y", w["Y"]),
		widget.NewFormItem("", container.NewHBox(layout.NewSpacer(), w["recordButton"])),
	)
}

func populateSearchAreasCreateFormWidgets(w map[string]fyne.CanvasObject) {
	w["Name"] = new(widget.Entry)
	w["LeftX"] = newCreateCoordIncrementer(0)
	w["TopY"] = newCreateCoordIncrementer(0)
	w["RightX"] = newCreateCoordIncrementer(0)
	w["BottomY"] = newCreateCoordIncrementer(0)
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

func populateMasksCreateFormWidgets(w map[string]fyne.CanvasObject) {
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

	w["CenterX"] = newCreateCoordIncrementer(0)
	w["CenterY"] = newCreateCoordIncrementer(0)
	w["centerContainer"] = container.NewGridWithColumns(2,
		container.NewBorder(nil, nil, widget.NewLabel("X %"), nil, w["CenterX"]),
		container.NewBorder(nil, nil, widget.NewLabel("Y %"), nil, w["CenterY"]),
	)

	w["Base"] = newCreateCoordIncrementer(0)
	w["Height"] = newCreateCoordIncrementer(0)
	w["rectContainer"] = container.NewGridWithColumns(3,
		w["Base"],
		container.NewCenter(widget.NewLabel("*")),
		w["Height"],
	)

	w["Radius"] = newCreateCoordIncrementer(0)
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

func prefillPointCreateDialog(w map[string]fyne.CanvasObject) {
	tab := shell().EditorTabs.PointsTab
	w["ProgramSelector"].(*widget.Select).SetSelected(tab.ProgramSelector.Selected)
	copyTabWidgetsToDialog(tab.Widgets, w, "Name", "X", "Y")
}

func prefillSearchAreaCreateDialog(w map[string]fyne.CanvasObject) {
	tab := shell().EditorTabs.SearchAreasTab
	w["ProgramSelector"].(*widget.Select).SetSelected(tab.ProgramSelector.Selected)
	copyTabWidgetsToDialog(tab.Widgets, w, "Name", "LeftX", "TopY", "RightX", "BottomY")
}

func prefillMaskCreateDialog(w map[string]fyne.CanvasObject) {
	tab := shell().EditorTabs.MasksTab
	w["ProgramSelector"].(*widget.Select).SetSelected(tab.ProgramSelector.Selected)
	copyTabWidgetsToDialog(tab.Widgets, w, "Name", "CenterX", "CenterY", "Base", "Height", "Radius", "shapeSelect", "Inverse")
	setMaskImageModeOnWidgets(w, false)
}

func prefillCollectionCreateDialog(w map[string]fyne.CanvasObject) {
	tab := shell().EditorTabs.CollectionsTab
	programName := ""
	if tab.ProgramSelector != nil {
		programName = tab.ProgramSelector.Selected
		w["ProgramSelector"].(*widget.Select).SetSelected(programName)
	}
	copyTabWidgetsToDialog(tab.Widgets, w, "Name", "Rows", "Cols")
	refreshSearchAreaSelectOptions(w, programName)
	if src, ok := tab.Widgets["searchAreaSelect"].(*widget.Select); ok {
		if dst, ok := w["searchAreaSelect"].(*widget.Select); ok && src.Selected != "" {
			dst.SetSelected(src.Selected)
		}
	}
}

func prefillItemCreateDialog(w map[string]fyne.CanvasObject, ctx *createDialogContext) {
	tab := shell().EditorTabs.ItemsTab
	w["ProgramSelector"].(*widget.Select).SetSelected(tab.ProgramSelector.Selected)
	copyTabWidgetsToDialog(tab.Widgets, w, "Name", "Cols", "Rows", "StackMax")
	ctx.draftItem = &models.Item{}
	if item, ok := tab.SelectedItem.(*models.Item); ok {
		ctx.draftItem.Tags = append([]string(nil), item.Tags...)
		ctx.draftItem.Mask = item.Mask
	}
}

func prefillProgramCreateDialog(w map[string]fyne.CanvasObject) {
	copyTabWidgetsToDialog(shell().EditorTabs.ProgramsTab.Widgets, w, "Name")
}
