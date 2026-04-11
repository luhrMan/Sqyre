package editor

import (
	"Sqyre/internal/config"
	"Sqyre/internal/models"
	"Sqyre/internal/models/repositories"
	"Sqyre/ui/custom_widgets"
	"errors"
	"fmt"
	"log"
	"os"
	"path/filepath"
	"strconv"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/container"
	"fyne.io/fyne/v2/dialog"
	"fyne.io/fyne/v2/layout"
	"fyne.io/fyne/v2/widget"
)

type createDialogConfig struct {
	title     string
	buildForm func() (content fyne.CanvasObject, widgets map[string]fyne.CanvasObject)
	prefill   func(widgets map[string]fyne.CanvasObject)
	onSave    func(widgets map[string]fyne.CanvasObject) error
	afterSave func()
}

func validateCreateName(name string) error {
	if name == "" {
		return errors.New("name cannot be empty")
	}
	return nil
}

func validateCreateProgramName(programName string) error {
	if programName == "" {
		return errors.New("program cannot be empty")
	}
	return nil
}

func ensureNameAvailable(name, objectType string, getByName func(string) (any, error)) error {
	if _, err := getByName(name); err == nil {
		return fmt.Errorf("a %s with that name already exists", objectType)
	}
	return nil
}

func showCreateDialog(cfg createDialogConfig, parent fyne.Window) {
	content, widgets := cfg.buildForm()
	cfg.prefill(widgets)

	var d dialog.Dialog
	saveButton := widget.NewButton("Create", func() {
		if err := cfg.onSave(widgets); err != nil {
			activeWire.ShowErrorWithEscape(err, parent)
			return
		}
		if cfg.afterSave != nil {
			cfg.afterSave()
		}
		d.Hide()
	})
	cancelButton := widget.NewButton("Cancel", func() {
		d.Hide()
	})
	saveButton.Importance = widget.SuccessImportance
	buttonBar := container.NewHBox(layout.NewSpacer(), saveButton, cancelButton)
	d = dialog.NewCustomWithoutButtons(cfg.title, container.NewBorder(nil, buttonBar, nil, nil, content), parent)
	activeWire.AddDialogEscapeClose(d, parent)
	d.Resize(fyne.NewSize(500, 300))
	d.Show()
}

func programCreateConfig() createDialogConfig {
	return createDialogConfig{
		title: "New Program",
		buildForm: func() (fyne.CanvasObject, map[string]fyne.CanvasObject) {
			w := map[string]fyne.CanvasObject{
				"Name": widget.NewEntry(),
			}
			form := widget.NewForm(
				widget.NewFormItem("Name", w["Name"]),
			)
			return form, w
		},
		prefill: func(w map[string]fyne.CanvasObject) {
			src := shell().EditorTabs.ProgramsTab.Widgets
			w["Name"].(*widget.Entry).SetText(src["Name"].(*widget.Entry).Text)
		},
		onSave: func(w map[string]fyne.CanvasObject) error {
			n := w["Name"].(*widget.Entry).Text
			if err := validateCreateName(n); err != nil {
				return err
			}
			if err := ensureNameAvailable(n, "program", func(name string) (any, error) {
				return repositories.ProgramRepo().Get(name)
			}); err != nil {
				return err
			}
			pro := repositories.ProgramRepo().New()
			pro.Name = n
			if err := repositories.ProgramRepo().Set(pro.Name, pro); err != nil {
				return err
			}
			shell().EditorTabs.ProgramsTab.SelectedItem = pro
			shell().RefreshEditorActionBar()
			return nil
		},
		afterSave: func() {
			refreshAllProgramRelatedUI()
			updateProgramSelectorOptions()
			markProgramsClean()
		},
	}
}

