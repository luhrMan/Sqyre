package editor

import (
	"Sqyre/internal/config"
	"Sqyre/internal/models"
	"Sqyre/internal/models/repositories"
	"Sqyre/internal/services"
	"Sqyre/internal/validation"
	"Sqyre/internal/vision"
	"Sqyre/ui/completionentry"
	"Sqyre/ui/custom_widgets"
	"fmt"
	"os"
	"slices"
	"strings"

	"fyne.io/fyne/v2/widget"
)

func refreshOpenMacroTreesAfterRename() {
	if mtabs := activeWire.MacroMTabs(); mtabs != nil {
		for _, tree := range mtabs.AllTrees() {
			tree.RefreshVisibleRowDisplays()
		}
	}
}

func propagateProgramRename(oldName, newName string) {
	if _, err := repositories.PropagateProgramRename(oldName, newName); err != nil {
		editorRepoLog("propagate rename", "program", oldName, err)
	}
	refreshOpenMacroTreesAfterRename()
}

func propagateEntityRename(kind models.ProgramEntityKind, program, oldName, newName string) {
	if _, err := repositories.PropagateProgramEntityRename(kind, program, oldName, newName); err != nil {
		editorRepoLog("propagate rename", program, oldName, err)
	}
	refreshOpenMacroTreesAfterRename()
}

func propagateMaskRename(program, oldName, newName string) {
	if _, err := repositories.PropagateMaskRenameInProgram(program, oldName, newName); err != nil {
		editorRepoLog("propagate rename", "mask", oldName, err)
	}
}

func setEditorUpdateHandlers() {
	setProgramUpdateHandler()
	setItemUpdateHandler()
	setPointUpdateHandler()
	setSearchAreaUpdateHandler()
	setMaskUpdateHandler()
}

func setProgramUpdateHandler() {
	tab := shell().EditorTabs.ProgramsTab
	tab.UpdateButton.OnTapped = func() {
		if !allTabFieldsValid(tab) {
			return
		}
		n := tab.Widgets["Name"].(*widget.Entry).Text
		if err := validateEntityNameForSave(n); err != nil {
			editorErr(err)
			return
		}
		si, ok := tab.SelectedItem.(*models.Program)
		if !ok {
			return
		}
		oldProgramName := si.Name
		saveRenamableEntity(renamableSaveConfig{
			entityType: "program",
			oldName:    oldProgramName,
			newName:    n,
			exists: func(name string) bool {
				_, err := repositories.ProgramRepo().Get(name)
				return err == nil
			},
			deleteOld: repositories.ProgramRepo().Delete,
			save: func() error {
				si.Name = n
				return repositories.ProgramRepo().Set(si.Name, si)
			},
			onSuccess: func() {
				propagateProgramRename(oldProgramName, n)
				refreshAllProgramRelatedUI()
				updateProgramSelectorOptions()
				markProgramsClean()
			},
		})
	}
}

