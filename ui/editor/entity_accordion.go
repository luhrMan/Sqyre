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
	tab              *EditorTab
	filterText       *string
	getKeys          func(*models.Program) []string
	sortKeys         func(*models.Program, []string)
	getEntity        func(*models.Program, string) (string, error)
	getPreviewImage  func(*models.Program, string) (custom_widgets.PreviewTooltipResult, error)
	onSelected       func(*models.Program, string)
	extraOnSelected  func() // optional hook after onSelected (e.g. macro sync)
}

func populateProgramEntityAccordion(acc *widget.Accordion, cfg entityAccordionConfig) {
	openState := captureAccordionOpenByProgram(acc.Items)
	acc.Items = []*widget.AccordionItem{}
	filterText := ""
	if sb, ok := cfg.tab.Widgets["searchbar"].(*widget.Entry); ok {
		filterText = sb.Text
		sb.OnChanged = func(string) {
			cfg.tab.SearchDebouncer().Call(func() { populateProgramEntityAccordion(acc, cfg) })
		}
	}
	if cfg.filterText != nil {
		*cfg.filterText = filterText
	}

	for _, program := range repositories.ProgramRepo().GetAllSortedByName() {
		if item, ok := buildProgramEntityAccordionRow(cfg, program, filterText); ok {
			acc.Append(item)
		}
	}
	applyAccordionOpenByProgram(acc.Items, openState)
	acc.Refresh()
}

// buildProgramEntityAccordionRow builds the accordion row (list + title) for one
// program, registering its list widget under "<program>-list". The boolean is
// false when the row should be hidden for the current filter.
func buildProgramEntityAccordionRow(cfg entityAccordionConfig, program *models.Program, filterText string) (*widget.AccordionItem, bool) {
	defaultList := cfg.getKeys(program)
	filtered := filterKeysByFuzzy(filterText, defaultList)
	cfg.sortKeys(program, filtered)
	if skipProgramAccordionRow(filterText, program.Name, filtered) {
		return nil, false
	}

	state := struct {
		list     *widget.List
		filtered []string
	}{filtered: filtered}

	state.list = widget.NewList(
		func() int { return len(state.filtered) },
		func() fyne.CanvasObject {
			if cfg.getPreviewImage == nil {
				return widget.NewLabel("template")
			}
			return custom_widgets.PreviewListRowTemplate()
		},
		func(id widget.ListItemID, co fyne.CanvasObject) {
			if id >= len(state.filtered) {
				return
			}
			key := state.filtered[id]
			name, err := cfg.getEntity(program, key)
			if err != nil {
				return
			}
			if cfg.getPreviewImage == nil {
				co.(*widget.Label).SetText(name)
				return
			}
			prog := program
			custom_widgets.BindPreviewListRow(co, name, func() (custom_widgets.PreviewTooltipResult, error) {
				return cfg.getPreviewImage(prog, key)
			})
		},
	)

	state.list.OnSelected = func(id widget.ListItemID) {
		if id < 0 || id >= len(state.filtered) {
			return
		}
		pro, err := repositories.ProgramRepo().Get(program.Name)
		if err != nil {
			editorRepoLog("load", "program", program.Name, err)
			return
		}
		if cfg.tab.ProgramSelector != nil {
			cfg.tab.ProgramSelector.SetSelected(pro.Name)
		}
		cfg.onSelected(pro, state.filtered[id])
		if cfg.extraOnSelected != nil {
			cfg.extraOnSelected()
		}
	}

	item := widget.NewAccordionItem(
		fmt.Sprintf("%s (%d)", program.Name, len(filtered)),
		state.list,
	)
	cfg.tab.Widgets[program.Name+"-list"] = state.list
	return item, true
}

// entityAccordionRowIndex returns the index of the accordion row for programName,
// or -1 if absent. Titles are "ProgramName (n)".
func entityAccordionRowIndex(acc *widget.Accordion, programName string) int {
	prefix := programName + " ("
	for i, item := range acc.Items {
		if item != nil && strings.HasPrefix(item.Title, prefix) {
			return i
		}
	}
	return -1
}

// refreshProgramEntityAccordionRow rebuilds only the row for programName instead
// of rebuilding every program's row. It recomputes that program's entity list
// from the repo, preserving the row's open state. When the program has no row yet
// (e.g. a freshly created program) it falls back to a full populate.
func refreshProgramEntityAccordionRow(acc *widget.Accordion, cfg entityAccordionConfig, programName string) {
	if acc == nil {
		return
	}
	idx := entityAccordionRowIndex(acc, programName)
	if idx < 0 {
		populateProgramEntityAccordion(acc, cfg)
		return
	}

	filterText := ""
	if sb, ok := cfg.tab.Widgets["searchbar"].(*widget.Entry); ok {
		filterText = sb.Text
	}
	program, err := repositories.ProgramRepo().Get(programName)
	if err != nil {
		populateProgramEntityAccordion(acc, cfg)
		return
	}

	wasOpen := acc.Items[idx].Open
	item, visible := buildProgramEntityAccordionRow(cfg, program, filterText)
	if !visible {
		acc.Items = append(acc.Items[:idx], acc.Items[idx+1:]...)
		acc.Refresh()
		return
	}
	item.Open = wasOpen
	acc.Items[idx] = item
	acc.Refresh()
}
