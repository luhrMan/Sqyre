package ui

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

const (
	entityPickerScreenMarginFraction float32 = 0.15 // 15% inset per edge → 70% of canvas
)

// programListAccordionConfig configures the generic program list accordion builder.
type programListAccordionConfig struct {
	GetKeys         func(*models.Program) []string
	GetDisplayName  func(*models.Program, string) string
	GetTooltip      func(*models.Program, string) string
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
		custom_widgets.BindPreviewListRow(co, labelText, func() (custom_widgets.PreviewTooltipResult, error) {
			return cfg.GetPreviewImage(prog, key)
		})
		return
	}
	lbl := co.(*ttwidget.Label)
	lbl.SetText(labelText)
	if cfg.GetTooltip != nil {
		lbl.SetToolTip(cfg.GetTooltip(program, key))
	}
}

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

type programDialogRowState struct {
	program  *models.Program
	filtered []string
	list     *widget.List
	item     *widget.AccordionItem
}

type programListAccordionHost struct {
	cfg  programListAccordionConfig
	rows map[string]*programDialogRowState
	acc  *custom_widgets.AccordionWithHeaderWidgets
}

func (host *programListAccordionHost) ensureRow(program *models.Program) *programDialogRowState {
	if row, ok := host.rows[program.Name]; ok {
		row.program = program
		return row
	}
	cfg := host.cfg
	prog := program
	row := &programDialogRowState{program: program}
	row.list = widget.NewList(
		func() int { return len(row.filtered) },
		func() fyne.CanvasObject { return newProgramListRowTemplate(cfg) },
		func(id widget.ListItemID, co fyne.CanvasObject) {
			if id < 0 || id >= len(row.filtered) {
				return
			}
			key := row.filtered[id]
			bindProgramListRow(co, cfg, prog, key, cfg.GetDisplayName(prog, key))
		},
	)
	row.list.OnSelected = func(id widget.ListItemID) {
		if id >= 0 && id < len(row.filtered) {
			cfg.OnSelect(prog, row.filtered[id])
		}
	}
	row.item = widget.NewAccordionItem("", row.list)
	host.rows[program.Name] = row
	return row
}

type programListEntry struct {
	program *models.Program
	key     string
}

func buildProgramListAccordionWithSearchbar(cfg programListAccordionConfig, initialRef actions.CoordinateRef) (*widget.Entry, *custom_widgets.AccordionWithHeaderWidgets) {
	searchbar := custom_widgets.NewFormEntry()
	searchbar.SetPlaceHolder("Filter programs and entries (fuzzy match)")
	searchDebounce := custom_widgets.NewDebouncer(custom_widgets.DefaultSearchDebounce)
	acc := custom_widgets.NewAccordionWithHeaderWidgets()
	host := &programListAccordionHost{cfg: cfg, rows: make(map[string]*programDialogRowState), acc: acc}

	sync := func() {
		filterText := searchbar.Text
		programs := repositories.ProgramRepo().GetAllSortedByName()
		seen := make(map[string]struct{}, len(programs))
		items := make([]*widget.AccordionItem, 0, len(programs))
		var selectAccordionIndex int = -1
		var selectList *widget.List
		var selectListIndex widget.ListItemID = -1
		accordionIndex := 0

		for _, p := range programs {
			seen[p.Name] = struct{}{}
			row := host.ensureRow(p)
			defaultList := cfg.GetKeys(p)
			filtered := slices.Clone(defaultList)
			if filterText != "" {
				filtered = filtered[:0]
				for _, key := range defaultList {
					if fuzzy.MatchFold(filterText, key) {
						filtered = append(filtered, key)
					}
				}
			}
			sort.Slice(filtered, func(i, j int) bool {
				return strings.Compare(cfg.GetDisplayName(p, filtered[i]), cfg.GetDisplayName(p, filtered[j])) < 0
			})
			row.filtered = filtered

			if filterText != "" && !fuzzy.MatchFold(filterText, p.Name) && len(filtered) == 0 {
				continue
			}
			row.item.Title = fmt.Sprintf("%s (%d)", p.Name, len(filtered))
			items = append(items, row.item)

			if selectListIndex < 0 {
				if key, ok := resolveCoordinateRefKey(initialRef, p, cfg.GetKeys); ok {
					if idx := slices.Index(filtered, key); idx >= 0 {
						selectAccordionIndex = accordionIndex
						selectList = row.list
						selectListIndex = widget.ListItemID(idx)
					}
				}
			}
			accordionIndex++
			row.list.UnselectAll()
			custom_widgets.RefreshListPreservingScroll(row.list)
		}

		for name := range host.rows {
			if _, ok := seen[name]; !ok {
				delete(host.rows, name)
			}
		}

		headers := make([]fyne.CanvasObject, len(items))
		acc.SetItems(items, headers)

		if selectListIndex >= 0 && selectList != nil {
			acc.Open(selectAccordionIndex)
			selectList.ScrollTo(selectListIndex)
		}
	}
	searchbar.OnChanged = func(string) { searchDebounce.Call(sync) }
	sync()
	return searchbar, acc
}