func setItemUpdateHandler() {
	tab := shell().EditorTabs.ItemsTab
	tab.UpdateButton.OnTapped = func() {
		if !allTabFieldsValid(tab) {
			return
		}
		w := tab.Widgets
		n := w["Name"].(*widget.Entry).Text
		if err := validateEntityNameForSave(n); err != nil {
			editorErr(err)
			return
		}
		if err := validateItemGridForSave(w); err != nil {
			editorErr(err)
			return
		}
		x, err := validation.ParsePositiveInt(custom_widgets.EntryText(w["Cols"]))
		if err != nil {
			editorErr(fmt.Errorf("cols: %w", err))
			return
		}
		y, err := validation.ParsePositiveInt(custom_widgets.EntryText(w["Rows"]))
		if err != nil {
			editorErr(fmt.Errorf("rows: %w", err))
			return
		}
		sm, err := validation.ParseNonNegativeInt(custom_widgets.EntryText(w["StackMax"]))
		if err != nil {
			editorErr(fmt.Errorf("stack max: %w", err))
			return
		}
		v, ok := tab.SelectedItem.(*models.Item)
		if !ok {
			return
		}
		p := tab.ProgramSelector.Selected
		program, ok := getProgramForEditor(p)
		if !ok {
			return
		}
		oldItemName := v.Name
		saveRenamableEntity(renamableSaveConfig{
			entityType: "item",
			oldName:    v.Name,
			newName:    n,
			exists: func(name string) bool {
				_, err := ProgramItemRepo(program).Get(name)
				return err == nil
			},
			deleteOld: func(oldName string) error {
				if oldName == n {
					return nil
				}
				iconService := services.IconVariantServiceInstance()
				if oldVariants, err := iconService.GetVariants(p, oldName); err == nil {
					for _, variant := range oldVariants {
						oldPath := iconService.GetVariantPath(p, oldName, variant)
						newPath := iconService.GetVariantPath(p, n, variant)
						if _, err := os.Stat(oldPath); err == nil {
							if err := os.Rename(oldPath, newPath); err != nil {
								editorRepoLog("rename file", "item variant", oldPath, err)
							}
						}
					}
				}
				return ProgramItemRepo(program).Delete(oldName)
			},
			save: func() error {
				v.Name = n
				v.GridSize = [2]int{x, y}
				v.StackMax = sm
				return ProgramItemRepo(program).Set(v.Name, v)
			},
			onSuccess: func() {
				propagateEntityRename(models.ProgramEntityItem, p, oldItemName, n)
				RefreshItemInGrid(p, oldItemName, v.Name)
				if editor, ok := w["iconVariantEditor"].(*custom_widgets.IconVariantEditor); ok {
					iconService := services.IconVariantServiceInstance()
					editor.SetProgramAndItem(p, iconService.GetBaseItemName(v.Name))
				}
				markItemsClean()
			},
		})
	}
}

func setPointUpdateHandler() {
	tab := shell().EditorTabs.PointsTab
	tab.UpdateButton.OnTapped = func() {
		if !allTabFieldsValid(tab) {
			return
		}
		p := pointFromWidgets(tab.Widgets)
		if err := validateEntityNameForSave(p.Name); err != nil {
			editorErr(err)
			return
		}
		v, ok := tab.SelectedItem.(*models.Point)
		if !ok {
			return
		}
		programName := tab.ProgramSelector.Selected
		program, ok := getProgramForEditor(programName)
		if !ok {
			return
		}
		repo := ProgramPointRepo(program, config.MainMonitorSizeString)
		oldPointName := v.Name
		saveRenamableEntity(renamableSaveConfig{
			entityType: "point",
			oldName:    oldPointName,
			newName:    p.Name,
			exists: func(name string) bool {
				_, err := repo.Get(name)
				return err == nil
			},
			deleteOld: repo.Delete,
			save: func() error {
				v.Name = p.Name
				v.X = p.X
				v.Y = p.Y
				return repo.Set(v.Name, v)
			},
			onSuccess: func() {
				vision.InvalidatePreviewTooltipCacheEntity(oldPointName)
				propagateEntityRename(models.ProgramEntityPoint, programName, oldPointName, p.Name)
				safeUpdatePointPreview(v)
				refreshPointsAccordionForProgram(programName)
				markPointsClean()
			},
		})
	}
}

