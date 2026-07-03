package editor

import (
	"Sqyre/internal/config"
	"Sqyre/internal/models"
	"Sqyre/ui/custom_widgets"
	"fmt"

	"fyne.io/fyne/v2/widget"
)

func setSearchAreaWidgets(sa models.SearchArea) {
	st := shell().EditorTabs.SearchAreasTab.Widgets
	st["Name"].(*widget.Entry).SetText(sa.Name)
	custom_widgets.SetEntryText(st["LeftX"], fmt.Sprintf("%v", sa.LeftX))
	custom_widgets.SetEntryText(st["TopY"], fmt.Sprintf("%v", sa.TopY))
	custom_widgets.SetEntryText(st["RightX"], fmt.Sprintf("%v", sa.RightX))
	custom_widgets.SetEntryText(st["BottomY"], fmt.Sprintf("%v", sa.BottomY))
	shell().RefreshEditorActionBar()
}

func setPointWidgets(p models.Point) {
	pt := shell().EditorTabs.PointsTab
	pt.Widgets["Name"].(*widget.Entry).SetText(p.Name)
	custom_widgets.SetEntryText(pt.Widgets["X"], fmt.Sprintf("%v", p.X))
	custom_widgets.SetEntryText(pt.Widgets["Y"], fmt.Sprintf("%v", p.Y))
	safeUpdatePointPreview(&p)
	shell().RefreshEditorActionBar()
}

func searchAreasAccordionConfig() entityAccordionConfig {
	tab := shell().EditorTabs.SearchAreasTab
	return entityAccordionConfig{
		tab: tab,
		getKeys: func(p *models.Program) []string {
			return ProgramSearchAreaRepo(p, config.MainMonitorSizeString).GetAllKeys()
		},
		sortKeys: sortSearchAreaKeysByDisplayName,
		getEntity: func(p *models.Program, key string) (string, error) {
			sa, err := ProgramSearchAreaRepo(p, config.MainMonitorSizeString).Get(key)
			if err != nil {
				return "", err
			}
			return sa.Name, nil
		},
		getPreviewImage: LoadSearchAreaPreviewImage,
		onSelected: func(p *models.Program, key string) {
			sa, err := ProgramSearchAreaRepo(p, config.MainMonitorSizeString).Get(key)
			if err != nil {
				return
			}
			tab.SelectedItem = sa
			setSearchAreaWidgets(*sa)
			safeUpdateSearchAreaPreview(sa)
			markSearchAreasClean()
		},
	}
}

func setAccordionSearchAreasLists(acc *widget.Accordion) {
	populateProgramEntityAccordion(acc, searchAreasAccordionConfig())
}

// refreshSearchAreasAccordionForProgram rebuilds only the given program's row in
// the Search Areas accordion (instead of every program's row).
func refreshSearchAreasAccordionForProgram(programName string) {
	if acc, ok := shell().EditorTabs.SearchAreasTab.Widgets["Accordion"].(*widget.Accordion); ok {
		refreshProgramEntityAccordionRow(acc, searchAreasAccordionConfig(), programName)
	}
}

func pointsAccordionConfig() entityAccordionConfig {
	tab := shell().EditorTabs.PointsTab
	return entityAccordionConfig{
		tab: tab,
		getKeys: func(p *models.Program) []string {
			return ProgramPointRepo(p, config.MainMonitorSizeString).GetAllKeys()
		},
		sortKeys: sortPointKeysByDisplayName,
		getEntity: func(p *models.Program, key string) (string, error) {
			point, err := ProgramPointRepo(p, config.MainMonitorSizeString).Get(key)
			if err != nil {
				return "", err
			}
			return point.Name, nil
		},
		getPreviewImage: LoadPointPreviewImage,
		onSelected: func(p *models.Program, key string) {
			point, err := ProgramPointRepo(p, config.MainMonitorSizeString).Get(key)
			if err != nil {
				return
			}
			tab.SelectedItem = point
			setPointWidgets(*point)
			markPointsClean()
		},
	}
}

func setAccordionPointsLists(acc *widget.Accordion) {
	populateProgramEntityAccordion(acc, pointsAccordionConfig())
}

// refreshPointsAccordionForProgram rebuilds only the given program's row in the
// Points accordion (instead of every program's row).
func refreshPointsAccordionForProgram(programName string) {
	if acc, ok := shell().EditorTabs.PointsTab.Widgets["Accordion"].(*widget.Accordion); ok {
		refreshProgramEntityAccordionRow(acc, pointsAccordionConfig(), programName)
	}
}

func setAccordionAutoPicSearchAreasLists(acc *widget.Accordion) {
	tab := shell().EditorTabs.AutoPicTab
	populateProgramEntityAccordion(acc, entityAccordionConfig{
		tab: tab,
		getKeys: func(p *models.Program) []string {
			return ProgramSearchAreaRepo(p, config.MainMonitorSizeString).GetAllKeys()
		},
		sortKeys: sortSearchAreaKeysByDisplayName,
		getEntity: func(p *models.Program, key string) (string, error) {
			sa, err := ProgramSearchAreaRepo(p, config.MainMonitorSizeString).Get(key)
			if err != nil {
				return "", err
			}
			return sa.Name, nil
		},
		onSelected: func(p *models.Program, key string) {
			sa, err := ProgramSearchAreaRepo(p, config.MainMonitorSizeString).Get(key)
			if err != nil {
				editorRepoLog("load", "search area", key, err)
				return
			}
			tab.SelectedItem = sa
			if saveButton, ok := tab.Widgets["saveButton"].(*widget.Button); ok {
				saveButton.Enable()
			}
			safeUpdateAutoPicPreview(sa)
		},
		getPreviewImage: LoadSearchAreaPreviewImage,
	})
}