func itemCreateConfig() createDialogConfig {
	return createDialogConfig{
		title: "New Item",
		buildForm: func() (fyne.CanvasObject, map[string]fyne.CanvasObject) {
			ps := widget.NewSelect(repositories.ProgramRepo().GetAllKeys(), nil)
			w := map[string]fyne.CanvasObject{
				"ProgramSelector": ps,
				"Name":            widget.NewEntry(),
				"Cols":            widget.NewEntry(),
				"Rows":            widget.NewEntry(),
				"StackMax":        widget.NewEntry(),
			}
			form := widget.NewForm(
				widget.NewFormItem("Name", w["Name"]),
				widget.NewFormItem("Cols", w["Cols"]),
				widget.NewFormItem("Rows", w["Rows"]),
				widget.NewFormItem("StackMax", w["StackMax"]),
			)
			return container.NewVBox(LabeledProgramSelector(ps), form), w
		},
		prefill: func(w map[string]fyne.CanvasObject) {
			src := shell().EditorTabs.ItemsTab.Widgets
			w["ProgramSelector"].(*widget.Select).SetSelected(shell().EditorTabs.ItemsTab.ProgramSelector.Selected)
			w["Name"].(*widget.Entry).SetText(src["Name"].(*widget.Entry).Text)
			w["Cols"].(*widget.Entry).SetText(src["Cols"].(*widget.Entry).Text)
			w["Rows"].(*widget.Entry).SetText(src["Rows"].(*widget.Entry).Text)
			w["StackMax"].(*widget.Entry).SetText(src["StackMax"].(*widget.Entry).Text)
		},
		onSave: func(w map[string]fyne.CanvasObject) error {
			n := w["Name"].(*widget.Entry).Text
			if err := validateCreateName(n); err != nil {
				return err
			}
			programName := w["ProgramSelector"].(*widget.Select).Selected
			if err := validateCreateProgramName(programName); err != nil {
				return err
			}
			pro := getOrCreateProgram(programName)
			if pro == nil {
				return errors.New("failed to get or create program")
			}
			if err := ensureNameAvailable(n, "item", func(name string) (any, error) {
				return pro.ItemRepo().Get(name)
			}); err != nil {
				return err
			}
			x, _ := strconv.Atoi(w["Cols"].(*widget.Entry).Text)
			y, _ := strconv.Atoi(w["Rows"].(*widget.Entry).Text)
			sm, _ := strconv.Atoi(w["StackMax"].(*widget.Entry).Text)
			i := pro.ItemRepo().New()
			i.Name = n
			i.GridSize = [2]int{x, y}
			i.StackMax = sm
			if err := pro.ItemRepo().Set(i.Name, i); err != nil {
				return err
			}
			shell().EditorTabs.ItemsTab.SelectedItem = i
			shell().EditorTabs.ItemsTab.ProgramSelector.SetSelected(programName)
			setItemsWidgets(*i)
			return nil
		},
		afterSave: func() {
			if acc, ok := shell().EditorTabs.ItemsTab.Widgets["Accordion"].(*custom_widgets.AccordionWithHeaderWidgets); ok {
				setAccordionItemsLists(acc)
			}
			markItemsClean()
		},
	}
}

