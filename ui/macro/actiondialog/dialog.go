package actiondialog

import (
	"Sqyre/internal/config"
	"Sqyre/internal/models"
	"Sqyre/internal/models/actions"
	"Sqyre/internal/models/repositories"
	"Sqyre/ui/custom_widgets"
	"Sqyre/ui/editor"
	"fmt"
	"slices"
	"sort"
	"strings"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/container"
	"fyne.io/fyne/v2/dialog"
	"fyne.io/fyne/v2/layout"
	"fyne.io/fyne/v2/theme"
	"fyne.io/fyne/v2/widget"
	fynetooltip "github.com/dweymouth/fyne-tooltip"
	ttwidget "github.com/dweymouth/fyne-tooltip/widget"
	"github.com/lithammer/fuzzysearch/fuzzy"
)

// formHint is a FormItem with no HintText (no helper line under fields, to save vertical space).
// The hint is shown as a hover tooltip on the control when it supports SetToolTip (fyne-tooltip
// widgets), or on a compact help icon beside the field otherwise. Name / Variable Name rows skip the icon.
func formHint(label string, w fyne.CanvasObject, hint string) *widget.FormItem {
	return &widget.FormItem{Text: label, Widget: applyFieldTooltip(label, w, hint), HintText: ""}
}

func isNameFieldLabel(label string) bool {
	switch strings.TrimSpace(label) {
	case "Name:", "Variable Name:", "Output Variable:", "Output X Variable:", "Output Y Variable:", "Length Variable (optional):":
		return true
	default:
		return false
	}
}

func applyFieldTooltip(label string, w fyne.CanvasObject, hint string) fyne.CanvasObject {
	if hint == "" {
		return w
	}
	if ts, ok := w.(interface{ SetToolTip(string) }); ok {
		ts.SetToolTip(hint)
		return w
	}
	if isNameFieldLabel(label) {
		return w
	}
	icon := ttwidget.NewIcon(theme.HelpIcon())
	icon.SetToolTip(hint)
	// Use Border so the field expands to the form column width; HBox would keep each child at MinSize width only.
	return container.NewBorder(nil, nil, icon, nil, w)
}

// newVarNameEntry creates an entry for defining or selecting a macro variable name.
func newVarNameEntry() *custom_widgets.VarNameEntry {
	return custom_widgets.NewVarNameEntryWithDefs(macroVariableDefs)
}

// waitTilFoundForm bundles the "Wait until found" checkbox and timeout / interval
// incrementers shared by OCR, Image Search, and Find Pixel dialogs.
type waitTilFoundForm struct {
	Check               *ttwidget.Check
	SecondsIncrementer  *custom_widgets.Incrementer
	IntervalIncrementer *custom_widgets.Incrementer
}

// newWaitTilFoundForm builds wait-until-found UI. intervalUIMin is the minimum value
// enforced by the interval incrementer (100 for image search / find pixel, 0 for OCR).
func newWaitTilFoundForm(waitTilFound bool, waitSeconds, intervalMs int, intervalUIMin int) *waitTilFoundForm {
	check := ttwidget.NewCheck("Wait until found", nil)
	check.SetChecked(waitTilFound)
	check.SetToolTip("When enabled, the search repeats until a match is found or the timeout elapses. Sub-actions run for each successful match. When disabled, the search runs once.")
	secondsMin := 0
	secondsInc := custom_widgets.NewIncrementerWithEntry(waitSeconds, 1, &secondsMin, nil)
	if waitSeconds <= 0 {
		secondsInc.SetValue(10)
	} else {
		secondsInc.SetValue(waitSeconds)
	}
	intervalMin := intervalUIMin
	intervalInc := custom_widgets.NewIncrementerWithEntry(intervalMs, 100, &intervalMin, nil)
	if intervalMs < 100 {
		intervalInc.SetValue(100)
	} else {
		intervalInc.SetValue(intervalMs)
	}
	setEnabled := func(enabled bool) {
		if enabled {
			secondsInc.Enable()
			intervalInc.Enable()
			return
		}
		secondsInc.Disable()
		intervalInc.Disable()
	}
	check.OnChanged = setEnabled
	setEnabled(check.Checked)
	return &waitTilFoundForm{
		Check:               check,
		SecondsIncrementer:  secondsInc,
		IntervalIncrementer: intervalInc,
	}
}

