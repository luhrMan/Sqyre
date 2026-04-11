package editor

import (
	"Sqyre/internal/config"
	"Sqyre/internal/models"
	"Sqyre/internal/models/repositories"
	"Sqyre/ui/custom_widgets"
	"fmt"
	"io"
	"log"
	"os"
	"path/filepath"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/dialog"
	"fyne.io/fyne/v2/storage"
	"fyne.io/fyne/v2/widget"
)

func setMaskWidgets(m models.Mask, programName string) {
	mtw := shell().EditorTabs.MasksTab.Widgets
	mtw["Name"].(*widget.Entry).SetText(m.Name)
	custom_widgets.SetEntryText(mtw["CenterX"], m.CenterX)
	custom_widgets.SetEntryText(mtw["CenterY"], m.CenterY)
	custom_widgets.SetEntryText(mtw["Base"], m.Base)
	custom_widgets.SetEntryText(mtw["Height"], m.Height)
	custom_widgets.SetEntryText(mtw["Radius"], m.Radius)

	shape := m.Shape
	if shape == "" {
		shape = "Rectangle"
	}
	// Capitalize for RadioGroup display values
	switch shape {
	case "rectangle":
		shape = "Rectangle"
	case "circle":
		shape = "Circle"
	}
	mtw["shapeSelect"].(*widget.RadioGroup).SetSelected(shape)

	if inverseCheck, ok := mtw["Inverse"].(*widget.Check); ok {
		inverseCheck.SetChecked(m.Inverse)
	}

	hasImage := HasMaskImage(programName, m.Name)
	shell().SetMaskImageMode(hasImage)
	if hasImage {
		shell().UpdateMaskPreview(programName, m.Name)
	} else {
		shell().ClearMaskPreviewImage()
	}
	shell().RefreshEditorActionBar()
}

func buildMaskAccordionItemForProgram(p *models.Program, filterText string) *widget.AccordionItem {
	defaultList := p.MaskRepo().GetAllKeys()
	filtered := filterKeysByFuzzy(filterText, defaultList)
	sortMaskKeysByDisplayName(p, filtered)
	if skipProgramAccordionRow(filterText, p.Name, filtered) {
		return nil
	}
	prog := p
	lists := struct {
		masks    *widget.List
		filtered []string
	}{filtered: filtered}

	lists.masks = widget.NewList(
		func() int { return len(lists.filtered) },
		func() fyne.CanvasObject { return widget.NewLabel("template") },
		func(id widget.ListItemID, co fyne.CanvasObject) {
			name := lists.filtered[id]
			label := co.(*widget.Label)
			program, err := repositories.ProgramRepo().Get(prog.Name)
			if err != nil {
				log.Printf("Error getting program %s: %v", prog.Name, err)
				return
			}
			mask, err := program.MaskRepo().Get(name)
			if err != nil {
				return
			}
			label.SetText(mask.Name)
		},
	)

	lists.masks.OnSelected = func(id widget.ListItemID) {
		program, err := repositories.ProgramRepo().Get(prog.Name)
		if err != nil {
			log.Printf("Error getting program %s: %v", prog.Name, err)
			return
		}
		shell().EditorTabs.MasksTab.ProgramSelector.SetSelected(program.Name)
		maskName := lists.filtered[id]
		mask, err := program.MaskRepo().Get(maskName)
		if err != nil {
			return
		}
		shell().EditorTabs.MasksTab.SelectedItem = mask
		setMaskWidgets(*mask, program.Name)
		markMasksClean()
	}

	shell().EditorTabs.MasksTab.Widgets[prog.Name+"-list"] = lists.masks
	return widget.NewAccordionItem(fmt.Sprintf("%s (%d)", prog.Name, len(filtered)), lists.masks)
}

func setAccordionMasksLists(acc *widget.Accordion) {
	et := shell().EditorTabs.MasksTab
	filterText := ""
	if sb, ok := et.Widgets["searchbar"].(*widget.Entry); ok {
		filterText = sb.Text
		sb.OnChanged = func(string) { setAccordionMasksLists(acc) }
	}
	var items []*widget.AccordionItem
	for _, p := range repositories.ProgramRepo().GetAllSortedByName() {
		if it := buildMaskAccordionItemForProgram(p, filterText); it != nil {
			items = append(items, it)
		}
	}
	acc.Items = items
	acc.Refresh()
}

