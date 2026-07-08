package editor

import (
	"Sqyre/internal/config"
	"Sqyre/internal/models"
	"Sqyre/internal/models/repositories"
	"Sqyre/internal/services"
	"Sqyre/ui/completionentry"
	"Sqyre/ui/custom_widgets"
	"Sqyre/ui/dialogs"
	"errors"
	"fmt"
	"io"
	"os"
	"path/filepath"
	"slices"
	"strconv"
	"strings"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/container"
	"fyne.io/fyne/v2/dialog"
	"fyne.io/fyne/v2/storage"
	"fyne.io/fyne/v2/theme"
	"fyne.io/fyne/v2/widget"
)

type createDialogContext struct {
	draftItem *models.Item
}

func copyTabWidgetsToDialog(src, dst map[string]fyne.CanvasObject, keys ...string) {
	for _, k := range keys {
		if src[k] == nil || dst[k] == nil {
			continue
		}
		text := custom_widgets.EntryText(src[k])
		switch d := dst[k].(type) {
		case *widget.Entry:
			d.SetText(text)
		case *custom_widgets.VarEntry:
			custom_widgets.SetEntryText(d, text)
		case *custom_widgets.VarEntryField:
			custom_widgets.SetEntryText(d, text)
		case *custom_widgets.Incrementer:
			v, err := strconv.Atoi(strings.TrimSpace(text))
			if err != nil {
				v = 0
			}
			d.SetValue(v)
		case *widget.RadioGroup:
			if rg, ok := src[k].(*widget.RadioGroup); ok {
				sel := rg.Selected
				if sel == "" {
					sel = "Rectangle"
				}
				d.SetSelected(sel)
			}
		case *widget.Check:
			if c, ok := src[k].(*widget.Check); ok {
				d.SetChecked(c.Checked)
			}
		}
	}
}

func wireCreateItemDialog(w map[string]fyne.CanvasObject, programSelector *widget.Select, ctx *createDialogContext) {
	if ctx.draftItem == nil {
		ctx.draftItem = &models.Item{}
	}

	wireItemTagHandlers(w, programSelector, ctx.draftItem)
	wireItemMaskHandlers(w, programSelector, ctx.draftItem)

	if editor, ok := w["iconVariantEditor"].(*custom_widgets.IconVariantEditor); ok {
		syncIconEditor := func() {
			programName := programSelector.Selected
			itemName := w["Name"].(*widget.Entry).Text
			if programName == "" || itemName == "" {
				return
			}
			iconService := services.IconVariantServiceInstance()
			baseName := iconService.GetBaseItemName(itemName)
			editor.SetProgramAndItem(programName, baseName)
		}
		w["Name"].(*widget.Entry).OnChanged = func(string) { syncIconEditor() }
		programSelector.OnChanged = func(string) { syncIconEditor() }
		syncIconEditor()
	}
}

func wireItemTagHandlers(w map[string]fyne.CanvasObject, programSelector *widget.Select, item *models.Item) {
	tagEntry, ok := w["tagEntry"].(*completionentry.CompletionEntry)
	if !ok {
		return
	}

	updateTags := func() {}
	updateTags = func() {
		tagsContainer, ok := w["Tags"].(*fyne.Container)
		if !ok {
			return
		}
		tagsContainer.Objects = nil
		for _, tag := range item.Tags {
			tagLabel := widget.NewLabel(tag)
			tagToRemove := tag
			removeButton := widget.NewButtonWithIcon("", theme.DeleteIcon(), func() {
				newTags := []string{}
				for _, t := range item.Tags {
					if t != tagToRemove {
						newTags = append(newTags, t)
					}
				}
				item.Tags = newTags
				updateTags()
			})
			removeButton.Importance = widget.LowImportance
			tagsContainer.Add(wrapTagChip(container.NewHBox(tagLabel, removeButton)))
		}
		tagsContainer.Refresh()
	}

	submitTag := func() {
		tagText := strings.TrimSpace(tagEntry.Text)
		tagEntry.HideCompletion()
		if tagText == "" {
			return
		}
		if slices.Contains(item.Tags, tagText) {
			return
		}
		item.Tags = append(item.Tags, tagText)
		tagEntry.SetText("")
		updateTags()
	}

	tagEntry.OnChanged = func(text string) {
		if text == "" {
			tagEntry.HideCompletion()
			return
		}
		matching := tagCompletionOptions(programSelector.Selected, text, item, 10)
		if len(matching) == 0 {
			tagEntry.HideCompletion()
			return
		}
		tagEntry.SetOptions(matching)
		tagEntry.ShowCompletion()
	}
	tagEntry.OnSubmitted = func(string) { submitTag() }
	if tagSubmitButton, ok := w["tagSubmitButton"].(*widget.Button); ok {
		tagSubmitButton.OnTapped = func() {
			if strings.TrimSpace(tagEntry.Text) != "" {
				submitTag()
				return
			}
			programTags := tagCompletionOptions(programSelector.Selected, "", item, 0)
			if len(programTags) == 0 {
				tagEntry.HideCompletion()
				return
			}
			tagEntry.SetOptions(programTags)
			tagEntry.ShowCompletion()
		}
	}
	updateTags()
}