func (w *waitTilFoundForm) writeTo(waitTilFound *bool, seconds *int, intervalMs *int) {
	*waitTilFound = w.Check.Checked
	if w.SecondsIncrementer.Value >= 0 {
		*seconds = w.SecondsIncrementer.Value
	}
	if w.IntervalIncrementer.Value >= 0 {
		*intervalMs = w.IntervalIncrementer.Value
	}
}

// smoothMoveForm bundles the smooth-move checkbox and speed / delay controls.
type smoothMoveForm struct {
	Check             *ttwidget.Check
	LowIncrementer    *custom_widgets.FloatIncrementer
	HighIncrementer   *custom_widgets.FloatIncrementer
	DelayIncrementer  *custom_widgets.Incrementer
}

func newSmoothMoveForm(smooth bool, low, high float64, delayMs int) *smoothMoveForm {
	check := ttwidget.NewCheck("Smooth", nil)
	check.SetChecked(smooth)
	check.SetToolTip("When enabled, the mouse moves along a smooth path to the target. When disabled, the cursor jumps instantly.")

	lowMin, lowMax := 0.05, 10.0
	highMin, highMax := 0.05, 50.0
	lowInc := custom_widgets.NewFloatIncrementer(low, 0.05, &lowMin, &lowMax, 2)
	highInc := custom_widgets.NewFloatIncrementer(high, 0.05, &highMin, &highMax, 2)
	delayMin, delayMax := 0, 200
	delayInc := custom_widgets.NewIncrementerWithEntry(delayMs, 1, &delayMin, &delayMax)
	delayInc.SetValue(delayMs)

	setEnabled := func(enabled bool) {
		if enabled {
			lowInc.Enable()
			highInc.Enable()
			delayInc.Enable()
			return
		}
		lowInc.Disable()
		highInc.Disable()
		delayInc.Disable()
	}
	check.OnChanged = setEnabled
	setEnabled(check.Checked)

	return &smoothMoveForm{
		Check:            check,
		LowIncrementer:   lowInc,
		HighIncrementer:  highInc,
		DelayIncrementer: delayInc,
	}
}

func (s *smoothMoveForm) writeTo(smooth *bool, low, high *float64, delayMs *int) {
	*smooth = s.Check.Checked
	*low = s.LowIncrementer.Value
	*high = s.HighIncrementer.Value
	*delayMs = s.DelayIncrementer.Value
}

func (s *smoothMoveForm) formItems() []*widget.FormItem {
	return []*widget.FormItem{
		formHint("Speed min:", s.LowIncrementer, "Shortest pause between cursor steps, in milliseconds. Lower values make the move faster; higher values slow it down."),
		formHint("Speed max:", s.HighIncrementer, "Longest pause between cursor steps, in milliseconds. Each step waits a random time between Speed min and this value. Should be at least Speed min."),
		formHint("Step delay (ms):", s.DelayIncrementer, "Extra pause in milliseconds after the smooth move finishes. Increase to add a brief hold at the destination."),
	}
}

// programListAccordionConfig configures the generic program list accordion builder.
// Callbacks receive the program and item key; implementors look up the model and invoke dialog-specific logic.
type programListAccordionConfig struct {
	GetKeys        func(*models.Program) []string
	GetDisplayName func(*models.Program, string) string
	// GetTooltip is optional; when set, each list row shows this text as a hover tooltip (e.g. coordinates).
	GetTooltip func(*models.Program, string) string
	// GetPreviewImage is optional; when set, each list row shows a hover popup with a screen capture preview.
	GetPreviewImage func(*models.Program, string) (custom_widgets.PreviewTooltipResult, error)
	OnSelect        func(*models.Program, string)
}

func newProgramListRowTemplate(cfg programListAccordionConfig) fyne.CanvasObject {
	if cfg.GetPreviewImage != nil {
		return custom_widgets.PreviewListRowTemplate()
	}
	return ttwidget.NewLabel("template")
}