func pointCreateConfig() createDialogConfig {
	return createDialogConfig{
		title: "New Point",
		buildForm: func() (fyne.CanvasObject, map[string]fyne.CanvasObject) {
			ps := widget.NewSelect(repositories.ProgramRepo().GetAllKeys(), nil)
			w := map[string]fyne.CanvasObject{
				"ProgramSelector": ps,
				"Name":            widget.NewEntry(),
				"X":               widget.NewEntry(),
				"Y":               widget.NewEntry(),
			}
			form := widget.NewForm(
				widget.NewFormItem("Name", w["Name"]),
				widget.NewFormItem("X", w["X"]),
				widget.NewFormItem("Y", w["Y"]),
			)
			return container.NewVBox(LabeledProgramSelector(ps), form), w
		},
		prefill: func(w map[string]fyne.CanvasObject) {
			src := shell().EditorTabs.PointsTab.Widgets
			w["ProgramSelector"].(*widget.Select).SetSelected(shell().EditorTabs.PointsTab.ProgramSelector.Selected)
			w["Name"].(*widget.Entry).SetText(src["Name"].(*widget.Entry).Text)
			w["X"].(*widget.Entry).SetText(custom_widgets.EntryText(src["X"]))
			w["Y"].(*widget.Entry).SetText(custom_widgets.EntryText(src["Y"]))
		},
		onSave: func(w map[string]fyne.CanvasObject) error {
			n := w["Name"].(*widget.Entry).Text
			if err := validateCreateName(n); err != nil {
				return err
			}
			programName := w["ProgramSelector"].(*widget.Select).Selected
			if err := validateCreateProgramName(programName); err != nil {
				return err
			}
			pro := getOrCreateProgram(programName)
			if pro == nil {
				return errors.New("failed to get or create program")
			}
			if err := ensureNameAvailable(n, "point", func(name string) (any, error) {
				return pro.PointRepo(config.MainMonitorSizeString).Get(name)
			}); err != nil {
				return err
			}
			xVal := parseIntOrString(w["X"].(*widget.Entry).Text)
			yVal := parseIntOrString(w["Y"].(*widget.Entry).Text)
			p := pro.PointRepo(config.MainMonitorSizeString).New()
			p.Name = n
			p.X = xVal
			p.Y = yVal
			if err := pro.PointRepo(config.MainMonitorSizeString).Set(p.Name, p); err != nil {
				return err
			}
			shell().EditorTabs.PointsTab.SelectedItem = p
			shell().EditorTabs.PointsTab.ProgramSelector.SetSelected(programName)
			setPointWidgets(*p)
			return nil
		},
		afterSave: func() {
			if acc, ok := shell().EditorTabs.PointsTab.Widgets["Accordion"].(*widget.Accordion); ok {
				setAccordionPointsLists(acc)
			}
			markPointsClean()
		},
	}
}

func searchAreaCreateConfig() createDialogConfig {
	return createDialogConfig{
		title: "New Search Area",
		buildForm: func() (fyne.CanvasObject, map[string]fyne.CanvasObject) {
			ps := widget.NewSelect(repositories.ProgramRepo().GetAllKeys(), nil)
			w := map[string]fyne.CanvasObject{
				"ProgramSelector": ps,
				"Name":            widget.NewEntry(),
				"LeftX":           widget.NewEntry(),
				"TopY":            widget.NewEntry(),
				"RightX":          widget.NewEntry(),
				"BottomY":         widget.NewEntry(),
			}
			form := widget.NewForm(
				widget.NewFormItem("Name", w["Name"]),
				widget.NewFormItem("LeftX", w["LeftX"]),
				widget.NewFormItem("TopY", w["TopY"]),
				widget.NewFormItem("RightX", w["RightX"]),
				widget.NewFormItem("BottomY", w["BottomY"]),
			)
			return container.NewVBox(LabeledProgramSelector(ps), form), w
		},
		prefill: func(w map[string]fyne.CanvasObject) {
			src := shell().EditorTabs.SearchAreasTab.Widgets
			w["ProgramSelector"].(*widget.Select).SetSelected(shell().EditorTabs.SearchAreasTab.ProgramSelector.Selected)
			w["Name"].(*widget.Entry).SetText(src["Name"].(*widget.Entry).Text)
			w["LeftX"].(*widget.Entry).SetText(custom_widgets.EntryText(src["LeftX"]))
			w["TopY"].(*widget.Entry).SetText(custom_widgets.EntryText(src["TopY"]))
			w["RightX"].(*widget.Entry).SetText(custom_widgets.EntryText(src["RightX"]))
			w["BottomY"].(*widget.Entry).SetText(custom_widgets.EntryText(src["BottomY"]))
		},
		onSave: func(w map[string]fyne.CanvasObject) error {
			n := w["Name"].(*widget.Entry).Text
			if err := validateCreateName(n); err != nil {
				return err
			}
			programName := w["ProgramSelector"].(*widget.Select).Selected
			if err := validateCreateProgramName(programName); err != nil {
				return err
			}
			pro := getOrCreateProgram(programName)
			if pro == nil {
				return errors.New("failed to get or create program")
			}
			if err := ensureNameAvailable(n, "search area", func(name string) (any, error) {
				return pro.SearchAreaRepo(config.MainMonitorSizeString).Get(name)
			}); err != nil {
				return err
			}
			sa := pro.SearchAreaRepo(config.MainMonitorSizeString).New()
			sa.Name = n
			sa.LeftX = parseIntOrString(w["LeftX"].(*widget.Entry).Text)
			sa.TopY = parseIntOrString(w["TopY"].(*widget.Entry).Text)
			sa.RightX = parseIntOrString(w["RightX"].(*widget.Entry).Text)
			sa.BottomY = parseIntOrString(w["BottomY"].(*widget.Entry).Text)
			if err := pro.SearchAreaRepo(config.MainMonitorSizeString).Set(sa.Name, sa); err != nil {
				return err
			}
			shell().EditorTabs.SearchAreasTab.SelectedItem = sa
			shell().EditorTabs.SearchAreasTab.ProgramSelector.SetSelected(programName)
			setSearchAreaWidgets(*sa)
			return nil
		},
		afterSave: func() {
			syncEditorSearchAreaAccordions()
			markSearchAreasClean()
		},
	}
}

