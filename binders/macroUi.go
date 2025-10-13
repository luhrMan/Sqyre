package binders

import (
	"Squire/internal/assets"
	"Squire/internal/programs/macro"
	"Squire/internal/utils"
	"Squire/ui"
	"errors"
	"slices"
	"strconv"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/container"
	"fyne.io/fyne/v2/dialog"
	"fyne.io/fyne/v2/theme"
	"fyne.io/fyne/v2/widget"
	"github.com/go-vgo/robotgo"
)

func GetMacrosAsStringSlice() []string {
	keys := make([]string, len(GetMacros()))

	i := 0
	for _, k := range GetMacros() {
		keys[i] = k.Name
		i++
	}
	return keys
}

func SetMacroUi() {
	setMtabSettingsAndWidgets()
	setMacroSelect()
	for _, m := range GetMacros() {
		AddMacroTab(m)
	}
}

func AddMacroTab(m *macro.Macro) {
	mtabs := ui.GetUi().Mui.MTabs

	//check if already open. if it is, select it.
	t := container.NewTabItem(m.Name, ui.NewMacroTree(m))
	mtabs.AddTab(m.Name, t)
	utils.RegisterHotkey(m.Hotkey, m.HotkeyCallback())
}

func setMtabSettingsAndWidgets() {
	mtabs := ui.GetUi().Mui.MTabs

	mtabs.CreateTab = func() *container.TabItem {
		macros := GetMacros()

		c := len(macros)
		name := "New macro " + strconv.Itoa(c)
		for {
			if slices.Contains(macros, GetMacro(name)) {
				c++
				name = "New macro " + strconv.Itoa(c)
			} else {
				break
			}
		}
		AddMacro(name, 0)
		mtabs.BoundMacroListWidget.Refresh()
		return container.NewTabItem(
			name,
			ui.NewMacroTree(GetMacro(name)),
		)
	}
	mtabs.OnClosed = func(ti *container.TabItem) {
		utils.UnregisterHotkey(GetMacro(ti.Text).Hotkey)
		mtabs.BoundMacroListWidget.Refresh()
	}

	mtabs.OnUnselected = func(ti *container.TabItem) {
		//unbindAll()
		ti.Content.(*ui.MacroTree).UnselectAll()
	}
	mtabs.OnSelected = func(ti *container.TabItem) {
		m := GetMacro(ti.Text)
		mtabs.MacroNameEntry.SetText(m.Name)
		mtabs.BoundGlobalDelayEntry.SetText(strconv.Itoa(m.GlobalDelay))

		mtabs.MacroHotkeyEntry.SetText(utils.ReverseParseMacroHotkey(m.Hotkey))
	}

	mtabs.MacroHotkeyEntry.PlaceHolder = "ctrl+shift+1 or ctrl+1 or ctrl+a+1"
	saveHotkey := func() {
		mt := mtabs.SelectedTab()
		utils.UnregisterHotkey(mt.Macro.Hotkey)
		mt.Macro.Hotkey = utils.ParseMacroHotkey(mtabs.MacroHotkeyEntry.Text)
		utils.RegisterHotkey(mt.Macro.Hotkey, mt.Macro.HotkeyCallback())
	}
	mtabs.MacroHotkeyEntry.ActionItem = widget.NewButtonWithIcon("", theme.DocumentSaveIcon(), func() {
		saveHotkey()
	})
	mtabs.MacroHotkeyEntry.OnSubmitted = func(s string) {
		saveHotkey()
	}

	mtabs.MacroNameEntry.OnSubmitted = func(sub string) {
		if sub == "" {
			e := dialog.NewError(errors.New("macro name cannot be empty"), ui.GetUi().MainWindow)
			e.Show()
			return
		}
		for _, m := range GetMacros() {
			if m.Name == sub {
				dialog.ShowError(errors.New("macro name already exists"), ui.GetUi().MainWindow)
				return
			}
		}
		mt := mtabs.SelectedTab()
		mt.Macro.Name = sub
		mtabs.Selected().Text = sub
		mtabs.BoundMacroListWidget.Refresh()
		mtabs.Refresh()
	}
	mtabs.BoundGlobalDelayEntry.OnChanged = func(s string) {
		mt := mtabs.SelectedTab()
		gd, _ := strconv.Atoi(s)

		mt.Macro.GlobalDelay = gd
		robotgo.MouseSleep = gd
		robotgo.KeySleep = gd
	}
}

func setMacroSelect() {
	ui.GetUi().Mui.MacroSelectButton = widget.NewButtonWithIcon("",
		theme.FolderOpenIcon(),
		func() {
			title := "Open Macro"
			for _, w := range fyne.CurrentApp().Driver().AllWindows() {
				if w.Title() == title {
					w.RequestFocus()
					return
				}
			}
			w := fyne.CurrentApp().NewWindow(title)
			w.SetIcon(assets.AppIcon)
			ui.GetUi().Mui.MTabs.BoundMacroListWidget = widget.NewList(
				func() int {
					return len(GetMacros())
				},
				func() fyne.CanvasObject {
					return widget.NewLabel("template")
				},
				func(id widget.ListItemID, co fyne.CanvasObject) {
					k := GetMacrosAsStringSlice()
					label := co.(*widget.Label)
					slices.Sort(k)
					v := k[id]
					label.SetText(v)
					label.Importance = widget.MediumImportance
					for _, d := range ui.GetUi().Mui.MTabs.Items {
						if d.Text == v {
							label.Importance = widget.SuccessImportance
						}
					}
					label.Refresh()
				},
			)
			ui.GetUi().Mui.MTabs.BoundMacroListWidget.OnSelected =
				func(id widget.ListItemID) {
					k := GetMacrosAsStringSlice()
					slices.Sort(k)
					AddMacroTab(GetMacro(k[id]))
					ui.GetUi().Mui.MTabs.BoundMacroListWidget.RefreshItem(id)
					ui.GetUi().Mui.MTabs.BoundMacroListWidget.UnselectAll()
				}
			w.SetContent(
				container.NewAdaptiveGrid(1,
					ui.GetUi().Mui.MTabs.BoundMacroListWidget,
				),
			)
			w.Resize(fyne.NewSize(300, 500))
			w.Show()
		},
	)
}

// MACRO TREE