func bindProgramListRow(co fyne.CanvasObject, cfg programListAccordionConfig, program *models.Program, key, labelText string) {
	if cfg.GetPreviewImage != nil {
		prog := program
		var onEdit custom_widgets.PreviewTooltipEditFunc
		if cfg.OnSelect != nil {
			onEdit = func() { cfg.OnSelect(prog, key) }
		}
		custom_widgets.BindPreviewListRow(co, labelText, func() (custom_widgets.PreviewTooltipResult, error) {
			return cfg.GetPreviewImage(prog, key)
		}, onEdit)
		return
	}
	lbl := co.(*ttwidget.Label)
	lbl.SetText(labelText)
	if cfg.GetTooltip != nil {
		lbl.SetToolTip(cfg.GetTooltip(program, key))
	}
}

// resolveCoordinateRefKey finds the repository key for ref within program p, if present.
func resolveCoordinateRefKey(ref actions.CoordinateRef, p *models.Program, getKeys func(*models.Program) []string) (string, bool) {
	if ref.IsEmpty() {
		return "", false
	}
	name := ref.Name()
	if programName := ref.Program(); programName != "" {
		if programName != p.Name {
			return "", false
		}
		if slices.Contains(getKeys(p), name) {
			return name, true
		}
		return "", false
	}
	if slices.Contains(getKeys(p), name) {
		return name, true
	}
	return "", false
}

// buildProgramListAccordionWithSearchbar builds an accordion of programs, each with a list of items (e.g. points or search areas).
// One searchbar above filters by program name or item key (fuzzy). Config provides key source, label text, and selection callback.
// When initialRef is set, the matching program accordion row is opened and the item is selected.
func buildProgramListAccordionWithSearchbar(cfg programListAccordionConfig, initialRef actions.CoordinateRef) (*widget.Entry, *widget.Accordion) {
	searchbar := widget.NewEntry()
	searchbar.SetPlaceHolder("Filter programs and entries (fuzzy match)")
	searchDebounce := custom_widgets.NewDebouncer(custom_widgets.DefaultSearchDebounce)
	acc := widget.NewAccordion()
	rebuild := func() {
		filterText := searchbar.Text
		acc.Items = nil
		var selectAccordionIndex int = -1
		var selectList *widget.List
		var selectListIndex widget.ListItemID = -1
		accordionIndex := 0
		for _, p := range repositories.ProgramRepo().GetAllSortedByName() {
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
			sort.Slice(filtered, func(i, j int) bool {
				return strings.Compare(cfg.GetDisplayName(p, filtered[i]), cfg.GetDisplayName(p, filtered[j])) < 0
			})
			if filterText != "" && !fuzzy.MatchFold(filterText, p.Name) && len(filtered) == 0 {
				continue
			}
			list := widget.NewList(
				func() int { return len(filtered) },
				func() fyne.CanvasObject { return newProgramListRowTemplate(cfg) },
				func(id widget.ListItemID, co fyne.CanvasObject) {
					key := filtered[id]
					bindProgramListRow(co, cfg, p, key, cfg.GetDisplayName(p, key))
				},
			)
			prog := p
			list.OnSelected = func(id widget.ListItemID) {
				if id >= 0 && id < len(filtered) {
					cfg.OnSelect(prog, filtered[id])
				}
			}
			if selectListIndex < 0 {
				if key, ok := resolveCoordinateRefKey(initialRef, p, cfg.GetKeys); ok {
					if idx := slices.Index(filtered, key); idx >= 0 {
						selectAccordionIndex = accordionIndex
						selectList = list
						selectListIndex = widget.ListItemID(idx)
					}
				}
			}
			acc.Append(widget.NewAccordionItem(fmt.Sprintf("%s (%d)", prog.Name, len(filtered)), list))
			accordionIndex++
		}
		acc.Refresh()
		if selectListIndex >= 0 && selectList != nil {
			acc.Open(selectAccordionIndex)
			selectList.Select(selectListIndex)
		}
	}
	searchbar.OnChanged = func(string) { searchDebounce.Call(rebuild) }
	rebuild()
	return searchbar, acc
}

type programListEntry struct {
	program *models.Program
	key     string
}