// refreshMasksAccordionProgramRow rebuilds one program row after an in-place edit (e.g. Update).
func refreshMasksAccordionProgramRow(acc *widget.Accordion, programName string) {
	p, err := repositories.ProgramRepo().Get(programName)
	if err != nil {
		log.Printf("Error getting program %s: %v", programName, err)
		return
	}
	et := shell().EditorTabs.MasksTab
	filterText := ""
	if sb, ok := et.Widgets["searchbar"].(*widget.Entry); ok {
		filterText = sb.Text
	}
	i := accordionRowIndexForProgram(acc, programName)
	newItem := buildMaskAccordionItemForProgram(p, filterText)
	if newItem == nil {
		if i >= 0 {
			setAccordionMasksLists(acc)
		}
		return
	}
	if i < 0 {
		setAccordionMasksLists(acc)
		return
	}
	wasOpen := acc.Items[i].Open
	newItem.Open = wasOpen
	acc.Items[i] = newItem
	acc.Refresh()
}

func readMaskShapeFromUI() string {
	mtw := shell().EditorTabs.MasksTab.Widgets
	sel := mtw["shapeSelect"].(*widget.RadioGroup).Selected
	switch sel {
	case "Circle":
		return "circle"
	default:
		return "rectangle"
	}
}

func setMasksForms() {
	et := shell().EditorTabs

	et.MasksTab.UpdateButton.OnTapped = func() {
		w := et.MasksTab.Widgets
		n := w["Name"].(*widget.Entry).Text

		if v, ok := et.MasksTab.SelectedItem.(*models.Mask); ok {
			p := shell().EditorTabs.MasksTab.ProgramSelector.Selected
			if p == "" {
				activeWire.ShowErrorWithEscape(fmt.Errorf("program cannot be empty"), activeWire.Window)
				return
			}
			program, err := repositories.ProgramRepo().Get(p)
			if err != nil {
				log.Printf("Error getting program %s: %v", p, err)
				return
			}
			applyMaskUpdate := func() {
				oldkey := v.Name
				v.Name = n
				v.Shape = readMaskShapeFromUI()
				v.CenterX = custom_widgets.EntryText(w["CenterX"])
				v.CenterY = custom_widgets.EntryText(w["CenterY"])
				v.Base = custom_widgets.EntryText(w["Base"])
				v.Height = custom_widgets.EntryText(w["Height"])
				v.Radius = custom_widgets.EntryText(w["Radius"])
				if inverseCheck, ok := w["Inverse"].(*widget.Check); ok {
					v.Inverse = inverseCheck.Checked
				}

				if err := program.MaskRepo().Set(v.Name, v); err != nil {
					log.Printf("Error saving mask %s: %v", v.Name, err)
					activeWire.ShowErrorWithEscape(fmt.Errorf("failed to save mask"), activeWire.Window)
					return
				}

				if oldkey != v.Name {
					if err := program.MaskRepo().Delete(oldkey); err != nil {
						log.Printf("Error deleting mask %s: %v", oldkey, err)
					}
					renameMaskImage(p, oldkey, v.Name)
				}

				if err := repositories.ProgramRepo().Set(program.Name, program); err != nil {
					log.Printf("Error saving program %s: %v", p, err)
					return
				}

				hasImage := HasMaskImage(p, v.Name)
				shell().SetMaskImageMode(hasImage)
				if hasImage {
					shell().UpdateMaskPreview(p, v.Name)
				}

				if acc, ok := et.MasksTab.Widgets["Accordion"].(*widget.Accordion); ok {
					refreshMasksAccordionProgramRow(acc, p)
				}
				markMasksClean()
			}

			if v.Name != n {
				if shouldConfirmOverwrite("mask", n, func(name string) bool {
					_, err := program.MaskRepo().Get(name)
					return err == nil
				}, activeWire.Window, applyMaskUpdate) {
					return
				}
			}
			applyMaskUpdate()
		}
	}
}

