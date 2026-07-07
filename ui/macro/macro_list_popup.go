package macro

import (
	macrologic "Sqyre/internal/macro"
	"Sqyre/internal/macrohotkey"
	"Sqyre/internal/models"
	"Sqyre/internal/models/repositories"
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
	ttwidget "github.com/dweymouth/fyne-tooltip/widget"
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

	searchEntry := custom_widgets.NewFormEntry()
	searchEntry.SetPlaceHolder("Search macros or tags…")

	openAllBtn := widget.NewButton("Open all", nil)
	openAllBtn.Disable()
	closeAllBtn := widget.NewButton("Close all", nil)
	closeAllBtn.Disable()

	syncOpenAllBtn := func() {
		query := strings.TrimSpace(searchEntry.Text)
		if query == "" || len(filtered) == 0 {
			openAllBtn.Disable()
		} else {
			openAllBtn.Enable()
		}
	}

	syncCloseAllBtn := func() {
		openTabs := openTabNames()
		for _, name := range filtered {
			if openTabs[name] {
				closeAllBtn.Enable()
				return
			}
		}
		closeAllBtn.Disable()
	}

	applyFilter := func() {
		query := strings.TrimSpace(searchEntry.Text)
		allKeys := repositories.MacroRepo().GetAllKeys()
		if query == "" {
			filtered = sortedMacroKeys(allKeys, sortBy, openTabNames())
			syncOpenAllBtn()
			syncCloseAllBtn()
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
		syncCloseAllBtn()
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
				container.NewHBox(
					widget.NewLabel("Template"),
					layout.NewSpacer(),
					widget.NewButtonWithIcon("", theme.CancelIcon(), nil),
					ttwidget.NewButtonWithIcon("", theme.InfoIcon(), nil),
					widget.NewButtonWithIcon("", theme.ContentCopyIcon(), nil),
					widget.NewButtonWithIcon("", theme.DeleteIcon(), nil),
				),
			)
		},
		func(id widget.ListItemID, co fyne.CanvasObject) {
			if id < 0 || id >= len(filtered) {
				return
			}
			name := filtered[id]
			nameRow := co.(*fyne.Container).Objects[0].(*fyne.Container)
			closeBtn := nameRow.Objects[2].(*widget.Button)
			tagsBtn := nameRow.Objects[3].(*ttwidget.Button)
			label := nameRow.Objects[0].(*widget.Label)
			duplicateBtn := nameRow.Objects[4].(*widget.Button)
			removeBtn := nameRow.Objects[5].(*widget.Button)

			label.SetText(name)
			label.Importance = widget.MediumImportance
			isOpen := openTabNames()[name]
			if isOpen {
				label.Importance = widget.SuccessImportance
			}
			label.Refresh()

			closeBtn.Importance = widget.LowImportance
			if isOpen {
				closeBtn.Enable()
				closeBtn.OnTapped = func() {
					CloseMacroTabs([]string{name})
					custom_widgets.RefreshListPreservingScroll(list)
					syncCloseAllBtn()
				}
			} else {
				closeBtn.Disable()
				closeBtn.OnTapped = nil
			}

			m, macroErr := repositories.MacroRepo().Get(name)
			if macroErr == nil {
				tagsBtn.SetToolTip(macroTagsListButtonTooltip(m))
			} else {
				tagsBtn.SetToolTip("Edit tags")
			}
			tagsBtn.Importance = widget.LowImportance
			tagsBtn.Enable()
			tagsBtn.OnTapped = func() {
				m, err := repositories.MacroRepo().Get(name)
				if err != nil {
					log.Printf("Error getting macro %s: %v", name, err)
					return
				}
				showMacroTagsEditorPopup(tagsBtn, m, func(*models.Macro) {
					custom_widgets.RefreshListPreservingScroll(list)
				})
			}
			tagsBtn.Refresh()

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
					macrohotkey.UnregisterMacroHotkey(m)
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
		syncCloseAllBtn()
	}

	closeAllBtn.OnTapped = func() {
		CloseMacroTabs(filtered)
		custom_widgets.RefreshListPreservingScroll(list)
		syncCloseAllBtn()
	}

	applyFilter()
	d.Mui.MTabs.BoundMacroListWidget = list

	top := container.NewBorder(nil, nil, nil, sortSelect, searchEntry)
	closeBtn := widget.NewButton("Close", nil)
	bottom := container.NewHBox(openAllBtn, closeAllBtn, layout.NewSpacer(), closeBtn)
	popUpContent := container.NewBorder(
		top,
		bottom,
		nil, nil,
		list,
	)
	popup = widget.NewModalPopUp(popUpContent, d.Window.Canvas())
	dlg := d.AddPopupEscapeClose(popup, d.Window)
	closeBtn.OnTapped = func() {
		hideMacroTagsEditorPopup()
		dlg.Hide()
	}
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
	dup, err := macrologic.CloneMacro(src)
	if err != nil {
		return nil, err
	}
	dup.Name = uniqueMacroCopyName(sourceName)
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
