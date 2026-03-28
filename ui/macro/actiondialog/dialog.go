package actiondialog

import (
	"Sqyre/internal/config"
	"Sqyre/internal/models"
	"Sqyre/internal/models/actions"
	"Sqyre/internal/models/repositories"
	"Sqyre/ui/custom_widgets"
	"Sqyre/ui/editor"
	"fmt"
	"image/color"
	"sort"
	"strings"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/canvas"
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
	case "Name:", "Variable Name:":
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

// newVarEntry creates a VarEntry wired to the current macro's variables.
func newVarEntry() *custom_widgets.VarEntry {
	return custom_widgets.NewVarEntry(macroVarNames)
}

// newMultiLineVarEntry creates a multi-line VarEntry wired to the current macro's variables.
func newMultiLineVarEntry() *custom_widgets.VarEntry {
	return custom_widgets.NewMultiLineVarEntry(macroVarNames)
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

// programListAccordionConfig configures the generic program list accordion builder.
// Callbacks receive the program and item key; implementors look up the model and invoke dialog-specific logic.
type programListAccordionConfig struct {
	GetKeys        func(*models.Program) []string
	GetDisplayName func(*models.Program, string) string
	// GetTooltip is optional; when set, each list row shows this text as a hover tooltip (e.g. coordinates).
	GetTooltip func(*models.Program, string) string
	OnSelect   func(*models.Program, string)
}

// buildProgramListAccordionWithSearchbar builds an accordion of programs, each with a list of items (e.g. points or search areas).
// One searchbar above filters by program name or item key (fuzzy). Config provides key source, label text, and selection callback.
func buildProgramListAccordionWithSearchbar(cfg programListAccordionConfig) (*widget.Entry, *widget.Accordion) {
	searchbar := widget.NewEntry()
	searchbar.SetPlaceHolder("Filter programs and entries (fuzzy match)")
	acc := widget.NewAccordion()
	rebuild := func() {
		filterText := searchbar.Text
		acc.Items = nil
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
				func() fyne.CanvasObject { return ttwidget.NewLabel("template") },
				func(id widget.ListItemID, co fyne.CanvasObject) {
					key := filtered[id]
					lbl := co.(*ttwidget.Label)
					lbl.SetText(cfg.GetDisplayName(p, key))
					if cfg.GetTooltip != nil {
						lbl.SetToolTip(cfg.GetTooltip(p, key))
					}
				},
			)
			prog := p
			list.OnSelected = func(id widget.ListItemID) {
				if id >= 0 && id < len(filtered) {
					cfg.OnSelect(prog, filtered[id])
				}
			}
			acc.Append(widget.NewAccordionItem(fmt.Sprintf("%s (%d)", prog.Name, len(filtered)), list))
		}
		acc.Refresh()
	}
	searchbar.OnChanged = func(string) { rebuild() }
	rebuild()
	return searchbar, acc
}

// buildPointsAccordionWithSearchbar builds a Points accordion with a single searchbar above it.
// Filter matches program name or point name (fuzzy). onPointSelected is called when user selects a point.
func buildPointsAccordionWithSearchbar(onPointSelected func(actions.Point)) (*widget.Entry, *widget.Accordion) {
	return buildProgramListAccordionWithSearchbar(programListAccordionConfig{
		GetKeys: func(p *models.Program) []string {
			return p.PointRepo(config.MainMonitorSizeString).GetAllKeys()
		},
		GetDisplayName: func(p *models.Program, key string) string {
			pt, _ := p.PointRepo(config.MainMonitorSizeString).Get(key)
			if pt != nil {
				return pt.Name
			}
			return key
		},
		GetTooltip: func(p *models.Program, key string) string {
			pt, _ := p.PointRepo(config.MainMonitorSizeString).Get(key)
			if pt == nil {
				return ""
			}
			return fmt.Sprintf("X: %v, Y: %v", pt.X, pt.Y)
		},
		OnSelect: func(p *models.Program, key string) {
			pt, _ := p.PointRepo(config.MainMonitorSizeString).Get(key)
			if pt != nil {
				onPointSelected(actions.Point{Name: pt.Name, X: pt.X, Y: pt.Y})
			}
		},
	})
}

// buildSearchAreasAccordionWithSearchbar builds a Search Areas accordion with a single searchbar above it.
// Filter matches program name or search area name (fuzzy). onSelected is called when user selects a search area.
func buildSearchAreasAccordionWithSearchbar(onSelected func(actions.SearchArea)) (*widget.Entry, *widget.Accordion) {
	return buildProgramListAccordionWithSearchbar(programListAccordionConfig{
		GetKeys: func(p *models.Program) []string {
			return p.SearchAreaRepo(config.MainMonitorSizeString).GetAllKeys()
		},
		GetDisplayName: func(p *models.Program, key string) string {
			sa, _ := p.SearchAreaRepo(config.MainMonitorSizeString).Get(key)
			if sa != nil {
				return sa.Name
			}
			return key
		},
		GetTooltip: func(p *models.Program, key string) string {
			sa, _ := p.SearchAreaRepo(config.MainMonitorSizeString).Get(key)
			if sa == nil {
				return ""
			}
			return fmt.Sprintf("Left: %v, Top: %v, Right: %v, Bottom: %v", sa.LeftX, sa.TopY, sa.RightX, sa.BottomY)
		},
		OnSelect: func(p *models.Program, key string) {
			sa, _ := p.SearchAreaRepo(config.MainMonitorSizeString).Get(key)
			if sa != nil {
				onSelected(actions.SearchArea{
					Name:    sa.Name,
					LeftX:   sa.LeftX,
					TopY:    sa.TopY,
					RightX:  sa.RightX,
					BottomY: sa.BottomY,
				})
			}
		},
	})
}