func maskCreateConfig() createDialogConfig {
	return createDialogConfig{
		title: "New Mask",
		buildForm: func() (fyne.CanvasObject, map[string]fyne.CanvasObject) {
			ps := widget.NewSelect(repositories.ProgramRepo().GetAllKeys(), nil)
			w := map[string]fyne.CanvasObject{
				"ProgramSelector": ps,
				"Name":            widget.NewEntry(),
				"shapeSelect":     widget.NewRadioGroup([]string{"Rectangle", "Circle"}, nil),
				"CenterX":         widget.NewEntry(),
				"CenterY":         widget.NewEntry(),
				"Base":            widget.NewEntry(),
				"Height":          widget.NewEntry(),
				"Radius":          widget.NewEntry(),
				"Inverse":         widget.NewCheck("Inverse (shape included, rest excluded)", nil),
			}
			w["shapeSelect"].(*widget.RadioGroup).Horizontal = true
			w["shapeSelect"].(*widget.RadioGroup).Required = true
			w["shapeSelect"].(*widget.RadioGroup).SetSelected("Rectangle")
			w["CenterX"].(*widget.Entry).PlaceHolder = "50"
			w["CenterY"].(*widget.Entry).PlaceHolder = "50"

			rectContainer := container.NewGridWithColumns(3,
				w["Base"], container.NewCenter(widget.NewLabel("*")), w["Height"],
			)
			circleContainer := container.NewBorder(
				nil, nil, widget.NewLabel("\u03c0 *"), widget.NewLabel("\u00b2"), w["Radius"],
			)
			circleContainer.Hide()

			w["shapeSelect"].(*widget.RadioGroup).OnChanged = func(selected string) {
				switch selected {
				case "Rectangle":
					rectContainer.Show()
					circleContainer.Hide()
				case "Circle":
					rectContainer.Hide()
					circleContainer.Show()
				}
			}

			centerContainer := container.NewGridWithColumns(2,
				container.NewBorder(nil, nil, widget.NewLabel("X %"), nil, w["CenterX"]),
				container.NewBorder(nil, nil, widget.NewLabel("Y %"), nil, w["CenterY"]),
			)

			form := widget.NewForm(
				widget.NewFormItem("Name", w["Name"]),
				widget.NewFormItem("Shape", w["shapeSelect"]),
				widget.NewFormItem("Center", centerContainer),
				widget.NewFormItem("", container.NewVBox(rectContainer, circleContainer)),
				widget.NewFormItem("", w["Inverse"]),
			)
			return container.NewVBox(LabeledProgramSelector(ps), form), w
		},
		prefill: func(w map[string]fyne.CanvasObject) {
			src := shell().EditorTabs.MasksTab.Widgets
			w["ProgramSelector"].(*widget.Select).SetSelected(shell().EditorTabs.MasksTab.ProgramSelector.Selected)
			w["Name"].(*widget.Entry).SetText(src["Name"].(*widget.Entry).Text)
			shape := src["shapeSelect"].(*widget.RadioGroup).Selected
			if shape == "" {
				shape = "Rectangle"
			}
			w["shapeSelect"].(*widget.RadioGroup).SetSelected(shape)
			w["CenterX"].(*widget.Entry).SetText(custom_widgets.EntryText(src["CenterX"]))
			w["CenterY"].(*widget.Entry).SetText(custom_widgets.EntryText(src["CenterY"]))
			w["Base"].(*widget.Entry).SetText(custom_widgets.EntryText(src["Base"]))
			w["Height"].(*widget.Entry).SetText(custom_widgets.EntryText(src["Height"]))
			w["Radius"].(*widget.Entry).SetText(custom_widgets.EntryText(src["Radius"]))
			if inv, ok := src["Inverse"].(*widget.Check); ok {
				w["Inverse"].(*widget.Check).SetChecked(inv.Checked)
			}
		},
		onSave: func(w map[string]fyne.CanvasObject) error {
			n := w["Name"].(*widget.Entry).Text
			if err := validateCreateName(n); err != nil {
				return err
			}
			programName := w["ProgramSelector"].(*widget.Select).Selected
			if err := validateCreateProgramName(programName); err != nil {
				return err
			}
			pro := getOrCreateProgram(programName)
			if pro == nil {
				return errors.New("failed to get or create program")
			}
			if err := ensureNameAvailable(n, "mask", func(name string) (any, error) {
				return pro.MaskRepo().Get(name)
			}); err != nil {
				return err
			}
			shape := "rectangle"
			if w["shapeSelect"].(*widget.RadioGroup).Selected == "Circle" {
				shape = "circle"
			}
			m := pro.MaskRepo().New()
			m.Name = n
			m.Shape = shape
			m.CenterX = w["CenterX"].(*widget.Entry).Text
			m.CenterY = w["CenterY"].(*widget.Entry).Text
			m.Base = w["Base"].(*widget.Entry).Text
			m.Height = w["Height"].(*widget.Entry).Text
			m.Radius = w["Radius"].(*widget.Entry).Text
			if inv, ok := w["Inverse"].(*widget.Check); ok {
				m.Inverse = inv.Checked
			}
			if err := pro.MaskRepo().Set(m.Name, m); err != nil {
				return err
			}
			shell().EditorTabs.MasksTab.SelectedItem = m
			shell().EditorTabs.MasksTab.ProgramSelector.SetSelected(programName)
			setMaskWidgets(*m, programName)
			return nil
		},
		afterSave: func() {
			if acc, ok := shell().EditorTabs.MasksTab.Widgets["Accordion"].(*widget.Accordion); ok {
				setAccordionMasksLists(acc)
			}
			markMasksClean()
		},
	}
}