// buildProgramFlatListWithSearchbar builds a single scrollable list of items across all programs.
// Filter matches program name or item key/display name (fuzzy). When initialRef is set, the matching row is selected.
func buildProgramFlatListWithSearchbar(cfg programListAccordionConfig, initialRef actions.CoordinateRef) (*widget.Entry, *widget.List) {
	searchbar := widget.NewEntry()
	searchbar.SetPlaceHolder("Filter programs and entries (fuzzy match)")
	searchDebounce := custom_widgets.NewDebouncer(custom_widgets.DefaultSearchDebounce)

	var entries []programListEntry
	var list *widget.List

	selectInitial := func() {
		if list == nil {
			return
		}
		for i, e := range entries {
			if key, ok := resolveCoordinateRefKey(initialRef, e.program, cfg.GetKeys); ok && key == e.key {
				list.Select(widget.ListItemID(i))
				return
			}
		}
	}

	rebuild := func() {
		filterText := searchbar.Text
		entries = entries[:0]
		for _, p := range repositories.ProgramRepo().GetAllSortedByName() {
			for _, key := range cfg.GetKeys(p) {
				displayName := cfg.GetDisplayName(p, key)
				if filterText != "" &&
					!fuzzy.MatchFold(filterText, p.Name) &&
					!fuzzy.MatchFold(filterText, key) &&
					!fuzzy.MatchFold(filterText, displayName) {
					continue
				}
				entries = append(entries, programListEntry{program: p, key: key})
			}
		}
		sort.Slice(entries, func(i, j int) bool {
			pi, pj := entries[i].program.Name, entries[j].program.Name
			if pi != pj {
				return strings.Compare(pi, pj) < 0
			}
			return strings.Compare(
				cfg.GetDisplayName(entries[i].program, entries[i].key),
				cfg.GetDisplayName(entries[j].program, entries[j].key),
			) < 0
		})
		if list != nil {
			list.UnselectAll()
			custom_widgets.RefreshListPreservingScroll(list)
			selectInitial()
		}
	}

	list = widget.NewList(
		func() int { return len(entries) },
		func() fyne.CanvasObject { return newProgramListRowTemplate(cfg) },
		func(id widget.ListItemID, co fyne.CanvasObject) {
			if id < 0 || id >= len(entries) {
				return
			}
			e := entries[id]
			bindProgramListRow(co, cfg, e.program, e.key, fmt.Sprintf("%s · %s", cfg.GetDisplayName(e.program, e.key), e.program.Name))
		},
	)
	list.OnSelected = func(id widget.ListItemID) {
		if id >= 0 && id < len(entries) {
			e := entries[id]
			cfg.OnSelect(e.program, e.key)
		}
	}

	searchbar.OnChanged = func(string) { searchDebounce.Call(rebuild) }
	rebuild()
	return searchbar, list
}

// buildPointsListWithSearchbar builds a flat points list with a searchbar above it.
// Filter matches program name or point name (fuzzy). onPointSelected is called when user selects a point.
func buildPointsListWithSearchbar(onPointSelected func(actions.CoordinateRef), initialRef actions.CoordinateRef) (*widget.Entry, *widget.List) {
	return buildProgramFlatListWithSearchbar(programListAccordionConfig{
		GetKeys: func(p *models.Program) []string {
			repo := editor.ProgramPointRepo(p, config.MainMonitorSizeString)
			if repo == nil {
				return nil
			}
			return repo.GetAllKeys()
		},
		GetDisplayName: func(p *models.Program, key string) string {
			repo := editor.ProgramPointRepo(p, config.MainMonitorSizeString)
			if repo == nil {
				return key
			}
			pt, _ := repo.Get(key)
			if pt != nil {
				return pt.Name
			}
			return key
		},
		GetPreviewImage: editor.LoadPointPreviewImage,
		OnSelect: func(p *models.Program, key string) {
			onPointSelected(actions.NewCoordinateRef(p.Name, key))
		},
	}, initialRef)
}