// buildItemsAccordionWithSearchbar builds an Items section with a searchbar above and an accordion
// (extending Fyne's) where each program has a tri-state (empty/half/full) on the right of the header row.
// Returns refreshAccordion so the dialog can refresh the accordion when selection changes (e.g. after tri-state or item click).
func buildItemsAccordionWithSearchbar(
	getTargets func() []string,
	onItemSelected func(programName, baseItemName string),
	onSelectionChanged func(newTargets []string),
	refreshPreview func(),
) (*widget.Entry, fyne.CanvasObject, func()) {
	searchbar := widget.NewEntry()
	searchbar.SetPlaceHolder("Filter programs and items (fuzzy match)")
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
	searchbar.OnChanged = func(string) { rebuild() }
	rebuild()
	return searchbar, container.NewScroll(acc), refreshAccordion
}

// ShowActionDialog displays a dialog for editing an action.
func ShowActionDialog(action actions.ActionInterface, onSave func(actions.ActionInterface)) {
	if active.Window == nil {
		return
	}
	if active.ClearOpenActionDialog != nil {
		active.ClearOpenActionDialog()
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
	case *actions.Type:
		content, saveFunc = createTypeDialogContent(node)
		content.Resize(fyne.NewSize(400, 120))
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
	// case *actions.Calibration:
	// 	content, saveFunc = createCalibrationDialogContent(node)
	// 	content.Resize(fyne.NewSize(600, 500))
	case *actions.FindPixel:
		content, saveFunc = createFindPixelDialogContent(node)
		content.Resize(fyne.NewSize(800, 500))
	case *actions.FocusWindow:
		content, saveFunc = createFocusWindowDialogContent(node)
		content.Resize(fyne.NewSize(500, 400))
	case *actions.RunMacro:
		content, saveFunc = createRunMacroDialogContent(node)
		content.Resize(fyne.NewSize(400, 120))
	default:
		unknown := ttwidget.NewLabel("Unknown action type")
		unknown.SetToolTip("This action type is not supported in the editor yet.")
		content = unknown
		saveFunc = func() {}
	}
	// Show custom dialog with save/cancel buttons
	showCustomActionDialog(action, content, saveFunc, onSave)
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

func showCustomActionDialog(action actions.ActionInterface, content fyne.CanvasObject, saveFunc func(), onSave func(actions.ActionInterface)) {
	var d *actionModalDialog
	saveButton := ttwidget.NewButton("Save", func() {
		saveFunc()
		if onSave != nil {
			onSave(action)
		}
		d.Hide()
	})
	saveButton.SetToolTip("Save changes to this action")

	cancelButton := ttwidget.NewButton("Cancel", func() {
		d.Hide()
	})
	cancelButton.SetToolTip("Cancel and discard changes")

	buttons := container.NewHBox(
		layout.NewSpacer(),
		cancelButton,
		saveButton,
	)

	titleLabel := ttwidget.NewLabel("Edit Action - " + action.GetType())
	titleLabel.TextStyle = fyne.TextStyle{Bold: true}
	titleLabel.SetToolTip("Edit fields for this action type, then Save to apply or Cancel to discard.")

	dialogContent := container.NewBorder(
		container.NewPadded(titleLabel),
		buttons,
		nil,
		nil,
		content,
	)

	th := fyne.CurrentApp().Settings().Theme()
	v := fyne.CurrentApp().Settings().ThemeVariant()
	panelBg := canvas.NewRectangle(th.Color(theme.ColorNameOverlayBackground, v))
	panelBg.CornerRadius = theme.InputRadiusSize()

	border := canvas.NewRectangle(color.Transparent)
	border.StrokeColor = th.Color(theme.ColorNamePrimary, v)
	border.StrokeWidth = 1
	border.CornerRadius = theme.InputRadiusSize()
	innerPadded := container.NewPadded(container.NewPadded(container.NewPadded(container.NewPadded(dialogContent))))
	borderedDialogContent := container.NewStack(panelBg, border, innerPadded)

	pop := widget.NewModalPopUp(borderedDialogContent, active.Window.Canvas())
	fynetooltip.AddPopUpToolTipLayer(pop)
	d = &actionModalDialog{pop: pop}
	if active.SetActionDialog != nil {
		active.SetActionDialog(d)
	}
	d.SetOnClosed(func() {
		if active.ClearActionDialogIfCurrent != nil {
			active.ClearActionDialogIfCurrent(d)
		}
	})
	if active.AddDialogEscapeClose != nil {
		active.AddDialogEscapeClose(d, active.Window)
	}
	parentSize := active.Window.Canvas().Size()
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
