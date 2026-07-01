package macro

import (
	"Sqyre/internal/models"
	"Sqyre/internal/models/actions"
	"Sqyre/internal/models/repositories"
	"Sqyre/internal/models/serialize"
	"Sqyre/ui/custom_widgets"
	"fmt"
	"log"
	"slices"
	"strings"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/container"
	"fyne.io/fyne/v2/layout"
	"fyne.io/fyne/v2/theme"
	"fyne.io/fyne/v2/widget"
)

type macroListSort int

const (
	macroSortNameAsc macroListSort = iota
	macroSortNameDesc
	macroSortOpenFirst

	macroListPopupWidth  float32 = 720
	macroListPopupHeight float32 = 900
)

var macroSortLabels = []string{"Name (A–Z)", "Name (Z–A)", "Open first"}

func macroListSortFromLabel(label string) macroListSort {
	for i, l := range macroSortLabels {
		if l == label {
			return macroListSort(i)
		}
	}
	return macroSortNameAsc
}

func showMacroListPopup(d WireDeps) {
	var popup *widget.PopUp
	var filtered []string
	sortBy := macroSortNameAsc

	openTabNames := func() map[string]bool {
		names := make(map[string]bool, len(d.Mui.MTabs.Items))
		for _, ti := range d.Mui.MTabs.Items {
			names[ti.Text] = true
		}
		return names
	}

	searchEntry := widget.NewEntry()
	searchEntry.SetPlaceHolder("Search macros or tags…")

	openAllBtn := widget.NewButton("Open all", nil)
	openAllBtn.Disable()

	syncOpenAllBtn := func() {
		query := strings.TrimSpace(searchEntry.Text)
		if query == "" || len(filtered) == 0 {
			openAllBtn.Disable()
		} else {
			openAllBtn.Enable()
		}
	}

	applyFilter := func() {
		query := strings.TrimSpace(searchEntry.Text)
		allKeys := repositories.MacroRepo().GetAllKeys()
		if query == "" {
			filtered = sortedMacroKeys(allKeys, sortBy, openTabNames())
			syncOpenAllBtn()
			return
		}
		filtered = nil
		for _, name := range allKeys {
			if macroMatchesSearch(name, query) {
				filtered = append(filtered, name)
			}
		}
		filtered = sortedMacroKeys(filtered, sortBy, openTabNames())
		syncOpenAllBtn()
	}

	sortSelect := widget.NewSelect(macroSortLabels, nil)
	sortSelect.SetSelected(macroSortLabels[0])

	var list *widget.List
	list = widget.NewList(
		func() int {
			return len(filtered)
		},
		func() fyne.CanvasObject {
			return container.NewPadded(
				container.NewVBox(
					container.NewHBox(
						widget.NewLabel("Template"),
						layout.NewSpacer(),
						widget.NewButtonWithIcon("", theme.ContentCopyIcon(), nil),
						widget.NewButtonWithIcon("", theme.DeleteIcon(), nil),
					),
					widget.NewLabel(""),
				),
			)
		},
		func(id widget.ListItemID, co fyne.CanvasObject) {
			if id < 0 || id >= len(filtered) {
				return
			}
			name := filtered[id]
			c := co.(*fyne.Container).Objects[0].(*fyne.Container)
			nameRow := c.Objects[0].(*fyne.Container)
			tagsLabel := c.Objects[1].(*widget.Label)
			label := nameRow.Objects[0].(*widget.Label)
			duplicateBtn := nameRow.Objects[2].(*widget.Button)
			removeBtn := nameRow.Objects[3].(*widget.Button)

			label.SetText(name)
			label.Importance = widget.MediumImportance
			if openTabNames()[name] {
				label.Importance = widget.SuccessImportance
			}
			label.Refresh()

			if m, err := repositories.MacroRepo().Get(name); err == nil {
				if tagText := formatMacroListTags(m); tagText != "" {
					tagsLabel.SetText(tagText)
					tagsLabel.Importance = widget.MediumImportance
					tagsLabel.Wrapping = fyne.TextWrapWord
					tagsLabel.Show()
				} else {
					tagsLabel.SetText("")
					tagsLabel.Hide()
				}
			} else {
				tagsLabel.Hide()
			}
			tagsLabel.Refresh()

			duplicateBtn.Importance = widget.LowImportance
			duplicateBtn.OnTapped = func() {
				dup, err := duplicateMacro(name)
				if err != nil {
					log.Printf("duplicate macro %s: %v", name, err)
					d.ShowErrorWithEscape(err, d.Window)
					return
				}
				if err := repositories.MacroRepo().Set(dup.Name, dup); err != nil {
					log.Printf("save duplicated macro %s: %v", dup.Name, err)
					d.ShowErrorWithEscape(err, d.Window)
					return
				}
				applyFilter()
				custom_widgets.RefreshListPreservingScroll(list)
				AddMacroTab(dup)
			}

			removeBtn.Importance = widget.LowImportance
			removeBtn.OnTapped = func() {
				m, err := repositories.MacroRepo().Get(name)
				if err != nil {
					log.Printf("Error getting macro %s: %v", name, err)
					return
				}
				d.ShowConfirmWithEscape("Delete Macro", "Are you sure you want to delete this macro?", func(ok bool) {
					if !ok {
						return
					}
					if err := repositories.MacroRepo().Delete(name); err != nil {
						log.Printf("Error deleting macro %s: %v", name, err)
						return
					}
					for _, ti := range d.Mui.MTabs.Items {
						if m.Name == ti.Text {
							d.Mui.MTabs.Remove(ti)
						}
					}
					d.Mui.MTabs.Refresh()
					applyFilter()
					custom_widgets.RefreshListPreservingScroll(list)
				}, d.Window)
			}
		},
	)

	list.OnSelected = func(id widget.ListItemID) {
		if id < 0 || id >= len(filtered) {
			return
		}
		m, err := repositories.MacroRepo().Get(filtered[id])
		if err != nil {
			log.Printf("Error getting macro %s: %v", filtered[id], err)
			return
		}
		AddMacroTab(m)
		list.RefreshItem(id)
		list.UnselectAll()
	}

	searchDebounce := custom_widgets.NewDebouncer(custom_widgets.DefaultSearchDebounce)
	searchEntry.OnChanged = func(string) {
		searchDebounce.Call(func() {
			applyFilter()
			custom_widgets.RefreshListPreservingScroll(list)
		})
	}
	sortSelect.OnChanged = func(label string) {
		sortBy = macroListSortFromLabel(label)
		applyFilter()
		custom_widgets.RefreshListPreservingScroll(list)
	}

	openAllBtn.OnTapped = func() {
		OpenMacroTabs(filtered)
		custom_widgets.RefreshListPreservingScroll(list)
	}

	applyFilter()
	d.Mui.MTabs.BoundMacroListWidget = list

	top := container.NewBorder(nil, nil, nil, sortSelect, searchEntry)
	closeBtn := widget.NewButton("Close", nil)
	bottom := container.NewHBox(openAllBtn, layout.NewSpacer(), closeBtn)
	popUpContent := container.NewBorder(
		top,
		bottom,
		nil, nil,
		list,
	)
	popup = widget.NewModalPopUp(popUpContent, d.Window.Canvas())
	dlg := d.AddPopupEscapeClose(popup, d.Window)
	closeBtn.OnTapped = func() { dlg.Hide() }
	dlg.Resize(fyne.NewSize(macroListPopupWidth, macroListPopupHeight))
	dlg.Show()
}

