package binders

import (
	"Sqyre/internal/config"
	"Sqyre/internal/models"
	"Sqyre/internal/models/repositories"
	"Sqyre/ui"
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
	"github.com/lithammer/fuzzysearch/fuzzy"
)

func setMaskWidgets(m models.Mask, programName string) {
	mtw := ui.GetUi().EditorTabs.MasksTab.Widgets
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
	if shape == "rectangle" {
		shape = "Rectangle"
	} else if shape == "circle" {
		shape = "Circle"
	}
	mtw["shapeSelect"].(*widget.RadioGroup).SetSelected(shape)

	if inverseCheck, ok := mtw["Inverse"].(*widget.Check); ok {
		inverseCheck.SetChecked(m.Inverse)
	}

	hasImage := ui.HasMaskImage(programName, m.Name)
	ui.GetUi().SetMaskImageMode(hasImage)
	if hasImage {
		ui.GetUi().UpdateMaskPreview(programName, m.Name)
	} else {
		ui.GetUi().ClearMaskPreviewImage()
	}
}

func setAccordionMasksLists(acc *widget.Accordion) {
	acc.Items = []*widget.AccordionItem{}
	et := ui.GetUi().EditorTabs.MasksTab
	filterText := ""
	if sb, ok := et.Widgets["searchbar"].(*widget.Entry); ok {
		filterText = sb.Text
		sb.OnChanged = func(string) { setAccordionMasksLists(acc) }
	}

	for _, p := range repositories.ProgramRepo().GetAll() {
		defaultList := p.MaskRepo().GetAllKeys()
		filtered := defaultList
		if filterText != "" {
			filtered = []string{}
			for _, i := range defaultList {
				if fuzzy.MatchFold(filterText, i) {
					filtered = append(filtered, i)
				}
			}
		}
		// Show program if search is empty, or program name matches, or any mask name matches
		if filterText != "" && !fuzzy.MatchFold(filterText, p.Name) && len(filtered) == 0 {
			continue
		}

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
				program, err := repositories.ProgramRepo().Get(p.Name)
				if err != nil {
					log.Printf("Error getting program %s: %v", p.Name, err)
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
			program, err := repositories.ProgramRepo().Get(p.Name)
			if err != nil {
				log.Printf("Error getting program %s: %v", p.Name, err)
				return
			}
			ui.GetUi().ProgramSelector.SetText(program.Name)
			maskName := lists.filtered[id]
			mask, err := program.MaskRepo().Get(maskName)
			if err != nil {
				return
			}
			ui.GetUi().EditorTabs.MasksTab.SelectedItem = mask
			setMaskWidgets(*mask, program.Name)
			markMasksClean()
		}

		programMaskListWidget := *widget.NewAccordionItem(p.Name, lists.masks)
		ui.GetUi().EditorTabs.MasksTab.Widgets[p.Name+"-list"] = lists.masks
		acc.Append(&programMaskListWidget)
	}
}

func readMaskShapeFromUI() string {
	mtw := ui.GetUi().EditorTabs.MasksTab.Widgets
	sel := mtw["shapeSelect"].(*widget.RadioGroup).Selected
	switch sel {
	case "Circle":
		return "circle"
	default:
		return "rectangle"
	}
}

func setMasksForms() {
	et := ui.GetUi().EditorTabs

	et.MasksTab.UpdateButton.OnTapped = func() {
		w := et.MasksTab.Widgets
		n := w["Name"].(*widget.Entry).Text

		if v, ok := et.MasksTab.SelectedItem.(*models.Mask); ok {
			p := ui.GetUi().ProgramSelector.Text
			program, err := repositories.ProgramRepo().Get(p)
			if err != nil {
				log.Printf("Error getting program %s: %v", p, err)
				return
			}
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
				dialog.ShowError(fmt.Errorf("failed to save mask"), ui.GetUi().Window)
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

			hasImage := ui.HasMaskImage(p, v.Name)
			ui.GetUi().SetMaskImageMode(hasImage)
			if hasImage {
				ui.GetUi().UpdateMaskPreview(p, v.Name)
			}

			if acc, ok := et.MasksTab.Widgets["Accordion"].(*widget.Accordion); ok {
				setAccordionMasksLists(acc)
			}
			markMasksClean()
		}
	}
}

func setMasksButtons() {
	et := ui.GetUi().EditorTabs

	// Upload image button
	if uploadBtn, ok := et.MasksTab.Widgets["uploadButton"].(*widget.Button); ok {
		uploadBtn.OnTapped = func() {
			mask, ok := et.MasksTab.SelectedItem.(*models.Mask)
			if !ok || mask.Name == "" {
				dialog.ShowError(fmt.Errorf("select a mask first"), ui.GetUi().Window)
				return
			}
			programName := ui.GetUi().ProgramSelector.Text
			if programName == "" {
				dialog.ShowError(fmt.Errorf("select a program first"), ui.GetUi().Window)
				return
			}

			fd := dialog.NewFileOpen(func(reader fyne.URIReadCloser, err error) {
				if err != nil {
					dialog.ShowError(err, ui.GetUi().Window)
					return
				}
				if reader == nil {
					return
				}
				defer reader.Close()

				masksPath := config.GetMasksPath()
				programMaskDir := filepath.Join(masksPath, programName)
				if err := os.MkdirAll(programMaskDir, 0755); err != nil {
					dialog.ShowError(fmt.Errorf("failed to create mask directory: %v", err), ui.GetUi().Window)
					return
				}

				destPath := filepath.Join(programMaskDir, mask.Name+config.PNG)
				destFile, err := os.Create(destPath)
				if err != nil {
					dialog.ShowError(fmt.Errorf("failed to create mask file: %v", err), ui.GetUi().Window)
					return
				}
				defer destFile.Close()

				if _, err := io.Copy(destFile, reader); err != nil {
					dialog.ShowError(fmt.Errorf("failed to write mask image: %v", err), ui.GetUi().Window)
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

				ui.GetUi().SetMaskImageMode(true)
				ui.GetUi().UpdateMaskPreview(programName, mask.Name)
			}, ui.GetUi().Window)
			fd.SetFilter(storage.NewExtensionFileFilter([]string{".png", ".jpg", ".jpeg", ".bmp"}))
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
			programName := ui.GetUi().ProgramSelector.Text
			masksPath := config.GetMasksPath()
			imgPath := filepath.Join(masksPath, programName, mask.Name+config.PNG)
			if err := os.Remove(imgPath); err != nil && !os.IsNotExist(err) {
				dialog.ShowError(fmt.Errorf("failed to remove mask image: %v", err), ui.GetUi().Window)
				return
			}
			ui.GetUi().SetMaskImageMode(false)
			ui.GetUi().ClearMaskPreviewImage()
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