func wireItemMaskHandlers(w map[string]fyne.CanvasObject, programSelector *widget.Select, item *models.Item) {
	updateMaskUI := func(maskName string) {
		maskLabel, _ := w["maskLabel"].(*widget.Label)
		maskDetailsLabel, _ := w["maskDetailsLabel"].(*widget.Label)
		if maskName == "" {
			if maskLabel != nil {
				maskLabel.SetText("None")
			}
			if maskDetailsLabel != nil {
				maskDetailsLabel.SetText("")
			}
			return
		}
		if maskLabel != nil {
			maskLabel.SetText(maskName)
		}
		if maskDetailsLabel == nil {
			return
		}
		prog := programSelector.Selected
		program, err := repositories.ProgramRepo().Get(prog)
		if err != nil {
			maskDetailsLabel.SetText("")
			return
		}
		mask, err := ProgramMaskRepo(program).Get(maskName)
		if err != nil {
			maskDetailsLabel.SetText("")
			return
		}
		if HasMaskImage(prog, maskName) {
			maskDetailsLabel.SetText("Image mask")
			return
		}
		center := fmt.Sprintf("X: %s%%  Y: %s%%", mask.CenterX, mask.CenterY)
		var equation string
		switch mask.Shape {
		case "circle":
			equation = fmt.Sprintf("π × %s²", mask.Radius)
		default:
			equation = fmt.Sprintf("%s × %s", mask.Base, mask.Height)
		}
		maskDetailsLabel.SetText(fmt.Sprintf("%s  •  %s", center, equation))
	}

	if btn, ok := w["maskSelectButton"].(*widget.Button); ok {
		btn.OnTapped = func() {
			showMaskSelectionPopupForItem(func(maskName string) {
				item.Mask = maskName
				updateMaskUI(maskName)
			})
		}
	}
	if btn, ok := w["maskClearButton"].(*widget.Button); ok {
		btn.OnTapped = func() {
			item.Mask = ""
			updateMaskUI("")
		}
	}
	updateMaskUI(item.Mask)
}

func showMaskSelectionPopupForItem(onSelect func(maskName string)) {
	var popup *widget.PopUp
	var hide func()
	acc := widget.NewAccordion()
	for _, p := range repositories.ProgramRepo().GetAllSortedByName() {
		pName := p.Name
		allKeys := ProgramMaskRepo(p).GetAllKeys()
		filtered := append([]string(nil), allKeys...)
		sortMaskKeysByDisplayName(p, filtered)

		searchDebounce := custom_widgets.NewDebouncer(custom_widgets.DefaultSearchDebounce)
		searchbar := custom_widgets.NewFormEntry()
		searchbar.PlaceHolder = "Search masks"
		maskList := widget.NewList(
			func() int { return len(filtered) },
			func() fyne.CanvasObject { return widget.NewLabel("template") },
			func(id widget.ListItemID, co fyne.CanvasObject) {
				if id < len(filtered) {
					co.(*widget.Label).SetText(filtered[id])
				}
			},
		)
		maskList.OnSelected = func(id widget.ListItemID) {
			if id >= len(filtered) {
				return
			}
			onSelect(filtered[id])
			hide()
		}
		searchbar.OnChanged = func(s string) {
			searchDebounce.Call(func() {
				defaultList := ProgramMaskRepo(p).GetAllKeys()
				if s == "" {
					filtered = defaultList
				} else {
					next := make([]string, 0, len(defaultList))
					sLower := strings.ToLower(s)
					for _, k := range defaultList {
						if strings.Contains(strings.ToLower(k), sLower) {
							next = append(next, k)
						}
					}
					filtered = next
				}
				sortMaskKeysByDisplayName(p, filtered)
				custom_widgets.RefreshListPreservingScroll(maskList)
			})
		}
		acc.Append(widget.NewAccordionItem(
			fmt.Sprintf("%s (%d)", pName, len(allKeys)),
			container.NewBorder(searchbar, nil, nil, nil, maskList),
		))
	}
	closeButton := widget.NewButton("Close", func() { hide() })
	popUpContent := container.NewBorder(closeButton, nil, nil, nil, acc)
	popup = widget.NewModalPopUp(popUpContent, activeWire.Window.Canvas())
	dlg := dialogs.AddPopupEscapeClose(popup, activeWire.Window)
	hide = dlg.Hide
	dlg.Resize(fyne.NewSize(400, 500))
	dlg.Show()
}

