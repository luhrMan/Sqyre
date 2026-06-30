package editor

import (
	"Sqyre/internal/models"
	"Sqyre/internal/models/repositories"
	"fmt"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/widget"
)

type entityAccordionConfig struct {
	tab              *EditorTab
	filterText       *string
	getKeys          func(*models.Program) []string
	sortKeys         func(*models.Program, []string)
	getEntity        func(*models.Program, string) (string, error)
	onSelected       func(*models.Program, string)
	extraOnSelected  func() // optional hook after onSelected (e.g. macro sync)
}

func populateProgramEntityAccordion(acc *widget.Accordion, cfg entityAccordionConfig) {
	openState := captureAccordionOpenByProgram(acc.Items)
	acc.Items = []*widget.AccordionItem{}
	filterText := ""
	if sb, ok := cfg.tab.Widgets["searchbar"].(*widget.Entry); ok {
		filterText = sb.Text
		sb.OnChanged = func(string) { populateProgramEntityAccordion(acc, cfg) }
	}
	if cfg.filterText != nil {
		*cfg.filterText = filterText
	}

	for _, p := range repositories.ProgramRepo().GetAllSortedByName() {
		program := p
		defaultList := cfg.getKeys(program)
		filtered := filterKeysByFuzzy(filterText, defaultList)
		cfg.sortKeys(program, filtered)
		if skipProgramAccordionRow(filterText, program.Name, filtered) {
			continue
		}

		state := struct {
			list     *widget.List
			filtered []string
		}{filtered: filtered}

		state.list = widget.NewList(
			func() int { return len(state.filtered) },
			func() fyne.CanvasObject { return widget.NewLabel("template") },
			func(id widget.ListItemID, co fyne.CanvasObject) {
				if id >= len(state.filtered) {
					return
				}
				name, err := cfg.getEntity(program, state.filtered[id])
				if err != nil {
					return
				}
				co.(*widget.Label).SetText(name)
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
		acc.Append(item)
	}
	applyAccordionOpenByProgram(acc.Items, openState)
	acc.Refresh()
}
