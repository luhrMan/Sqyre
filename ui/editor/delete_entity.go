package editor

import (
	"Sqyre/internal/config"
	"Sqyre/internal/models"
	"Sqyre/internal/models/repositories"
	"Sqyre/internal/vision"
	"os"
	"path/filepath"

	"Sqyre/ui/custom_widgets"

	"fyne.io/fyne/v2/widget"
)

type deleteEntityConfig struct {
	entityType string
	name       string
	delete     func(*models.Program) error
	reset      func(*models.Program)
	refresh    func()
	cleanup    func() // optional filesystem cleanup
}

func deleteEntityForTab(cfg deleteEntityConfig, program *models.Program) {
	if err := cfg.delete(program); err != nil {
		editorRepoErr("delete", cfg.name, err)
		return
	}
	if cfg.cleanup != nil {
		cfg.cleanup()
	}
	if cfg.reset != nil {
		cfg.reset(program)
	}
	if cfg.refresh != nil {
		cfg.refresh()
	}
	shell().RefreshEditorActionBar()
}

func performDeleteForTab() {
	programName := shell().ActiveProgramName()
	et := shell().EditorTabs

	switch shell().EditorTabs.Selected().Text {
	case "Programs":
		if v, ok := et.ProgramsTab.SelectedItem.(*models.Program); ok && v.Name != "" {
			deleteEntityForTab(deleteEntityConfig{
				entityType: "program",
				name:       v.Name,
				delete: func(*models.Program) error {
					return repositories.ProgramRepo().Delete(v.Name)
				},
				reset: func(*models.Program) {
					et.ProgramsTab.SelectedItem = repositories.ProgramRepo().New()
					if list, ok := et.ProgramsTab.Widgets["list"].(*widget.List); ok {
						list.UnselectAll()
					}
				},
				refresh: func() {
					refreshAllProgramRelatedUI()
					updateProgramSelectorOptions()
				},
			}, nil)
		}
	case "Items":
		if v, ok := et.ItemsTab.SelectedItem.(*models.Item); ok && v.Name != "" {
			program, ok := getProgramForEditor(programName)
			if !ok {
				return
			}
			deleteEntityForTab(deleteEntityConfig{
				entityType: "item",
				name:       v.Name,
				delete: func(p *models.Program) error {
					return ProgramItemRepo(p).Delete(v.Name)
				},
				reset: func(p *models.Program) {
					et.ItemsTab.SelectedItem = ProgramItemRepo(p).New()
					if list, ok := et.ItemsTab.Widgets[programName+"-list"].(*widget.GridWrap); ok {
						list.UnselectAll()
					}
				},
				refresh: func() {
					if acc, ok := et.ItemsTab.Widgets["Accordion"].(*custom_widgets.AccordionWithHeaderWidgets); ok {
						syncItemsAccordionProgramRow(acc, programName)
					}
					InvalidateProgramTagsCache(programName)
				},
			}, program)
		}
	case "Points":
		if v, ok := et.PointsTab.SelectedItem.(*models.Point); ok && v.Name != "" {
			program, ok := getProgramForEditor(programName)
			if !ok {
				return
			}
			deleteEntityForTab(deleteEntityConfig{
				entityType: "point",
				name:       v.Name,
				delete: func(p *models.Program) error {
					return ProgramPointRepo(p, config.MainMonitorSizeString).Delete(v.Name)
				},
				reset: func(p *models.Program) {
					et.PointsTab.SelectedItem = ProgramPointRepo(p, config.MainMonitorSizeString).New()
					if list, ok := et.PointsTab.Widgets[programName+"-list"].(*widget.List); ok {
						list.UnselectAll()
					}
				},
				refresh: func() {
					vision.InvalidatePreviewTooltipCacheEntity(v.Name)
					refreshPointsAccordionForProgram(programName)
				},
			}, program)
		}
	case "Masks":
		if v, ok := et.MasksTab.SelectedItem.(*models.Mask); ok && v.Name != "" {
			program, ok := getProgramForEditor(programName)
			if !ok {
				return
			}
			deleteEntityForTab(deleteEntityConfig{
				entityType: "mask",
				name:       v.Name,
				delete: func(p *models.Program) error {
					return ProgramMaskRepo(p).Delete(v.Name)
				},
				cleanup: func() {
					imgPath := filepath.Join(config.GetMasksPath(), programName, v.Name+config.PNG)
					if err := os.Remove(imgPath); err != nil && !os.IsNotExist(err) {
						editorRepoLog("remove file", "mask image", imgPath, err)
					}
					shell().SetMaskImageMode(false)
					shell().ClearMaskPreviewImage()
				},
				reset: func(*models.Program) {
					et.MasksTab.SelectedItem = &models.Mask{}
					if list, ok := et.MasksTab.Widgets[programName+"-list"].(*widget.List); ok {
						list.UnselectAll()
					}
				},
				refresh: func() {
					refreshMasksAccordionForProgram(programName)
				},
			}, program)
		}
	case "Search Areas":
		if v, ok := et.SearchAreasTab.SelectedItem.(*models.SearchArea); ok && v.Name != "" {
			program, ok := getProgramForEditor(programName)
			if !ok {
				return
			}
			deleteEntityForTab(deleteEntityConfig{
				entityType: "search area",
				name:       v.Name,
				delete: func(p *models.Program) error {
					return ProgramSearchAreaRepo(p, config.MainMonitorSizeString).Delete(v.Name)
				},
				reset: func(p *models.Program) {
					et.SearchAreasTab.SelectedItem = ProgramSearchAreaRepo(p, config.MainMonitorSizeString).New()
					if list, ok := et.SearchAreasTab.Widgets[programName+"-list"].(*widget.List); ok {
						list.UnselectAll()
					}
				},
				refresh: func() {
					vision.InvalidatePreviewTooltipCacheEntity(v.Name)
					refreshSearchAreasAccordionForProgram(programName)
				},
			}, program)
		}
	}
}