func wireCreateMaskDialog(w map[string]fyne.CanvasObject, programSelector *widget.Select, previewPanel *editorPreviewPanel, refreshBtn *widget.Button) {
	if refreshBtn != nil {
		refreshBtn.OnTapped = func() {
			p := programSelector.Selected
			n := w["Name"].(*widget.Entry).Text
			if p == "" || n == "" {
				return
			}
			if HasMaskImage(p, n) {
				updateMaskPreviewPanel(previewPanel, p, n)
			} else if previewPanel != nil {
				previewPanel.clear()
			}
		}
	}

	if uploadBtn, ok := w["uploadButton"].(*widget.Button); ok {
		uploadBtn.OnTapped = func() {
			maskName := w["Name"].(*widget.Entry).Text
			if maskName == "" {
				dialogs.ShowErrorWithEscape(errors.New("enter a mask name first"), activeWire.Window)
				return
			}
			programName := programSelector.Selected
			if programName == "" {
				dialogs.ShowErrorWithEscape(errors.New("select a program first"), activeWire.Window)
				return
			}
			fd := dialog.NewFileOpen(func(reader fyne.URIReadCloser, err error) {
				if err != nil {
					dialogs.ShowErrorWithEscape(err, activeWire.Window)
					return
				}
				if reader == nil {
					return
				}
				defer reader.Close()
				masksPath := config.GetMasksPath()
				programMaskDir := filepath.Join(masksPath, programName)
				if err := os.MkdirAll(programMaskDir, 0755); err != nil {
					dialogs.ShowErrorWithEscape(fmt.Errorf("failed to create mask directory: %v", err), activeWire.Window)
					return
				}
				destPath := filepath.Join(programMaskDir, maskName+config.PNG)
				destFile, err := os.Create(destPath)
				if err != nil {
					dialogs.ShowErrorWithEscape(fmt.Errorf("failed to create mask file: %v", err), activeWire.Window)
					return
				}
				defer destFile.Close()
				if _, err := io.Copy(destFile, reader); err != nil {
					dialogs.ShowErrorWithEscape(fmt.Errorf("failed to write mask image: %v", err), activeWire.Window)
					return
				}
				setMaskImageModeOnWidgets(w, true)
				updateMaskPreviewPanel(previewPanel, programName, maskName)
			}, activeWire.Window)
			fd.SetFilter(storage.NewExtensionFileFilter([]string{".png", ".jpg", ".jpeg", ".bmp"}))
			dialogs.AddDialogEscapeClose(fd, activeWire.Window)
			fd.Show()
		}
	}

	if removeBtn, ok := w["removeImageButton"].(*widget.Button); ok {
		removeBtn.OnTapped = func() {
			maskName := w["Name"].(*widget.Entry).Text
			programName := programSelector.Selected
			if maskName == "" || programName == "" {
				return
			}
			imgPath := filepath.Join(config.GetMasksPath(), programName, maskName+config.PNG)
			if err := os.Remove(imgPath); err != nil && !os.IsNotExist(err) {
				dialogs.ShowErrorWithEscape(fmt.Errorf("failed to remove mask image: %v", err), activeWire.Window)
				return
			}
			setMaskImageModeOnWidgets(w, false)
			if previewPanel != nil {
				previewPanel.clear()
			}
		}
	}
}

func readMaskShapeFromWidgets(w map[string]fyne.CanvasObject) string {
	sel := w["shapeSelect"].(*widget.RadioGroup).Selected
	switch sel {
	case "Circle":
		return "circle"
	default:
		return "rectangle"
	}
}

func maskFromWidgets(w map[string]fyne.CanvasObject) *models.Mask {
	m := &models.Mask{
		Name:    w["Name"].(*widget.Entry).Text,
		Shape:   readMaskShapeFromWidgets(w),
		CenterX: custom_widgets.EntryText(w["CenterX"]),
		CenterY: custom_widgets.EntryText(w["CenterY"]),
		Base:    custom_widgets.EntryText(w["Base"]),
		Height:  custom_widgets.EntryText(w["Height"]),
		Radius:  custom_widgets.EntryText(w["Radius"]),
	}
	if inv, ok := w["Inverse"].(*widget.Check); ok {
		m.Inverse = inv.Checked
	}
	return m
}