func buildProgramFlatListWithSearchbar(cfg programListAccordionConfig, initialRef actions.CoordinateRef) (*widget.Entry, *widget.List) {
	searchbar := custom_widgets.NewFormEntry()
	searchbar.SetPlaceHolder("Filter programs and entries (fuzzy match)")
	searchDebounce := custom_widgets.NewDebouncer(custom_widgets.DefaultSearchDebounce)

	var entries []programListEntry
	var list *widget.List

	scrollToInitial := func() {
		if list == nil {
			return
		}
		for i, e := range entries {
			if key, ok := resolveCoordinateRefKey(initialRef, e.program, cfg.GetKeys); ok && key == e.key {
				list.ScrollTo(widget.ListItemID(i))
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
			scrollToInitial()
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

func buildPointsListWithSearchbar(onStage func(actions.CoordinateRef), initialRef actions.CoordinateRef) (*widget.Entry, *widget.List) {
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
			onStage(actions.NewCoordinateRef(p.Name, key))
		},
	}, initialRef)
}

func buildSearchAreasAccordionWithSearchbar(onStage func(actions.CoordinateRef), initialRef actions.CoordinateRef) (*widget.Entry, *custom_widgets.AccordionWithHeaderWidgets) {
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
			onStage(actions.NewCoordinateRef(p.Name, key))
		},
	}, initialRef)
}

func buildItemsAccordionWithSearchbar(
	getTargets func() []string,
	onItemSelected func(programName, baseItemName string),
	onSelectionChanged func(programName string, newTargets []string),
) (*widget.Entry, fyne.CanvasObject) {
	searchbar := custom_widgets.NewFormEntry()
	searchbar.SetPlaceHolder("Filter programs and items (fuzzy match)")
	searchDebounce := custom_widgets.NewDebouncer(custom_widgets.DefaultSearchDebounce)
	acc := custom_widgets.NewAccordionWithHeaderWidgets()
	gridsByProgram := make(map[string]*widget.GridWrap)
	refreshProgram := func(programName string) {
		if g := gridsByProgram[programName]; g != nil {
			custom_widgets.RefreshGridWrapPreservingScroll(g)
		}
		acc.RefreshHeaderWidgets()
	}
	rebuild := func() {
		filterText := searchbar.Text
		clear(gridsByProgram)
		editor.PopulateItemsSearchAccordion(acc, filterText, func(prog *models.Program) editor.ItemsAccordionOptions {
			programName := prog.Name
			return editor.ItemsAccordionOptions{
				Program:            prog,
				FilterText:         filterText,
				GetSelectedTargets: getTargets,
				OnItemSelected: func(baseItemName string) {
					onItemSelected(programName, baseItemName)
				},
				OnSelectionChanged: func(newTargets []string) {
					onSelectionChanged(programName, newTargets)
					acc.RefreshHeaderWidgets()
				},
				OnSelectionMaybeChanged: func() { refreshProgram(programName) },
				AllButtonInHeader:       true,
				RegisterRefreshTarget: func(grid *widget.GridWrap) {
					gridsByProgram[programName] = grid
				},
			}
		})
	}
	searchbar.OnChanged = func(string) { searchDebounce.Call(rebuild) }
	rebuild()
	return searchbar, container.NewScroll(acc)
}

func entityPickerSize(canvas fyne.Canvas) fyne.Size {
	if canvas == nil {
		return fyne.NewSize(560, 640)
	}
	s := canvas.Size()
	scale := 1 - 2*entityPickerScreenMarginFraction
	return fyne.NewSize(s.Width*scale, s.Height*scale)
}