func setMasksButtons() {
	et := shell().EditorTabs

	// Upload image button
	if uploadBtn, ok := et.MasksTab.Widgets["uploadButton"].(*widget.Button); ok {
		uploadBtn.OnTapped = func() {
			mask, ok := et.MasksTab.SelectedItem.(*models.Mask)
			if !ok || mask.Name == "" {
				activeWire.ShowErrorWithEscape(fmt.Errorf("select a mask first"), activeWire.Window)
				return
			}
			programName := shell().EditorTabs.MasksTab.ProgramSelector.Selected
			if programName == "" {
				activeWire.ShowErrorWithEscape(fmt.Errorf("select a program first"), activeWire.Window)
				return
			}

			fd := dialog.NewFileOpen(func(reader fyne.URIReadCloser, err error) {
				if err != nil {
					activeWire.ShowErrorWithEscape(err, activeWire.Window)
					return
				}
				if reader == nil {
					return
				}
				defer reader.Close()

				masksPath := config.GetMasksPath()
				programMaskDir := filepath.Join(masksPath, programName)
				if err := os.MkdirAll(programMaskDir, 0755); err != nil {
					activeWire.ShowErrorWithEscape(fmt.Errorf("failed to create mask directory: %v", err), activeWire.Window)
					return
				}

				destPath := filepath.Join(programMaskDir, mask.Name+config.PNG)
				destFile, err := os.Create(destPath)
				if err != nil {
					activeWire.ShowErrorWithEscape(fmt.Errorf("failed to create mask file: %v", err), activeWire.Window)
					return
				}
				defer destFile.Close()

				if _, err := io.Copy(destFile, reader); err != nil {
					activeWire.ShowErrorWithEscape(fmt.Errorf("failed to write mask image: %v", err), activeWire.Window)
					return
				}

				program, err := repositories.ProgramRepo().Get(programName)
				if err != nil {
					log.Printf("Error getting program %s: %v", programName, err)
					return
				}
				if err := program.MaskRepo().Set(mask.Name, mask); err != nil {
					log.Printf("Error saving mask %s: %v", mask.Name, err)
				}
				if err := repositories.ProgramRepo().Set(program.Name, program); err != nil {
					log.Printf("Error saving program %s: %v", programName, err)
				}

				shell().SetMaskImageMode(true)
				shell().UpdateMaskPreview(programName, mask.Name)
			}, activeWire.Window)
			fd.SetFilter(storage.NewExtensionFileFilter([]string{".png", ".jpg", ".jpeg", ".bmp"}))
			activeWire.AddDialogEscapeClose(fd, activeWire.Window)
			fd.Show()
		}
	}

	// Remove image button
	if removeBtn, ok := et.MasksTab.Widgets["removeImageButton"].(*widget.Button); ok {
		removeBtn.OnTapped = func() {
			mask, ok := et.MasksTab.SelectedItem.(*models.Mask)
			if !ok || mask.Name == "" {
				return
			}
			programName := shell().EditorTabs.MasksTab.ProgramSelector.Selected
			masksPath := config.GetMasksPath()
			imgPath := filepath.Join(masksPath, programName, mask.Name+config.PNG)
			if err := os.Remove(imgPath); err != nil && !os.IsNotExist(err) {
				activeWire.ShowErrorWithEscape(fmt.Errorf("failed to remove mask image: %v", err), activeWire.Window)
				return
			}
			shell().SetMaskImageMode(false)
			shell().ClearMaskPreviewImage()
		}
	}
}

func renameMaskImage(programName, oldName, newName string) {
	masksPath := config.GetMasksPath()
	oldPath := filepath.Join(masksPath, programName, oldName+config.PNG)
	newPath := filepath.Join(masksPath, programName, newName+config.PNG)
	if _, err := os.Stat(oldPath); err == nil {
		if err := os.Rename(oldPath, newPath); err != nil {
			log.Printf("Warning: Failed to rename mask image %s to %s: %v", oldPath, newPath, err)
		}
	}
}
