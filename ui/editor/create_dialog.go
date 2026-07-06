package editor

import (
	"Sqyre/internal/config"
	"Sqyre/internal/models/repositories"
	"Sqyre/internal/validation"
	"Sqyre/ui/custom_widgets"
	"errors"
	"fmt"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/container"
	"fyne.io/fyne/v2/dialog"
	"fyne.io/fyne/v2/layout"
	"fyne.io/fyne/v2/widget"
)

type createDialogConfig struct {
	title      string
	dialogSize fyne.Size
	buildForm  func() (content fyne.CanvasObject, widgets map[string]fyne.CanvasObject)
	prefill    func(widgets map[string]fyne.CanvasObject)
	wire       func(widgets map[string]fyne.CanvasObject)
	onSave     func(widgets map[string]fyne.CanvasObject) error
	afterSave  func()
}

func validateCreateName(name string) error {
	return validation.ValidateEntityName(name)
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
	if cfg.wire != nil {
		cfg.wire(widgets)
	}

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
	scroll := container.NewVScroll(content)
	size := cfg.dialogSize
	if size.Width == 0 || size.Height == 0 {
		size = fyne.NewSize(500, 300)
	}
	scroll.SetMinSize(size)
	d = dialog.NewCustomWithoutButtons(cfg.title, container.NewBorder(nil, buttonBar, nil, nil, scroll), parent)
	activeWire.AddDialogEscapeClose(d, parent)
	d.Resize(fyne.NewSize(size.Width+40, size.Height+80))
	d.Show()
}