func setSearchAreaUpdateHandler() {
	tab := shell().EditorTabs.SearchAreasTab
	tab.UpdateButton.OnTapped = func() {
		if !allTabFieldsValid(tab) {
			return
		}
		sa := searchAreaFromWidgets(tab.Widgets)
		if err := validateSearchAreaForSave(tab.Widgets); err != nil {
			editorErr(err)
			return
		}
		v, ok := tab.SelectedItem.(*models.SearchArea)
		if !ok {
			return
		}
		programName := tab.ProgramSelector.Selected
		program, ok := getProgramForEditor(programName)
		if !ok {
			return
		}
		repo := ProgramSearchAreaRepo(program, config.MainMonitorSizeString)
		oldAreaName := v.Name
		saveRenamableEntity(renamableSaveConfig{
			entityType: "search area",
			oldName:    oldAreaName,
			newName:    sa.Name,
			exists: func(name string) bool {
				_, err := repo.Get(name)
				return err == nil
			},
			deleteOld: repo.Delete,
			save: func() error {
				v.Name = sa.Name
				v.LeftX = sa.LeftX
				v.TopY = sa.TopY
				v.RightX = sa.RightX
				v.BottomY = sa.BottomY
				return repo.Set(v.Name, v)
			},
			onSuccess: func() {
				vision.InvalidatePreviewTooltipCacheEntity(oldAreaName)
				propagateEntityRename(models.ProgramEntitySearchArea, programName, oldAreaName, sa.Name)
				safeUpdateSearchAreaPreview(v)
				refreshSearchAreasAccordionForProgram(programName)
				markSearchAreasClean()
			},
		})
	}
}

func setMaskUpdateHandler() {
	tab := shell().EditorTabs.MasksTab
	tab.UpdateButton.OnTapped = func() {
		if !allTabFieldsValid(tab) {
			return
		}
		m := maskFromWidgets(tab.Widgets)
		if err := validateEntityNameForSave(m.Name); err != nil {
			editorErr(err)
			return
		}
		v, ok := tab.SelectedItem.(*models.Mask)
		if !ok {
			return
		}
		programName := tab.ProgramSelector.Selected
		program, ok := getProgramForEditor(programName)
		if !ok {
			return
		}
		repo := ProgramMaskRepo(program)
		oldMaskName := v.Name
		saveRenamableEntity(renamableSaveConfig{
			entityType: "mask",
			oldName:    oldMaskName,
			newName:    m.Name,
			exists: func(name string) bool {
				_, err := repo.Get(name)
				return err == nil
			},
			deleteOld: func(name string) error {
				if name != m.Name {
					renameMaskImage(programName, name, m.Name)
				}
				return repo.Delete(name)
			},
			save: func() error {
				*v = *m
				return repo.Set(v.Name, v)
			},
			onSuccess: func() {
				propagateMaskRename(programName, oldMaskName, m.Name)
				hasImage := HasMaskImage(programName, v.Name)
				shell().SetMaskImageMode(hasImage)
				if hasImage {
					shell().UpdateMaskPreview(programName, v.Name)
				}
				refreshMasksAccordionForProgram(programName)
				markMasksClean()
			},
		})
	}
}

func setItemTagHandlers(tab *EditorTab) {
	tagEntry, ok := tab.Widgets["tagEntry"].(*completionentry.CompletionEntry)
	if !ok {
		return
	}
	submitTag := func() {
		tagText := strings.TrimSpace(tagEntry.Text)
		tagEntry.HideCompletion()
		if tagText == "" {
			return
		}
		v, ok := tab.SelectedItem.(*models.Item)
		if !ok {
			return
		}
		if slices.Contains(v.Tags, tagText) {
			return
		}
		v.Tags = append(v.Tags, tagText)
		p := tab.ProgramSelector.Selected
		program, ok := getProgramForEditor(p)
		if !ok {
			return
		}
		if err := ProgramItemRepo(program).Set(v.Name, v); err != nil {
			editorRepoErr("save", v.Name, err)
			return
		}
		InvalidateProgramTagsCache(p)
		appendTagChip(v, tagText)
		tagEntry.SetText("")
	}
	tagEntry.OnChanged = func(text string) {
		if text == "" {
			tagEntry.HideCompletion()
			return
		}
		var item *models.Item
		if v, ok := tab.SelectedItem.(*models.Item); ok {
			item = v
		}
		matchingTags := tagCompletionOptions(tab.ProgramSelector.Selected, text, item, 10)
		if len(matchingTags) == 0 {
			tagEntry.HideCompletion()
			return
		}
		tagEntry.SetOptions(matchingTags)
		tagEntry.ShowCompletion()
	}
	tagEntry.OnSubmitted = func(string) { submitTag() }
	if tagSubmitButton, ok := tab.Widgets["tagSubmitButton"].(*widget.Button); ok {
		tagSubmitButton.OnTapped = func() {
			if strings.TrimSpace(tagEntry.Text) != "" {
				submitTag()
				return
			}
			var item *models.Item
			if v, ok := tab.SelectedItem.(*models.Item); ok {
				item = v
			}
			programTags := tagCompletionOptions(tab.ProgramSelector.Selected, "", item, 0)
			if len(programTags) == 0 {
				tagEntry.HideCompletion()
				return
			}
			tagEntry.SetOptions(programTags)
			tagEntry.ShowCompletion()
		}
	}
}

