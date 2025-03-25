package ui

import (
	"Squire/internal/config"
	"Squire/internal/programs"
	"Squire/internal/programs/macro"
	"errors"

	"fyne.io/fyne/v2/container"
	"fyne.io/fyne/v2/widget"
)

func (u *Ui) createDocTabs() {
	u.dt = container.NewDocTabs()
	u.dt.OnClosed = func(ti *container.TabItem) { delete(u.mtMap, ti.Text) }
	for _, m := range programs.GetPrograms().GetProgram(config.DarkAndDarker).Macros {
		u.addMacroDocTab(m)
	}
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
	u.AddMacroTree(macro.Name, &MacroTree{Macro: macro, Tree: &widget.Tree{}})
	if _, ok := u.mtMap[macro.Name]; !ok {
		return
	}
	mt := u.mtMap[macro.Name]

	mt.createTree()

	t := container.NewTabItem(macro.Name, mt.Tree)
	u.dt.Append(t)
	u.dt.Select(t)
	mt.updateTreeOnselect()
	mt.Tree.Refresh()
}
