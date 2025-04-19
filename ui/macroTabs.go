package ui

import (
	"Squire/internal/config"
	"Squire/internal/programs"
	"Squire/internal/utils"
	"errors"
	"log"
	"slices"
	"strconv"

	"fyne.io/fyne/v2/container"
	"fyne.io/fyne/v2/dialog"
	"fyne.io/fyne/v2/theme"
	"fyne.io/fyne/v2/widget"
	"github.com/go-vgo/robotgo"
)

type macroTabs struct {
	container.DocTabs
	isExecuting widget.Activity

	boundMacroListWidget  *widget.List
	macroNameEntry        *widget.Entry
	boundGlobalDelayEntry *widget.Entry
	macroHotkeyEntry      *widget.Entry
}

func NewMacroTabs() *macroTabs {
	t := &macroTabs{
		boundMacroListWidget:  &widget.List{},
		macroNameEntry:        widget.NewEntry(),
		boundGlobalDelayEntry: widget.NewEntry(),
		macroHotkeyEntry:      widget.NewEntry(),
	}
	t.ExtendBaseWidget(t)

	return t
}

func (mtabs *macroTabs) constructTabs() {
	mtabs.setMtabSettingsAndWidgets()
	for _, m := range programs.GetPrograms().GetProgram(config.DarkAndDarker).Macros {
		mtabs.addTab(m.Name)
	}
}

func (mtabs *macroTabs) selectedTab() (*MacroTree, error) {
	if mtabs == nil || mtabs.Selected() == nil || mtabs.Selected().Text == "" {
		return nil, errors.New("no selected tab")
	}
	return mtabs.Selected().Content.(*MacroTree), nil
}

func (mtabs *macroTabs) addTab(name string) {
	//check if already open. if it is, select it.
	for _, d := range mtabs.Items {
		if d.Text == name {
			log.Println("macro already open")
			mtabs.Select(d)
			return
		}
	}
	m := ui.p.GetMacroByName(name)
	m.RegisterHotkey()
	mtabs.Append(container.NewTabItem(name, NewMacroTree(m)))
}
func (mtabs *macroTabs) setMtabSettingsAndWidgets() {
	mtabs.CreateTab = func() *container.TabItem {
		macros := programs.CurrentProgram().Macros

		c := len(macros)
		name := "New macro " + strconv.Itoa(c)
		for {
			if slices.Contains(macros, ui.p.GetMacroByName(name)) {
				c++
				name = "New macro " + strconv.Itoa(c)
			} else {
				break
			}
		}
		ui.p.AddMacro(name, 0)
		mtabs.boundMacroListWidget.Refresh()
		return container.NewTabItem(
			name,
			NewMacroTree(ui.p.GetMacroByName(name)),
		)
	}
	mtabs.OnClosed = func(ti *container.TabItem) {
		ui.p.GetMacroByName(ti.Text).UnregisterHotkey()
		mtabs.boundMacroListWidget.Refresh()
	}
	mtabs.OnSelected = func(ti *container.TabItem) {
		unbindAll()
		m := ui.p.GetMacroByName(ti.Text)
		mtabs.macroNameEntry.SetText(m.Name)
		mtabs.boundGlobalDelayEntry.SetText(strconv.Itoa(m.GlobalDelay))

		mtabs.macroHotkeyEntry.SetText(utils.ReverseParseMacroHotkey(m.Hotkey))
	}

	mtabs.macroHotkeyEntry.PlaceHolder = "ctrl+shift+1 or ctrl+1 or ctrl+a+1"
	saveHotkey := func() {
		mt, err := mtabs.selectedTab()
		if err != nil {
			log.Println(err)
			return
		}
		mt.Macro.UnregisterHotkey()
		mt.Macro.Hotkey = utils.ParseMacroHotkey(mtabs.macroHotkeyEntry.Text)
		mt.Macro.RegisterHotkey()
	}
	mtabs.macroHotkeyEntry.ActionItem = widget.NewButtonWithIcon("", theme.DocumentSaveIcon(), func() {
		saveHotkey()
	})
	mtabs.macroHotkeyEntry.OnSubmitted = func(s string) {
		saveHotkey()
	}

	mtabs.macroNameEntry.OnSubmitted = func(sub string) {
		if sub == "" {
			e := dialog.NewError(errors.New("macro name cannot be empty"), ui.win)
			e.Show()
			return
		}
		for _, m := range ui.p.Macros {
			if m.Name == sub {
				dialog.ShowError(errors.New("macro name already exists"), ui.win)
				return
			}
		}
		mt, err := mtabs.selectedTab()
		if err != nil {
			log.Println(err)
			return
		}

		mt.Macro.Name = sub
		mtabs.Selected().Text = sub
		mtabs.boundMacroListWidget.Refresh()
		mtabs.Refresh()
	}
	mtabs.boundGlobalDelayEntry.OnChanged = func(s string) {
		mt, err := mtabs.selectedTab()
		if err != nil {
			log.Println(err)
			return
		}
		gd, _ := strconv.Atoi(s)

		mt.Macro.GlobalDelay = gd
		robotgo.MouseSleep = gd
		robotgo.KeySleep = gd
	}
}
