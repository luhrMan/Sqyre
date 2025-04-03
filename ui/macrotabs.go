package ui

import (
	"Squire/internal/config"
	"Squire/internal/programs"
	"Squire/internal/programs/macro"
	"errors"
	"log"

	"fyne.io/fyne/v2/container"
	"fyne.io/fyne/v2/widget"
)

func (u *Ui) createDocTabs() {
	u.dt = container.NewDocTabs()

	u.dt.OnClosed = func(ti *container.TabItem) {
		delete(u.mtMap, ti.Text)
		ReRegisterMacroHotkeys()
	}
	u.dt.OnSelected = func(ti *container.TabItem) {
		mt, err := u.selectedMacroTab()
		if err != nil {
			log.Println(err)
			return
		}
		if u.at == nil {
			return
		}
		u.ms.boundGlobalDelay.Set(mt.Macro.GlobalDelay)
		u.ms.boundMacroName.Set(mt.Macro.Name)
		if len(mt.Macro.Hotkey) < 3 {
			mt.Macro.Hotkey = []string{"ctrl", "shift", "1"}
		}
		u.ms.boundMacroHotkey.Set(mt.Macro.Hotkey)

		v, _ := u.ms.boundMacroHotkey.Get()
		log.Println("boundMacroHotkey set from:", mt.Macro.Name)
		log.Println("boundMacroHotkey set to:", v)
		u.ms.macroHotkeySelect1.SetSelected(mt.Macro.Hotkey[0])
		u.ms.macroHotkeySelect2.SetSelected(mt.Macro.Hotkey[1])
		u.ms.macroHotkeySelect3.SetSelected(mt.Macro.Hotkey[2])
	}

	u.dt.Items = append(u.dt.Items, container.NewTabItem("", container.NewBorder(nil, nil, nil, nil)))
	u.dt.SelectIndex(0)

	for _, m := range programs.GetPrograms().GetProgram(config.DarkAndDarker).Macros {
		u.addMacroDocTab(m)
	}

	u.dt.RemoveIndex(0)
	u.dt.SelectIndex(0)

}

func (u *Ui) selectedMacroTab() (*MacroTree, error) {
	if u.dt == nil || u.dt.Selected() == nil {
		return nil, errors.New("no selected tab")
	}
	macroTree, exists := u.mtMap[u.dt.Selected().Text]
	if !exists {
		return nil, errors.New("selected tab does not have a corresponding MacroTree")
	}

	return macroTree, nil
}

func (u *Ui) addMacroDocTab(macro *macro.Macro) {
	if _, ok := u.mtMap[macro.Name]; ok {
		log.Println("macro is already open")
		for _, d := range u.dt.Items {
			if d.Text == macro.Name {
				u.dt.Select(d)
			}
		}
		return
	}
	u.AddMacroTree(macro.Name, &MacroTree{Macro: macro, Tree: &widget.Tree{}})
	mt := u.mtMap[macro.Name]

	mt.createTree()

	t := container.NewTabItem(macro.Name, mt.Tree)
	u.dt.Append(t)
	u.dt.Select(t)
	mt.updateTreeOnselect()
	ReRegisterMacroHotkeys()
	mt.Tree.Refresh()
}