// buildSearchAreasAccordionWithSearchbar builds a Search Areas accordion with a single searchbar above it.
// Filter matches program name or search area name (fuzzy). onSelected is called when user selects a search area.
func buildSearchAreasAccordionWithSearchbar(onSelected func(actions.CoordinateRef), initialRef actions.CoordinateRef) (*widget.Entry, *widget.Accordion) {
	return buildProgramListAccordionWithSearchbar(programListAccordionConfig{
		GetKeys: func(p *models.Program) []string {
			repo := editor.ProgramSearchAreaRepo(p, config.MainMonitorSizeString)
			if repo == nil {
				return nil
			}
			return repo.GetAllKeys()
		},
		GetDisplayName: func(p *models.Program, key string) string {
			repo := editor.ProgramSearchAreaRepo(p, config.MainMonitorSizeString)
			if repo == nil {
				return key
			}
			sa, _ := repo.Get(key)
			if sa != nil {
				return sa.Name
			}
			return key
		},
		GetPreviewImage: editor.LoadSearchAreaPreviewImage,
		OnSelect: func(p *models.Program, key string) {
			onSelected(actions.NewCoordinateRef(p.Name, key))
		},
	}, initialRef)
}

// buildItemsAccordionWithSearchbar builds an Items section with a searchbar above and an accordion
// (extending Fyne's) where each program has a tri-state (empty/half/full) on the right of the header row.
// Returns refreshAccordion so the dialog can refresh the accordion when selection changes (e.g. after tri-state or item click).
func buildItemsAccordionWithSearchbar(
	getTargets func() []string,
	onItemSelected func(programName, baseItemName string),
	onSelectionChanged func(newTargets []string),
) (*widget.Entry, fyne.CanvasObject, func()) {
	searchbar := widget.NewEntry()
	searchbar.SetPlaceHolder("Filter programs and items (fuzzy match)")
	searchDebounce := custom_widgets.NewDebouncer(custom_widgets.DefaultSearchDebounce)
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
		itemGrids = itemGrids[:0]
		editor.PopulateItemsSearchAccordion(acc, filterText, func(prog *models.Program) editor.ItemsAccordionOptions {
			return editor.ItemsAccordionOptions{
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
		})
	}
	searchbar.OnChanged = func(string) { searchDebounce.Call(rebuild) }
	rebuild()
	return searchbar, container.NewScroll(acc), refreshAccordion
}

// ShowActionDialog displays a dialog for editing an action.
// When onCancel is set, it runs if the dialog is dismissed without saving (Cancel or Escape).
func ShowActionDialog(action actions.ActionInterface, onSave func(actions.ActionInterface), onCancel func()) {
	if active.Window == nil {
		return
	}
	if active.ClearOpenActionDialog != nil {
		active.ClearOpenActionDialog()
	}
	resetDialogValidation()

	// Create dialog content based on action type
	var content fyne.CanvasObject
	var saveFunc func()

	switch node := action.(type) {
	case *actions.Wait:
		content, saveFunc = createWaitDialogContent(node)
	case *actions.Pause:
		content, saveFunc = createPauseDialogContent(node)
	case *actions.Move:
		content, saveFunc = createMoveDialogContent(node)
	case *actions.Click:
		content, saveFunc = createClickDialogContent(node)
	case *actions.Key:
		content, saveFunc = createKeyDialogContent(node)
	case *actions.Type:
		content, saveFunc = createTypeDialogContent(node)
	case *actions.Loop:
		content, saveFunc = createLoopDialogContent(node)
	case *actions.Conditional:
		content, saveFunc = createConditionalDialogContent(node)
	case *actions.ImageSearch:
		content, saveFunc = createImageSearchDialogContent(node)
	case *actions.Ocr:
		content, saveFunc = createOcrDialogContent(node)
	case *actions.SetVariable:
		content, saveFunc = createSetVariableDialogContent(node)
	case *actions.Calculate:
		content, saveFunc = createCalculateDialogContent(node)
	case *actions.ForEachRow:
		content, saveFunc = createForEachRowDialogContent(node)
	case *actions.SaveVariable:
		content, saveFunc = createSaveVariableDialogContent(node)
	// case *actions.Calibration:
	// 	content, saveFunc = createCalibrationDialogContent(node)
	case *actions.FindPixel:
		content, saveFunc = createFindPixelDialogContent(node)
	case *actions.FocusWindow:
		content, saveFunc = createFocusWindowDialogContent(node)
	case *actions.RunMacro:
		content, saveFunc = createRunMacroDialogContent(node)
	case *actions.Break:
		content, saveFunc = createBreakDialogContent()
	case *actions.Continue:
		content, saveFunc = createContinueDialogContent()
	default:
		unknown := ttwidget.NewLabel("Unknown action type")
		unknown.SetToolTip("This action type is not supported in the editor yet.")
		content = unknown
		saveFunc = func() {}
	}
	// Show custom dialog with save/cancel buttons
	showCustomActionDialog(action, content, saveFunc, onSave, onCancel)
}