func setEditorRecordHandlers() {
	et := shell().EditorTabs
	wirePointRecordButton(et.PointsTab.Widgets, func(x, y int) {
		if v, ok := et.PointsTab.SelectedItem.(*models.Point); ok {
			safeUpdatePointPreview(&models.Point{Name: v.Name, X: x, Y: y})
		}
	})
	wireSearchAreaRecordButton(et.SearchAreasTab.Widgets, func(lx, ty, rx, by int) {
		if v, ok := et.SearchAreasTab.SelectedItem.(*models.SearchArea); ok {
			safeUpdateSearchAreaPreview(&models.SearchArea{
				Name: v.Name, LeftX: lx, TopY: ty, RightX: rx, BottomY: by,
			})
		}
	})
}

func setEditorPreviewRefreshHandlers() {
	et := shell().EditorTabs
	wirePointPreviewRefresh(et.PointsTab.previewPanel, et.PointsTab.PreviewRefreshButton, et.PointsTab.Widgets)
	wireSearchAreaPreviewRefresh(et.SearchAreasTab.previewPanel, et.SearchAreasTab.PreviewRefreshButton, et.SearchAreasTab.Widgets)

	if et.MasksTab.PreviewRefreshButton != nil {
		et.MasksTab.PreviewRefreshButton.OnTapped = func() {
			p := et.MasksTab.ProgramSelector.Selected
			n := et.MasksTab.Widgets["Name"].(*widget.Entry).Text
			if p == "" || n == "" {
				return
			}
			if HasMaskImage(p, n) {
				shell().UpdateMaskPreview(p, n)
			} else {
				shell().ClearMaskPreviewImage()
			}
		}
	}
	if et.AutoPicTab.PreviewRefreshButton != nil {
		et.AutoPicTab.PreviewRefreshButton.OnTapped = func() {
			sa, ok := et.AutoPicTab.SelectedItem.(*models.SearchArea)
			if !ok || sa == nil || sa.Name == "" {
				return
			}
			safeUpdateAutoPicPreview(sa)
		}
	}
}

func safeUpdatePointPreview(p *models.Point) {
	defer func() {
		if r := recover(); r != nil {
			services.LogPanicToFile(r, "Point: Preview update (point: "+p.Name+")")
		}
	}()
	if p != nil {
		vision.InvalidatePreviewTooltipCacheEntity(p.Name)
	}
	shell().UpdatePointPreview(p)
}

func safeUpdateSearchAreaPreview(sa *models.SearchArea) {
	defer func() {
		if r := recover(); r != nil {
			services.LogPanicToFile(r, "SearchArea: Preview update (area: "+sa.Name+")")
		}
	}()
	if sa != nil {
		vision.InvalidatePreviewTooltipCacheEntity(sa.Name)
	}
	shell().UpdateSearchAreaPreview(sa)
}

func safeUpdateAutoPicPreview(sa *models.SearchArea) {
	defer func() {
		if r := recover(); r != nil {
			services.LogPanicToFile(r, "AutoPic: Preview update (area: "+sa.Name+")")
		}
	}()
	shell().UpdateAutoPicPreview(sa)
}
