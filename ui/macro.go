package ui

import (
	"errors"
	"log"
	"sort"

	"fyne.io/fyne/v2/data/binding"
	"fyne.io/fyne/v2/dialog"
	"fyne.io/fyne/v2/widget"
	"github.com/go-vgo/robotgo"
)

func (u *Ui) constructMacroSettings() {
	boundLocX = binding.BindInt(&locX)
	boundLocY = binding.BindInt(&locY)
	boundLocXLabel = widget.NewLabelWithData(binding.IntToString(boundLocX))
	boundLocYLabel = widget.NewLabelWithData(binding.IntToString(boundLocY))

	boundMacro := binding.NewUntyped()
	boundMacro.Set(u.p.Macros[0])
	u.ms.boundMacroList = binding.BindStringList(&macroList)
	for _, m := range u.p.Macros {
		u.ms.boundMacroList.Append(m.Name)
	}
	u.ms.boundMacroList.AddListener(binding.NewDataListener(func() {
		ml, err := u.ms.boundMacroList.Get()
		if err != nil {
			log.Println(err)
			return
		}
		sort.Strings(ml)
	}))
	u.ms.boundMacroName = binding.BindString(&macroName)
	u.ms.boundMacroNameEntry = widget.NewEntryWithData(u.ms.boundMacroName)
	u.ms.boundMacroNameEntry.OnSubmitted = func(string) {
		t, err := u.GetMacroTabMacroTree()
		if err != nil {
			log.Println(err)
			return
		}
		for _, m := range u.p.Macros {
			if m.Name == macroName {
				dialog.ShowError(errors.New("macro name already exists"), u.win)
				return
			}
		}
		delete(u.mtMap, t.Macro.Name)
		u.ms.boundMacroList.Remove(t.Macro.Name)
		u.SetMacroTreeMapKeyValue(macroName, t)
		// u.mtMap[macroName] = t
		t.Macro.Name = macroName
		u.dt.Selected().Text = macroName
		u.ms.boundMacroList.Append(macroName)

		u.dt.Refresh()
	}
	macroHotkey = []string{"1", "2", "3"}
	u.ms.boundMacroHotkey = binding.BindStringList(&macroHotkey)
	u.ms.macroHotkeySelect1 = &widget.Select{Options: []string{"ctrl"}}
	u.ms.macroHotkeySelect2 = &widget.Select{Options: []string{"", "shift"}}
	u.ms.macroHotkeySelect3 = &widget.Select{Options: []string{"1", "2", "3", "4", "5"}}

	u.ms.macroHotkeySelect1.SetSelectedIndex(0)

	u.ms.boundGlobalDelay = binding.BindInt(&globalDelay)
	u.ms.boundGlobalDelayEntry = widget.NewEntryWithData(binding.IntToString(u.ms.boundGlobalDelay))
	u.ms.boundGlobalDelay.AddListener(binding.NewDataListener(func() {
		t, err := u.GetMacroTabMacroTree()
		if err != nil {
			log.Println(err)
			return
		}
		t.Macro.GlobalDelay = globalDelay
		robotgo.MouseSleep = globalDelay
		robotgo.KeySleep = globalDelay
	}))

}
