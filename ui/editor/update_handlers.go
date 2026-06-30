package editor

import (
	"Sqyre/internal/config"
	"Sqyre/internal/models"
	"Sqyre/internal/models/actions"
	"Sqyre/internal/models/repositories"
	"Sqyre/internal/services"
	"Sqyre/ui/completionentry"
	"Sqyre/ui/custom_widgets"
	"os"
	"strconv"
	"strings"

	"fyne.io/fyne/v2/widget"
)

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
		n := tab.Widgets["Name"].(*widget.Entry).Text
		si, ok := tab.SelectedItem.(*models.Program)
		if !ok {
			return
		}
		saveRenamableEntity(renamableSaveConfig{
			entityType: "program",
			oldName:    si.Name,
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
		w := tab.Widgets
		n := w["Name"].(*widget.Entry).Text
		x, _ := strconv.Atoi(w["Cols"].(*widget.Entry).Text)
		y, _ := strconv.Atoi(w["Rows"].(*widget.Entry).Text)
		sm, _ := strconv.Atoi(w["StackMax"].(*widget.Entry).Text)
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
				_, err := program.ItemRepo().Get(name)
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
				return program.ItemRepo().Delete(oldName)
			},
			save: func() error {
				v.Name = n
				v.GridSize = [2]int{x, y}
				v.StackMax = sm
				return program.ItemRepo().Set(v.Name, v)
			},
			onSuccess: func() {
				saveProgramAfterMutation(program, p)
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
		p := pointFromWidgets(tab.Widgets)
		v, ok := tab.SelectedItem.(*models.Point)
		if !ok {
			return
		}
		programName := tab.ProgramSelector.Selected
		program, ok := getProgramForEditor(programName)
		if !ok {
			return
		}
		repo := program.PointRepo(config.MainMonitorSizeString)
		saveRenamableEntity(renamableSaveConfig{
			entityType: "point",
			oldName:    v.Name,
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
				saveProgramAfterMutation(program, programName)
				safeUpdatePointPreview(v)
				if acc, ok := tab.Widgets["Accordion"].(*widget.Accordion); ok {
					setAccordionPointsLists(acc)
				}
				markPointsClean()
			},
		})
	}
}

func setSearchAreaUpdateHandler() {
	tab := shell().EditorTabs.SearchAreasTab
	tab.UpdateButton.OnTapped = func() {
		sa := searchAreaFromWidgets(tab.Widgets)
		v, ok := tab.SelectedItem.(*models.SearchArea)
		if !ok {
			return
		}
		programName := tab.ProgramSelector.Selected
		program, ok := getProgramForEditor(programName)
		if !ok {
			return
		}
		repo := program.SearchAreaRepo(config.MainMonitorSizeString)
		saveRenamableEntity(renamableSaveConfig{
			entityType: "search area",
			oldName:    v.Name,
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
				saveProgramAfterMutation(program, programName)
				safeUpdateSearchAreaPreview(v)
				if acc, ok := tab.Widgets["Accordion"].(*widget.Accordion); ok {
					setAccordionSearchAreasLists(acc)
				}
				markSearchAreasClean()
			},
		})
	}
}

func setMaskUpdateHandler() {
	tab := shell().EditorTabs.MasksTab
	tab.UpdateButton.OnTapped = func() {
		m := maskFromWidgets(tab.Widgets)
		v, ok := tab.SelectedItem.(*models.Mask)
		if !ok {
			return
		}
		programName := tab.ProgramSelector.Selected
		program, ok := getProgramForEditor(programName)
		if !ok {
			return
		}
		repo := program.MaskRepo()
		saveRenamableEntity(renamableSaveConfig{
			entityType: "mask",
			oldName:    v.Name,
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
				saveProgramAfterMutation(program, programName)
				hasImage := HasMaskImage(programName, v.Name)
				shell().SetMaskImageMode(hasImage)
				if hasImage {
					shell().UpdateMaskPreview(programName, v.Name)
				}
				if acc, ok := tab.Widgets["Accordion"].(*widget.Accordion); ok {
					setAccordionMasksLists(acc)
				}
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
		for _, existingTag := range v.Tags {
			if existingTag == tagText {
				return
			}
		}
		v.Tags = append(v.Tags, tagText)
		p := tab.ProgramSelector.Selected
		program, ok := getProgramForEditor(p)
		if !ok {
			return
		}
		if err := program.ItemRepo().Set(v.Name, v); err != nil {
			editorRepoErr("save", "item", v.Name, err)
			return
		}
		if !saveProgramAfterMutation(program, p) {
			return
		}
		updateTagsDisplay(v)
		tagEntry.SetText("")
	}
	tagEntry.OnChanged = func(text string) {
		if text == "" {
			tagEntry.HideCompletion()
			return
		}
		allTags := getAllExistingTags()
		searchLower := strings.ToLower(text)
		matchingTags := []string{}
		for _, tag := range allTags {
			if strings.Contains(strings.ToLower(tag), searchLower) {
				matchingTags = append(matchingTags, tag)
			}
		}
		if len(matchingTags) > 10 {
			matchingTags = matchingTags[:10]
		}
		if len(matchingTags) == 0 {
			tagEntry.HideCompletion()
			return
		}
		tagEntry.SetOptions(matchingTags)
		tagEntry.ShowCompletion()
	}
	tagEntry.OnSubmitted = func(string) { submitTag() }
	if tagSubmitButton, ok := tab.Widgets["tagSubmitButton"].(*widget.Button); ok {
		tagSubmitButton.OnTapped = submitTag
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
	wirePointPreviewRefresh(et.PointsTab.PreviewRefreshButton, et.PointsTab.Widgets)
	wireSearchAreaPreviewRefresh(et.SearchAreasTab.PreviewRefreshButton, et.SearchAreasTab.Widgets)

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
	shell().UpdatePointPreview(p)
}

func safeUpdateSearchAreaPreview(sa *models.SearchArea) {
	defer func() {
		if r := recover(); r != nil {
			services.LogPanicToFile(r, "SearchArea: Preview update (area: "+sa.Name+")")
		}
	}()
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

func macroPointSync(point *models.Point) {
	if st := activeWire.MacroMTabs().SelectedTab(); st != nil {
		if v, ok := st.Macro.Root.GetAction(st.SelectedNode).(*actions.Move); ok {
			v.Point = actions.Point{Name: point.Name, X: point.X, Y: point.Y}
		}
	}
}