func deferHidePickerDialog(dlg dialog.Dialog) {
	if dlg == nil {
		return
	}
	fyne.Do(func() {
		dlg.Hide()
	})
}

func wrapEntityPickerPopupContent(inner fyne.CanvasObject) fyne.CanvasObject {
	return WrapSqyreFrame(container.NewPadded(inner))
}

func showEntityPickerModal(parent fyne.Window, title string, body fyne.CanvasObject, onSave func(), onClosed func()) dialog.Dialog {
	if parent == nil {
		return nil
	}
	header := widget.NewLabelWithStyle(title, fyne.TextAlignLeading, fyne.TextStyle{Bold: true})
	saveBtn := widget.NewButtonWithIcon("", theme.DocumentSaveIcon(), nil)
	cancelBtn := widget.NewButtonWithIcon("", theme.CancelIcon(), nil)
	saveBtn.Importance = widget.HighImportance
	bottom := container.NewHBox(layout.NewSpacer(), saveBtn, cancelBtn)
	content := container.NewBorder(header, bottom, nil, nil, body)
	pop := widget.NewModalPopUp(wrapEntityPickerPopupContent(content), parent.Canvas())
	fynetooltip.AddPopUpToolTipLayer(pop)
	custom_widgets.AddPopUpItemTooltipLayer(pop)
	dlg := AddPopupEscapeClose(pop, parent)
	if onClosed != nil {
		dlg.SetOnClosed(onClosed)
	}
	saveBtn.OnTapped = func() {
		if onSave != nil {
			onSave()
		}
		deferHidePickerDialog(dlg)
	}
	cancelBtn.OnTapped = func() { deferHidePickerDialog(dlg) }
	dlg.Resize(entityPickerSize(parent.Canvas()))
	dlg.Show()
	return dlg
}

// ShowPointPicker opens a searchable modal to pick a point.
func ShowPointPicker(parent fyne.Window, initial actions.CoordinateRef, onSelect func(actions.CoordinateRef), onClosed func()) {
	if parent == nil || onSelect == nil {
		return
	}
	staged := initial
	searchbar, list := buildPointsListWithSearchbar(func(ref actions.CoordinateRef) { staged = ref }, initial)
	body := container.NewBorder(searchbar, nil, nil, nil, list)
	showEntityPickerModal(parent, "Select Point", body, func() {
		onSelect(staged)
	}, onClosed)
}

// ShowSearchAreaPicker opens a searchable modal to pick a search area.
func ShowSearchAreaPicker(parent fyne.Window, initial actions.CoordinateRef, onSelect func(actions.CoordinateRef), onClosed func()) {
	if parent == nil || onSelect == nil {
		return
	}
	staged := initial
	searchbar, acc := buildSearchAreasAccordionWithSearchbar(func(ref actions.CoordinateRef) { staged = ref }, initial)
	body := container.NewBorder(searchbar, nil, nil, nil, acc)
	showEntityPickerModal(parent, "Select Search Area", body, func() {
		onSelect(staged)
	}, onClosed)
}

// ShowItemsPicker opens a searchable modal to toggle image-search target items.
func ShowItemsPicker(parent fyne.Window, getTargets func() []string, onChanged func(newTargets []string), onClosed func()) {
	if parent == nil || getTargets == nil || onChanged == nil {
		return
	}
	staged := slices.Clone(getTargets())
	searchbar, accScroll := buildItemsAccordionWithSearchbar(
		func() []string { return staged },
		func(programName, baseItemName string) {
			name := programName + config.ProgramDelimiter + baseItemName
			if i := slices.Index(staged, name); i != -1 {
				staged = slices.Delete(staged, i, i+1)
			} else {
				staged = append(staged, name)
			}
			slices.Sort(staged)
		},
		func(_ string, newTargets []string) {
			staged = slices.Clone(newTargets)
		},
	)
	body := container.NewBorder(searchbar, nil, nil, nil, accScroll)
	showEntityPickerModal(parent, "Select Items", body, func() {
		current := getTargets()
		if slices.Equal(staged, current) {
			return
		}
		onChanged(slices.Clone(staged))
	}, onClosed)
}