func sortedMacroKeys(keys []string, sortBy macroListSort, openTabs map[string]bool) []string {
	out := slices.Clone(keys)
	switch sortBy {
	case macroSortNameDesc:
		slices.SortFunc(out, func(a, b string) int {
			return strings.Compare(b, a)
		})
	case macroSortOpenFirst:
		slices.SortFunc(out, func(a, b string) int {
			aOpen, bOpen := openTabs[a], openTabs[b]
			if aOpen != bOpen {
				if aOpen {
					return -1
				}
				return 1
			}
			return strings.Compare(a, b)
		})
	default:
		slices.Sort(out)
	}
	return out
}

func duplicateMacro(sourceName string) (*models.Macro, error) {
	src, err := repositories.MacroRepo().Get(sourceName)
	if err != nil {
		return nil, err
	}
	dup, err := cloneMacro(src)
	if err != nil {
		return nil, err
	}
	dup.Name = uniqueMacroCopyName(sourceName)
	return dup, nil
}

func cloneMacro(src *models.Macro) (*models.Macro, error) {
	if src == nil || src.Root == nil {
		return nil, fmt.Errorf("cannot clone macro")
	}
	rootMap, err := serialize.ActionToMap(src.Root)
	if err != nil {
		return nil, err
	}
	rootAction, err := serialize.ViperSerializer.CreateActionFromMap(rootMap, nil)
	if err != nil {
		return nil, err
	}
	root, ok := rootAction.(*actions.Loop)
	if !ok {
		return nil, fmt.Errorf("macro root is not a loop")
	}
	decls := make([]models.VariableDecl, len(src.VariableDecls))
	copy(decls, src.VariableDecls)
	tags := append([]string(nil), src.Tags...)

	dup := models.NewMacro("", src.GlobalDelay, nil)
	dup.Root = root
	dup.VariableDecls = decls
	dup.Tags = tags
	dup.InitRuntimeVariables()
	return dup, nil
}

func uniqueMacroCopyName(srcName string) string {
	base := srcName + " copy"
	if _, err := repositories.MacroRepo().Get(base); err != nil {
		return base
	}
	for i := 2; ; i++ {
		candidate := fmt.Sprintf("%s copy %d", srcName, i)
		if _, err := repositories.MacroRepo().Get(candidate); err != nil {
			return candidate
		}
	}
}