func programCreateConfig() createDialogConfig {
	return createDialogConfig{
		title:      "New Program",
		dialogSize: fyne.NewSize(400, 120),
		buildForm: func() (fyne.CanvasObject, map[string]fyne.CanvasObject) {
			w := make(map[string]fyne.CanvasObject)
			populateProgramsFormWidgets(w)
			return buildProgramsRightPanel(w), w
		},
		prefill: prefillProgramCreateDialog,
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
	var draftCtx createDialogContext
	return createDialogConfig{
		title:      "New Item",
		dialogSize: fyne.NewSize(800, 700),
		buildForm: func() (fyne.CanvasObject, map[string]fyne.CanvasObject) {
			w := make(map[string]fyne.CanvasObject)
			ps := widget.NewSelect(repositories.ProgramRepo().GetAllKeys(), nil)
			w["ProgramSelector"] = ps
			populateItemsFormWidgets(w, activeWire.Window)
			return buildItemsRightPanel(ps, w), w
		},
		prefill: func(w map[string]fyne.CanvasObject) {
			prefillItemCreateDialog(w, &draftCtx)
		},
		wire: func(w map[string]fyne.CanvasObject) {
			ps := w["ProgramSelector"].(*widget.Select)
			wireCreateItemDialog(w, ps, &draftCtx)
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
				return ProgramItemRepo(pro).Get(name)
			}); err != nil {
				return err
			}
			x, err := validation.ParsePositiveInt(w["Cols"].(*widget.Entry).Text)
			if err != nil {
				return fmt.Errorf("cols: %w", err)
			}
			y, err := validation.ParsePositiveInt(w["Rows"].(*widget.Entry).Text)
			if err != nil {
				return fmt.Errorf("rows: %w", err)
			}
			sm, err := validation.ParseNonNegativeInt(w["StackMax"].(*widget.Entry).Text)
			if err != nil {
				return fmt.Errorf("stack max: %w", err)
			}
			i := ProgramItemRepo(pro).New()
			i.Name = n
			i.GridSize = [2]int{x, y}
			i.StackMax = sm
			if draftCtx.draftItem != nil {
				i.Tags = append([]string(nil), draftCtx.draftItem.Tags...)
				i.Mask = draftCtx.draftItem.Mask
			}
			if err := ProgramItemRepo(pro).Set(i.Name, i); err != nil {
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
	var previewPanel *editorPreviewPanel
	var refreshBtn *widget.Button
	return createDialogConfig{
		title:      "New Point",
		dialogSize: fyne.NewSize(850, 650),
		buildForm: func() (fyne.CanvasObject, map[string]fyne.CanvasObject) {
			w := make(map[string]fyne.CanvasObject)
			ps := widget.NewSelect(repositories.ProgramRepo().GetAllKeys(), nil)
			w["ProgramSelector"] = ps
			populatePointsCreateFormWidgets(w)
			previewPanel = newEditorPreviewPanel()
			refreshBtn = newEditorPreviewRefreshButton()
			return buildPointsRightPanel(ps, w, previewPanel, refreshBtn), w
		},
		prefill: prefillPointCreateDialog,
		wire: func(w map[string]fyne.CanvasObject) {
			wireCreateCoordPreview(w, []string{"X", "Y"}, func() {
				safeUpdatePointPreviewPanel(previewPanel, pointFromWidgets(w))
			})
			wirePointRecordButton(w, func(x, y int) {
				p := pointFromWidgets(w)
				p.X = x
				p.Y = y
				safeUpdatePointPreviewPanel(previewPanel, p)
			})
			wirePointPreviewRefresh(previewPanel, refreshBtn, w)
			safeUpdatePointPreviewPanel(previewPanel, pointFromWidgets(w))
		},
		onSave: func(w map[string]fyne.CanvasObject) error {
			p := pointFromWidgets(w)
			if err := validateCreateName(p.Name); err != nil {
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
			if err := ensureNameAvailable(p.Name, "point", func(name string) (any, error) {
				return ProgramPointRepo(pro, config.MainMonitorSizeString).Get(name)
			}); err != nil {
				return err
			}
			newPoint := ProgramPointRepo(pro, config.MainMonitorSizeString).New()
			newPoint.Name = p.Name
			newPoint.X = p.X
			newPoint.Y = p.Y
			if err := ProgramPointRepo(pro, config.MainMonitorSizeString).Set(newPoint.Name, newPoint); err != nil {
				return err
			}
			shell().EditorTabs.PointsTab.SelectedItem = newPoint
			shell().EditorTabs.PointsTab.ProgramSelector.SetSelected(programName)
			setPointWidgets(*newPoint)
			return nil
		},
		afterSave: func() {
			refreshPointsAccordionForProgram(shell().EditorTabs.PointsTab.ProgramSelector.Selected)
			markPointsClean()
		},
	}
}

func searchAreaCreateConfig() createDialogConfig {
	var previewPanel *editorPreviewPanel
	var refreshBtn *widget.Button
	return createDialogConfig{
		title:      "New Search Area",
		dialogSize: fyne.NewSize(850, 650),
		buildForm: func() (fyne.CanvasObject, map[string]fyne.CanvasObject) {
			w := make(map[string]fyne.CanvasObject)
			ps := widget.NewSelect(repositories.ProgramRepo().GetAllKeys(), nil)
			w["ProgramSelector"] = ps
			populateSearchAreasCreateFormWidgets(w)
			previewPanel = newEditorPreviewPanel()
			refreshBtn = newEditorPreviewRefreshButton()
			return buildSearchAreasRightPanel(ps, w, previewPanel, refreshBtn), w
		},
		prefill: prefillSearchAreaCreateDialog,
		wire: func(w map[string]fyne.CanvasObject) {
			wireCreateCoordPreview(w, []string{"LeftX", "TopY", "RightX", "BottomY"}, func() {
				safeUpdateSearchAreaPreviewPanel(previewPanel, searchAreaFromWidgets(w))
			})
			wireSearchAreaRecordButton(w, func(lx, ty, rx, by int) {
				sa := searchAreaFromWidgets(w)
				sa.LeftX = lx
				sa.TopY = ty
				sa.RightX = rx
				sa.BottomY = by
				safeUpdateSearchAreaPreviewPanel(previewPanel, sa)
			})
			wireSearchAreaPreviewRefresh(previewPanel, refreshBtn, w)
			safeUpdateSearchAreaPreviewPanel(previewPanel, searchAreaFromWidgets(w))
		},
		onSave: func(w map[string]fyne.CanvasObject) error {
			sa := searchAreaFromWidgets(w)
			if err := validateCreateName(sa.Name); err != nil {
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
			if err := ensureNameAvailable(sa.Name, "search area", func(name string) (any, error) {
				return ProgramSearchAreaRepo(pro, config.MainMonitorSizeString).Get(name)
			}); err != nil {
				return err
			}
			if err := validation.ValidateSearchAreaSave(sa); err != nil {
				return err
			}
			newSA := ProgramSearchAreaRepo(pro, config.MainMonitorSizeString).New()
			newSA.Name = sa.Name
			newSA.LeftX = sa.LeftX
			newSA.TopY = sa.TopY
			newSA.RightX = sa.RightX
			newSA.BottomY = sa.BottomY
			if err := ProgramSearchAreaRepo(pro, config.MainMonitorSizeString).Set(newSA.Name, newSA); err != nil {
				return err
			}
			shell().EditorTabs.SearchAreasTab.SelectedItem = newSA
			shell().EditorTabs.SearchAreasTab.ProgramSelector.SetSelected(programName)
			setSearchAreaWidgets(*newSA)
			return nil
		},
		afterSave: func() {
			refreshSearchAreasAccordionForProgram(shell().EditorTabs.SearchAreasTab.ProgramSelector.Selected)
			markSearchAreasClean()
		},
	}
}

func maskCreateConfig() createDialogConfig {
	var previewPanel *editorPreviewPanel
	var refreshBtn *widget.Button
	return createDialogConfig{
		title:      "New Mask",
		dialogSize: fyne.NewSize(850, 650),
		buildForm: func() (fyne.CanvasObject, map[string]fyne.CanvasObject) {
			w := make(map[string]fyne.CanvasObject)
			ps := widget.NewSelect(repositories.ProgramRepo().GetAllKeys(), nil)
			w["ProgramSelector"] = ps
			populateMasksCreateFormWidgets(w)
			previewPanel = newEditorPreviewPanel()
			refreshBtn = newEditorPreviewRefreshButton()
			return buildMasksRightPanel(ps, w, previewPanel, refreshBtn), w
		},
		prefill: prefillMaskCreateDialog,
		wire: func(w map[string]fyne.CanvasObject) {
			ps := w["ProgramSelector"].(*widget.Select)
			wireCreateMaskDialog(w, ps, previewPanel, refreshBtn)
		},
		onSave: func(w map[string]fyne.CanvasObject) error {
			m := maskFromWidgets(w)
			if err := validateCreateName(m.Name); err != nil {
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
			if err := ensureNameAvailable(m.Name, "mask", func(name string) (any, error) {
				return ProgramMaskRepo(pro).Get(name)
			}); err != nil {
				return err
			}
			if err := ProgramMaskRepo(pro).Set(m.Name, m); err != nil {
				return err
			}
			shell().EditorTabs.MasksTab.SelectedItem = m
			shell().EditorTabs.MasksTab.ProgramSelector.SetSelected(programName)
			setMaskWidgets(*m, programName)
			return nil
		},
		afterSave: func() {
			refreshMasksAccordionForProgram(shell().EditorTabs.MasksTab.ProgramSelector.Selected)
			markMasksClean()
		},
	}
}

