package editor

import (
	"fmt"
	"log"
	"strings"

	"Sqyre/internal/models"
	"Sqyre/internal/models/repositories"
	"Sqyre/ui/custom_widgets"
	"Sqyre/ui/dialogs"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/container"
	"fyne.io/fyne/v2/widget"
)

// updateMaskDisplay updates the mask label and details label on the Items tab.
func updateMaskDisplay(maskName string) {
	it := shell().EditorTabs.ItemsTab.Widgets
	maskLabel, _ := it["maskLabel"].(*widget.Label)
	maskDetailsLabel, _ := it["maskDetailsLabel"].(*widget.Label)

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

	prog := shell().EditorTabs.ItemsTab.ProgramSelector.Selected
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

// showMaskSelectionPopup displays a modal popup with mask accordions for the user to select a mask.
func showMaskSelectionPopup() {
	var popup *widget.PopUp
	var hide func()

	acc := widget.NewAccordion()
	for _, p := range repositories.ProgramRepo().GetAllSortedByName() {
		programName := p.Name
		allKeys := ProgramMaskRepo(p).GetAllKeys()
		filtered := make([]string, len(allKeys))
		copy(filtered, allKeys)
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
			maskName := filtered[id]

			if v, ok := shell().EditorTabs.ItemsTab.SelectedItem.(*models.Item); ok {
				v.Mask = maskName

				prog := shell().EditorTabs.ItemsTab.ProgramSelector.Selected
				program, err := repositories.ProgramRepo().Get(prog)
				if err != nil {
					log.Printf("Error getting program %s: %v", prog, err)
					return
				}
				if err := ProgramItemRepo(program).Set(v.Name, v); err != nil {
					log.Printf("Error saving item %s: %v", v.Name, err)
					return
				}

				updateMaskDisplay(maskName)
			}
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
			fmt.Sprintf("%s (%d)", programName, len(allKeys)),
			container.NewBorder(searchbar, nil, nil, nil, maskList),
		))
	}

	closeButton := widget.NewButton("Close", func() { hide() })

	popUpContent := container.NewBorder(
		closeButton, nil, nil, nil,
		acc,
	)
	popup = widget.NewModalPopUp(popUpContent, activeWire.Window.Canvas())
	dlg := dialogs.AddPopupEscapeClose(popup, activeWire.Window)
	hide = dlg.Hide
	dlg.Resize(fyne.NewSize(400, 500))
	dlg.Show()
}

// setMaskSelectionButtons wires up the mask select and clear buttons on the Items tab.
func setMaskSelectionButtons() {
	it := shell().EditorTabs.ItemsTab.Widgets

	if btn, ok := it["maskSelectButton"].(*widget.Button); ok {
		btn.OnTapped = func() {
			if v, ok := shell().EditorTabs.ItemsTab.SelectedItem.(*models.Item); ok {
				if v.Name == "" {
					return
				}
			}
			showMaskSelectionPopup()
		}
	}

	if btn, ok := it["maskClearButton"].(*widget.Button); ok {
		btn.OnTapped = func() {
			if v, ok := shell().EditorTabs.ItemsTab.SelectedItem.(*models.Item); ok {
				v.Mask = ""

				prog := shell().EditorTabs.ItemsTab.ProgramSelector.Selected
				program, err := repositories.ProgramRepo().Get(prog)
				if err != nil {
					log.Printf("Error getting program %s: %v", prog, err)
					return
				}
				if err := ProgramItemRepo(program).Set(v.Name, v); err != nil {
					log.Printf("Error saving item %s: %v", v.Name, err)
					return
				}

				updateMaskDisplay("")
			}
		}
	}
}