// performDeleteForTab executes the delete operation for the currently active editor tab.
func performDeleteForTab() {
	program := shell().ActiveProgramName()
	et := shell().EditorTabs
	prog, err := repositories.ProgramRepo().Get(program)

	switch shell().EditorTabs.Selected().Text {
	case "Programs":
		if v, ok := et.ProgramsTab.SelectedItem.(*models.Program); ok {
			if err := repositories.ProgramRepo().Delete(v.Name); err != nil {
				log.Printf("Error deleting program: %v", err)
			}
			refreshAllProgramRelatedUI()
			updateProgramSelectorOptions()
			et.ProgramsTab.SelectedItem = repositories.ProgramRepo().New()
			if list, ok := et.ProgramsTab.Widgets["list"].(*widget.List); ok {
				list.UnselectAll()
			}
		}
	case "Items":
		if err != nil {
			log.Printf("Error getting program %s: %v", program, err)
			return
		}
		if v, ok := et.ItemsTab.SelectedItem.(*models.Item); ok {
			if err := prog.ItemRepo().Delete(v.Name); err != nil {
				log.Printf("Error deleting item %s: %v", v.Name, err)
			}
			if prog != nil {
				et.ItemsTab.SelectedItem = prog.ItemRepo().New()
			} else {
				et.ItemsTab.SelectedItem = &models.Item{}
			}
			if acc, ok := et.ItemsTab.Widgets["Accordion"].(*custom_widgets.AccordionWithHeaderWidgets); ok {
				setAccordionItemsLists(acc)
			}
			if list, ok := et.ItemsTab.Widgets[program+"-list"].(*widget.GridWrap); ok {
				list.UnselectAll()
			}
		}
	case "Points":
		if err != nil {
			log.Printf("Error getting program %s: %v", program, err)
			return
		}
		if v, ok := et.PointsTab.SelectedItem.(*models.Point); ok {
			if err := prog.PointRepo(config.MainMonitorSizeString).Delete(v.Name); err != nil {
				log.Printf("Error deleting point %s: %v", v.Name, err)
			}
			if prog != nil {
				et.PointsTab.SelectedItem = prog.PointRepo(config.MainMonitorSizeString).New()
			} else {
				et.PointsTab.SelectedItem = &models.Point{}
			}
			if acc, ok := et.PointsTab.Widgets["Accordion"].(*widget.Accordion); ok {
				setAccordionPointsLists(acc)
			}
			if list, ok := et.PointsTab.Widgets[program+"-list"].(*widget.List); ok {
				list.UnselectAll()
			}
		}
	case "Masks":
		if err != nil {
			log.Printf("Error getting program %s: %v", program, err)
			return
		}
		if v, ok := et.MasksTab.SelectedItem.(*models.Mask); ok {
			if delErr := prog.MaskRepo().Delete(v.Name); delErr != nil {
				log.Printf("Error deleting mask %s: %v", v.Name, delErr)
				return
			}
			masksPath := config.GetMasksPath()
			imgPath := filepath.Join(masksPath, program, v.Name+config.PNG)
			if removeErr := os.Remove(imgPath); removeErr != nil && !os.IsNotExist(removeErr) {
				log.Printf("Warning: Failed to remove mask image %s: %v", imgPath, removeErr)
			}
			et.MasksTab.SelectedItem = &models.Mask{}
			shell().SetMaskImageMode(false)
			shell().ClearMaskPreviewImage()
			if acc, ok := et.MasksTab.Widgets["Accordion"].(*widget.Accordion); ok {
				setAccordionMasksLists(acc)
			}
			if list, ok := et.MasksTab.Widgets[program+"-list"].(*widget.List); ok {
				list.UnselectAll()
			}
		}
	case "Search Areas":
		if err != nil {
			log.Printf("Error getting program %s: %v", program, err)
			return
		}
		if v, ok := et.SearchAreasTab.SelectedItem.(*models.SearchArea); ok {
			if delErr := prog.SearchAreaRepo(config.MainMonitorSizeString).Delete(v.Name); delErr != nil {
				log.Printf("Error deleting searcharea %s: %v", v.Name, delErr)
				return
			}
			if prog != nil {
				et.SearchAreasTab.SelectedItem = prog.SearchAreaRepo(config.MainMonitorSizeString).New()
			} else {
				et.SearchAreasTab.SelectedItem = &models.SearchArea{}
			}
			syncEditorSearchAreaAccordions()
			if list, ok := et.SearchAreasTab.Widgets[program+"-list"].(*widget.List); ok {
				list.UnselectAll()
			}
		}
	}
	shell().RefreshEditorActionBar()
}
