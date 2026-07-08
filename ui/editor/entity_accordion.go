package editor

import (
	"Sqyre/internal/models"
	"Sqyre/internal/models/repositories"
	"Sqyre/ui/custom_widgets"
	"fmt"
	"strings"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/widget"
)

type entityAccordionConfig struct {
	tab             *EditorTab
	filterText      *string
	getKeys         func(*models.Program) []string
	sortKeys        func(*models.Program, []string)
	getEntity       func(*models.Program, string) (string, error)
	getPreviewImage func(*models.Program, string) (custom_widgets.PreviewTooltipResult, error)
	onSelected      func(*models.Program, string)
	extraOnSelected func() // optional hook after onSelected (e.g. macro sync)
}

type programEntityRowState struct {
	programName string
	program     *models.Program
	filtered    []string
	list        *widget.List
	item        *widget.AccordionItem
}

type entityAccordionState struct {
	rows map[string]*programEntityRowState
}

func populateProgramEntityAccordion(acc *custom_widgets.AccordionWithHeaderWidgets, cfg entityAccordionConfig) {
	syncProgramEntityAccordion(acc, cfg)
}

func syncProgramEntityAccordion(acc *custom_widgets.AccordionWithHeaderWidgets, cfg entityAccordionConfig) {
	tab := cfg.tab
	if tab.entityAccordionState == nil {
		tab.entityAccordionState = &entityAccordionState{rows: make(map[string]*programEntityRowState)}
		if sb, ok := tab.Widgets["searchbar"].(*widget.Entry); ok {
			sb.OnChanged = func(string) {
				tab.SearchDebouncer().Call(func() { syncProgramEntityAccordion(acc, cfg) })
			}
		}
	}
	st := tab.entityAccordionState

	filterText := entityAccordionFilterText(tab)
	if cfg.filterText != nil {
		*cfg.filterText = filterText
	}

	openState := captureAccordionOpenByProgram(acc.Items)
	programs := repositories.ProgramRepo().GetAllSortedByName()
	seen := make(map[string]struct{}, len(programs))

	items := make([]*widget.AccordionItem, 0, len(programs))
	for _, program := range programs {
		seen[program.Name] = struct{}{}
		row := st.ensureRow(program, cfg)
		defaultList := cfg.getKeys(program)
		row.filtered = filterKeysByFuzzy(filterText, defaultList)
		cfg.sortKeys(program, row.filtered)

		if skipProgramAccordionRow(filterText, program.Name, row.filtered) {
			continue
		}
		row.item.Title = fmt.Sprintf("%s (%d)", program.Name, len(row.filtered))
		items = append(items, row.item)
		tab.Widgets[program.Name+"-list"] = row.list
		custom_widgets.RefreshListPreservingScroll(row.list)
	}

	for name := range st.rows {
		if _, ok := seen[name]; !ok {
			delete(st.rows, name)
		}
	}

	headers := make([]fyne.CanvasObject, len(items))
	applyAccordionOpenByProgram(items, openState)
	acc.SetItems(items, headers)
}

func entityAccordionFilterText(tab *EditorTab) string {
	if sb, ok := tab.Widgets["searchbar"].(*widget.Entry); ok {
		return sb.Text
	}
	return ""
}

func (st *entityAccordionState) ensureRow(program *models.Program, cfg entityAccordionConfig) *programEntityRowState {
	if row, ok := st.rows[program.Name]; ok {
		row.program = program
		return row
	}
	row := &programEntityRowState{
		programName: program.Name,
		program:     program,
	}
	prog := program
	row.list = widget.NewList(
		func() int { return len(row.filtered) },
		func() fyne.CanvasObject {
			if cfg.getPreviewImage == nil {
				return widget.NewLabel("template")
			}
			return custom_widgets.PreviewListRowTemplate()
		},
		func(id widget.ListItemID, co fyne.CanvasObject) {
			if id >= len(row.filtered) {
				return
			}
			key := row.filtered[id]
			name, err := cfg.getEntity(prog, key)
			if err != nil {
				return
			}
			if cfg.getPreviewImage == nil {
				co.(*widget.Label).SetText(name)
				return
			}
			custom_widgets.BindPreviewListRow(co, name, func() (custom_widgets.PreviewTooltipResult, error) {
				return cfg.getPreviewImage(prog, key)
			})
		},
	)
	row.list.OnSelected = func(id widget.ListItemID) {
		if id < 0 || id >= len(row.filtered) {
			return
		}
		pro, err := repositories.ProgramRepo().Get(prog.Name)
		if err != nil {
			editorRepoLog("load", "program", prog.Name, err)
			return
		}
		if cfg.tab.ProgramSelector != nil {
			cfg.tab.ProgramSelector.SetSelected(pro.Name)
		}
		cfg.onSelected(pro, row.filtered[id])
		if cfg.extraOnSelected != nil {
			cfg.extraOnSelected()
		}
	}
	row.item = widget.NewAccordionItem("", row.list)
	st.rows[program.Name] = row
	return row
}

// entityAccordionRowIndex returns the index of the accordion row for programName,
// or -1 if absent. Titles are "ProgramName (n)".
func entityAccordionRowIndex(acc *custom_widgets.AccordionWithHeaderWidgets, programName string) int {
	prefix := programName + " ("
	for i, item := range acc.Items {
		if item != nil && strings.HasPrefix(item.Title, prefix) {
			return i
		}
	}
	return -1
}

// refreshProgramEntityAccordionRow updates only the row for programName instead
// of rebuilding every program's row. When the program has no row yet (e.g. freshly
// created) it falls back to a full sync.
func refreshProgramEntityAccordionRow(acc *custom_widgets.AccordionWithHeaderWidgets, cfg entityAccordionConfig, programName string) {
	if acc == nil {
		return
	}
	tab := cfg.tab
	if tab.entityAccordionState == nil {
		syncProgramEntityAccordion(acc, cfg)
		return
	}

	program, err := repositories.ProgramRepo().Get(programName)
	if err != nil {
		syncProgramEntityAccordion(acc, cfg)
		return
	}

	filterText := entityAccordionFilterText(tab)
	st := tab.entityAccordionState
	row := st.ensureRow(program, cfg)
	row.filtered = filterKeysByFuzzy(filterText, cfg.getKeys(program))
	cfg.sortKeys(program, row.filtered)

	visible := !skipProgramAccordionRow(filterText, program.Name, row.filtered)
	idx := entityAccordionRowIndex(acc, programName)

	if !visible {
		if idx >= 0 {
			items := append(acc.Items[:idx], acc.Items[idx+1:]...)
			acc.SetItems(items, nil)
		}
		return
	}

	row.item.Title = fmt.Sprintf("%s (%d)", program.Name, len(row.filtered))
	tab.Widgets[programName+"-list"] = row.list
	custom_widgets.RefreshListPreservingScroll(row.list)

	if idx < 0 {
		items := append(acc.Items, row.item)
		acc.SetItems(items, nil)
		return
	}

	wasOpen := acc.Items[idx].Open
	acc.Items[idx] = row.item
	row.item.Open = wasOpen
	acc.Items[idx].Title = row.item.Title
	acc.Refresh()
}
