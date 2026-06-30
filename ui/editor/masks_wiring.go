package editor

import (
	"Sqyre/internal/config"
	"Sqyre/internal/models"
	"fmt"
	"io"
	"os"
	"path/filepath"

	"Sqyre/ui/custom_widgets"

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

func masksAccordionConfig() entityAccordionConfig {
	tab := shell().EditorTabs.MasksTab
	return entityAccordionConfig{
		tab: tab,
		getKeys: func(p *models.Program) []string {
			return p.MaskRepo().GetAllKeys()
		},
		sortKeys: sortMaskKeysByDisplayName,
		getEntity: func(p *models.Program, key string) (string, error) {
			mask, err := p.MaskRepo().Get(key)
			if err != nil {
				return "", err
			}
			return mask.Name, nil
		},
		onSelected: func(p *models.Program, key string) {
			mask, err := p.MaskRepo().Get(key)
			if err != nil {
				return
			}
			tab.SelectedItem = mask
			setMaskWidgets(*mask, p.Name)
			markMasksClean()
		},
	}
}

func setAccordionMasksLists(acc *widget.Accordion) {
	populateProgramEntityAccordion(acc, masksAccordionConfig())
}

// refreshMasksAccordionForProgram rebuilds only the given program's row in the
// Masks accordion (instead of every program's row).
func refreshMasksAccordionForProgram(programName string) {
	if acc, ok := shell().EditorTabs.MasksTab.Widgets["Accordion"].(*widget.Accordion); ok {
		refreshProgramEntityAccordionRow(acc, masksAccordionConfig(), programName)
	}
}

func setMasksForms() {}

func setMasksButtons() {
	et := shell().EditorTabs

	if uploadBtn, ok := et.MasksTab.Widgets["uploadButton"].(*widget.Button); ok {
		uploadBtn.OnTapped = func() {
			mask, ok := et.MasksTab.SelectedItem.(*models.Mask)
			if !ok || mask.Name == "" {
				editorErr(fmt.Errorf("select a mask first"))
				return
			}
			programName := et.MasksTab.ProgramSelector.Selected
			if programName == "" {
				editorErr(fmt.Errorf("select a program first"))
				return
			}

			fd := dialog.NewFileOpen(func(reader fyne.URIReadCloser, err error) {
				if err != nil {
					editorErr(err)
					return
				}
				if reader == nil {
					return
				}
				defer reader.Close()

				masksPath := config.GetMasksPath()
				programMaskDir := filepath.Join(masksPath, programName)
				if err := os.MkdirAll(programMaskDir, 0755); err != nil {
					editorErr(fmt.Errorf("failed to create mask directory: %w", err))
					return
				}

				destPath := filepath.Join(programMaskDir, mask.Name+config.PNG)
				destFile, err := os.Create(destPath)
				if err != nil {
					editorErr(fmt.Errorf("failed to create mask file: %w", err))
					return
				}
				defer destFile.Close()

				if _, err := io.Copy(destFile, reader); err != nil {
					editorErr(fmt.Errorf("failed to write mask image: %w", err))
					return
				}

				program, ok := getProgramForEditor(programName)
				if !ok {
					return
				}
				if err := program.MaskRepo().Set(mask.Name, mask); err != nil {
					editorRepoErr("save", "mask", mask.Name, err)
					return
				}
				shell().SetMaskImageMode(true)
				shell().UpdateMaskPreview(programName, mask.Name)
			}, activeWire.Window)
			fd.SetFilter(storage.NewExtensionFileFilter([]string{".png", ".jpg", ".jpeg", ".bmp"}))
			activeWire.AddDialogEscapeClose(fd, activeWire.Window)
			fd.Show()
		}
	}

	if removeBtn, ok := et.MasksTab.Widgets["removeImageButton"].(*widget.Button); ok {
		removeBtn.OnTapped = func() {
			mask, ok := et.MasksTab.SelectedItem.(*models.Mask)
			if !ok || mask.Name == "" {
				return
			}
			programName := et.MasksTab.ProgramSelector.Selected
			imgPath := filepath.Join(config.GetMasksPath(), programName, mask.Name+config.PNG)
			if err := os.Remove(imgPath); err != nil && !os.IsNotExist(err) {
				editorErr(fmt.Errorf("failed to remove mask image: %w", err))
				return
			}
			shell().SetMaskImageMode(false)
			shell().ClearMaskPreviewImage()
		}
	}
}

func renameMaskImage(programName, oldName, newName string) {
	oldPath := filepath.Join(config.GetMasksPath(), programName, oldName+config.PNG)
	newPath := filepath.Join(config.GetMasksPath(), programName, newName+config.PNG)
	if _, err := os.Stat(oldPath); err == nil {
		if err := os.Rename(oldPath, newPath); err != nil {
			editorRepoLog("rename file", "mask image", oldPath, err)
		}
	}
}