// actionModalDialog implements dialog.Dialog using widget.NewModalPopUp so content is not
// inset by fyne dialog.Layout (padWidth/2); the bordered panel can align with the popup edge.
type actionModalDialog struct {
	pop      *widget.PopUp
	onClosed func()
}

func (d *actionModalDialog) Show()                 { d.pop.Show() }
func (d *actionModalDialog) Dismiss()              { d.Hide() }
func (d *actionModalDialog) SetDismissText(string) {}
func (d *actionModalDialog) SetOnClosed(closed func()) {
	if closed == nil {
		return
	}
	orig := d.onClosed
	d.onClosed = func() {
		if orig != nil {
			orig()
		}
		closed()
	}
}
func (d *actionModalDialog) Hide() {
	d.pop.Hide()
	if d.onClosed != nil {
		cb := d.onClosed
		d.onClosed = nil
		cb()
	}
}
func (d *actionModalDialog) Refresh()           { d.pop.Refresh() }
func (d *actionModalDialog) Resize(s fyne.Size) { d.pop.Resize(s) }
func (d *actionModalDialog) MinSize() fyne.Size { return d.pop.MinSize() }

var _ dialog.Dialog = (*actionModalDialog)(nil)

func showCustomActionDialog(action actions.ActionInterface, content fyne.CanvasObject, saveFunc func(), onSave func(actions.ActionInterface), onCancel func()) {
	var d *actionModalDialog
	var saved bool
	submitAction := func() {
		if !allDialogFieldsValid() {
			return
		}
		saveFunc()
		if p, ok := action.(*actions.Pause); ok {
			if err := validatePauseAction(p); err != nil {
				dialog.ShowError(err, active.Window)
				return
			}
		}
		if k, ok := action.(*actions.Key); ok {
			if err := validateKeyAction(k); err != nil {
				dialog.ShowError(err, active.Window)
				return
			}
		}
		if onSave != nil {
			onSave(action)
		}
		saved = true
		d.Hide()
	}
	saveButton := ttwidget.NewButton("Save", submitAction)
	saveButton.SetToolTip("Save changes to this action")

	updateSaveState := func() {
		if allDialogFieldsValid() {
			saveButton.Enable()
		} else {
			saveButton.Disable()
		}
	}
	wireDialogValidation(updateSaveState)
	updateSaveState()

	cancelButton := ttwidget.NewButton("Cancel", func() {
		d.Hide()
	})
	cancelButton.SetToolTip("Cancel and discard changes")

	buttons := container.NewHBox(
		layout.NewSpacer(),
		cancelButton,
		saveButton,
	)

	title := "Edit Action - " + action.GetType()
	borderedDialogContent := buildActionDialogPanel(title, wrapActionDialogContent(action, content), buttons)

	pop := widget.NewModalPopUp(borderedDialogContent, active.Window.Canvas())
	fynetooltip.AddPopUpToolTipLayer(pop)
	custom_widgets.AddPopUpItemTooltipLayer(pop)
	d = &actionModalDialog{pop: pop}
	if active.SetActionDialog != nil {
		active.SetActionDialog(d)
	}
	d.SetOnClosed(func() {
		if !saved && onCancel != nil {
			onCancel()
		}
		if active.ClearActionDialogIfCurrent != nil {
			active.ClearActionDialogIfCurrent(d)
		}
	})
	if active.AddDialogEscapeClose != nil {
		active.AddDialogEscapeClose(d, active.Window)
	}
	if active.AddActionDialogEnterSave != nil {
		active.AddActionDialogEnterSave(d, active.Window, submitAction)
	}
	d.Resize(actionDialogSize(active.Window.Canvas().Size(), action, pop.MinSize()))

	d.Show()
}

// Dialog content creators - these create independent widgets for editing
